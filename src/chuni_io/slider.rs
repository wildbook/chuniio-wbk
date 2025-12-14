use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use log::{info, warn};

use crate::{Rgb, STATE, chuni_io::HRESULT};

static SLIDER_ACTIVE: AtomicBool = AtomicBool::new(false);

type SliderCallbackFn = extern "C" fn(data: *const [u8; 32]);

#[unsafe(no_mangle)]
pub extern "C" fn chuni_io_slider_init() -> HRESULT {
    info!("chuni_io_slider_init");
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn chuni_io_slider_start(callback: Option<SliderCallbackFn>) {
    info!("chuni_io_slider_start");

    let Some(callback) = callback else {
        warn!("chuni_io_slider_start: no callback");
        return;
    };

    // Already running? Just return.
    if SLIDER_ACTIVE.swap(true, Ordering::AcqRel) {
        return;
    }

    std::thread::spawn(move || {
        while SLIDER_ACTIVE.load(Ordering::Acquire) {
            let ptr = STATE.load(Ordering::Acquire);
            if ptr.is_null() {
                std::thread::sleep(Duration::from_millis(10));
                continue;
            }

            unsafe { callback(&raw const (*ptr).i.slider_pressure) };
            std::thread::sleep(Duration::from_millis(1));
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn chuni_io_slider_stop() {
    info!("chuni_io_slider_stop");
    SLIDER_ACTIVE.store(false, Ordering::Release);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn chuni_io_slider_set_leds(rgb: *const [Rgb; 31]) {
    let Some(state) = (unsafe { STATE.load(Ordering::Acquire).as_mut() }) else {
        return;
    };

    state.o.slider.copy_from_slice(unsafe { &*rgb });
}
