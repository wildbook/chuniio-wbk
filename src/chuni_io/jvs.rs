use std::sync::atomic::Ordering;

use log::info;

use crate::{STATE, chuni_io::HRESULT};

#[unsafe(no_mangle)]
pub extern "C" fn chuni_io_jvs_init() -> HRESULT {
    info!("chuni_io_jvs_init");
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn chuni_io_jvs_poll(opbtn: *mut u8, beams: *mut u8) {
    let Some(state) = (unsafe { STATE.load(Ordering::Acquire).as_ref() }) else {
        return;
    };

    unsafe { opbtn.write(state.i.fn_buttons) };
    unsafe { beams.write(state.i.ir_sensors) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn chuni_io_jvs_read_coin_counter(total: *mut u16) {
    let Some(state) = (unsafe { STATE.load(Ordering::Acquire).as_ref() }) else {
        return;
    };

    unsafe { total.write(state.i.coin_count) };
}
