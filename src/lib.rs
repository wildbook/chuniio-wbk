use std::{
    ffi::c_void,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicBool, AtomicPtr, Ordering},
    },
    thread::JoinHandle,
    time::Duration,
};

use log::{debug, error, info};
use rusb::{GlobalContext, HotplugBuilder};
use zerocopy::FromZeros;
use zerocopy_derive::FromZeros;

use crate::{device_finder::DeviceFinder, device_manager::DeviceCollection};

mod chuni_io;
mod device_finder;
mod device_manager;
mod devices;
mod logger;
mod shared_memory;

pub trait InputDevice: Send {
    fn poll(&mut self) -> anyhow::Result<()>;
    fn poll_jvs(&mut self) -> anyhow::Result<(u8, u8)>;
    fn poll_slider(&mut self) -> anyhow::Result<[u8; 32]>;
    fn set_leds(&mut self, brg: &[Rgb; 31]) -> anyhow::Result<()>;
}

pub type Rgb = [u8; 3];

const MAGIC: u32 = u32::from_le_bytes(*b"CHNI");

#[repr(C)]
#[derive(FromZeros)]
struct SharedState {
    m: u32, // magic
    i: InputState,
    o: OutputState,
}

#[repr(C)]
#[derive(FromZeros)]
struct OutputState {
    board_0_billboard: [Rgb; 10 * 5],
    board_0_air_tower: [Rgb; 3],

    board_1_billboard: [Rgb; 10 * 6],
    board_1_air_tower: [Rgb; 3],

    slider: [Rgb; 31], // 16 keys, 15 gaps, alternating
}

#[repr(C)]
#[derive(FromZeros)]
struct InputState {
    ir_sensors: u8, // ..654321 for IR beams
    fn_buttons: u8, // ......21 for FN1/FN2
    coin_count: u16,
    slider_pressure: [u8; 32],
}

/// Wrapper to make Shmem usable in static.
struct ShmemHolder(shared_memory::Shmem);
unsafe impl Send for ShmemHolder {}
unsafe impl Sync for ShmemHolder {}

static SHMEM: OnceLock<ShmemHolder> = OnceLock::new();
static STATE: AtomicPtr<SharedState> = AtomicPtr::new(std::ptr::null_mut());

/// Shutdown signal for host thread.
static SHUTDOWN: AtomicBool = AtomicBool::new(false);

fn init_child(state: *mut SharedState) {
    info!("Initializing as child (64-bit)");
    // There's nothing to do here, at least for now.

    while unsafe { (*state).m } != MAGIC {
        debug!("waiting for host to initialize...");
        std::thread::sleep(Duration::from_millis(100));
    }

    STATE.store(state, Ordering::Release);
}

fn init_host(state: *mut SharedState) {
    info!("Initializing as host (32-bit)");

    let (send, recv) = std::sync::mpsc::channel();
    let mut dm = DeviceCollection::new(recv);

    let ctx = GlobalContext {};
    let _rh = match HotplugBuilder::new()
        .enumerate(true)
        .register(ctx, Box::new(DeviceFinder(send)))
    {
        Ok(h) => h,
        Err(e) => {
            error!("Failed to register hotplug: {e}");
            return;
        }
    };

    unsafe { state.write(SharedState::new_zeroed()) };
    unsafe { (*state).m = MAGIC };

    STATE.store(state, Ordering::Release);

    while !SHUTDOWN.load(Ordering::Acquire) {
        dm.update_devices(state);
        std::thread::sleep(Duration::from_millis(1));
    }

    info!("Host thread exiting");
}

fn initialize() {
    logger::init_logger();

    let len = std::mem::size_of::<SharedState>();
    let mem = shared_memory::create("chuniio_wbk_shared", len) //
        .expect("Failed to create shared memory");

    let mem = SHMEM.get_or_init(move || ShmemHolder(mem));
    let ptr = mem.0.as_ptr().cast::<SharedState>();

    // 32-bit = host, 64-bit = child
    match std::mem::size_of::<usize>() {
        4 => init_host(ptr),
        8 => init_child(ptr),
        _ => unreachable!(),
    }
}

#[unsafe(no_mangle)]
extern "system" fn DllMain(_hinst: *mut c_void, reason: u32, _reserved: *mut c_void) -> i32 {
    const DLL_PROCESS_ATTACH: u32 = 1;
    const DLL_PROCESS_DETACH: u32 = 0;

    static THREAD: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);

    match reason {
        DLL_PROCESS_ATTACH => *THREAD.lock().unwrap() = Some(std::thread::spawn(initialize)),
        DLL_PROCESS_DETACH => {
            SHUTDOWN.store(true, Ordering::Release);

            if let Some(handle) = THREAD.lock().unwrap().take() {
                info!("Waiting for main thread to exit.");
                handle.join().unwrap();
            }
        }
        _ => {}
    }

    1
}
