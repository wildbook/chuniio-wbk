use std::sync::atomic::Ordering;

use log::{info, warn};

use crate::{Rgb, STATE, chuni_io::HRESULT};

#[unsafe(no_mangle)]
pub extern "C" fn chuni_io_led_init() -> HRESULT {
    info!("chuni_io_led_init");
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn chuni_io_led_set_colors(board: u8, rgb: *const u8) {
    let Some(state) = (unsafe { STATE.load(Ordering::Acquire).as_mut() }) else {
        return;
    };

    match board {
        0 => {
            let colors: &[Rgb; 53] = unsafe { &*(rgb as *const [Rgb; 53]) };
            state.o.board_0_billboard.copy_from_slice(&colors[0..50]);
            state.o.board_0_air_tower.copy_from_slice(&colors[50..53]);
        }
        1 => {
            let colors: &[Rgb; 63] = unsafe { &*(rgb as *const [Rgb; 63]) };
            state.o.board_1_billboard.copy_from_slice(&colors[0..60]);
            state.o.board_1_air_tower.copy_from_slice(&colors[60..63]);
        }
        _ => warn!("Invalid board: {board}"),
    }
}
