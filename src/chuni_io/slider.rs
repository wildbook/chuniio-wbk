use std::{
    sync::atomic::{AtomicBool, AtomicPtr, Ordering},
    thread::JoinHandle,
    time::Duration,
};

use parking_lot::Mutex;
use tracing::debug;

use crate::{Rgb, STATE, chuni_io::HRESULT};

static SLIDER_THREAD: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);
static SLIDER_ACTIVE: AtomicBool = AtomicBool::new(false);

type SliderCallbackFn = extern "C" fn(data: *const [u8; 32]);

extern "C" fn dummy_callback(_data: *const [u8; 32]) {}

#[unsafe(no_mangle)]
pub extern "C" fn chuni_io_slider_init() -> HRESULT {
    debug!("chuni_io_slider_init");
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn chuni_io_slider_start(callback: Option<SliderCallbackFn>) {
    static CALLBACK_FN: AtomicPtr<()> /* SliderCallbackFn */ = AtomicPtr::new(dummy_callback as _);
    debug!("chuni_io_slider_start: {callback:?}");

    // Set the active callback, or a dummy if none pr
    let callback = callback.unwrap_or(dummy_callback);
    CALLBACK_FN.store(callback as _, Ordering::Relaxed);

    // Lock before checking SLIDER_ACTIVE to avoid racing with stop.
    let mut mtx = SLIDER_THREAD.lock();

    // Already running? Just return.
    if SLIDER_ACTIVE.swap(true, Ordering::AcqRel) {
        return;
    }

    let thread_fn = move || {
        while SLIDER_ACTIVE.load(Ordering::Acquire) {
            let Some(ptr) = (unsafe { STATE.load(Ordering::Acquire).as_ref() }) else {
                std::thread::sleep(Duration::from_millis(10));
                continue;
            };

            let callback = CALLBACK_FN.load(Ordering::Relaxed);
            let callback = unsafe { std::mem::transmute::<_, SliderCallbackFn>(callback) };

            callback(&ptr.i.slider_pressure);
            std::thread::sleep(Duration::from_millis(1));
        }

        debug!("Slider thread exiting");
    };

    *mtx = Some(std::thread::spawn(thread_fn));
}

#[unsafe(no_mangle)]
pub extern "C" fn chuni_io_slider_stop() {
    debug!("chuni_io_slider_stop");

    // Lock before checking SLIDER_ACTIVE to avoid racing with start.
    let mut mtx = SLIDER_THREAD.lock();

    // Setting active to false will signal the thread to exit.
    SLIDER_ACTIVE.store(false, Ordering::Release);

    // Join the thread if it exists, then remove it from the mutex.
    if let Some(handle) = mtx.take() {
        handle.join().expect("Failed to join slider thread");
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn chuni_io_slider_set_leds(rgb: *const [Rgb; 31]) {
    let Some(state) = (unsafe { STATE.load(Ordering::Acquire).as_mut() }) else {
        return;
    };

    state.o.slider.copy_from_slice(unsafe { &*rgb });
}
