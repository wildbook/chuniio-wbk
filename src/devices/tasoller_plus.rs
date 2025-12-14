use std::time::Duration;

use rusb::{DeviceHandle, UsbContext};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_F14};
use zerocopy::{FromZeros, IntoBytes};
use zerocopy_derive::{FromBytes, Immutable, IntoBytes};

use crate::{InputDevice, Rgb};

const DEVICE_INTERFACE: u8 = 0;

const R_ENDPOINT: u8 = 0x84;
const W_ENDPOINT: u8 = 0x03;

const R_TIMEOUT: Duration = Duration::from_millis(2);
const W_TIMEOUT: Duration = Duration::from_millis(2);

#[repr(C)]
#[derive(IntoBytes, FromBytes, Immutable)]
struct TasollerPlusInput {
    magic: [u8; 3],
    ir_buttons: u8, // 6 bits for IR beams, 2 bits for FN1/FN2
    slider_pressure: [u8; 32],
}

#[repr(C)]
#[derive(IntoBytes, FromBytes, Immutable)]
struct TasollerPlusOutput {
    magic: [u8; 2],
    protocol_version: u8,
    led_slider: [Rgb; 31],
    led_tower_l: [Rgb; 3],
    led_tower_r: [Rgb; 3],
}

pub struct TasollerPlus<T: UsbContext> {
    dev: DeviceHandle<T>,
    tpi: TasollerPlusInput,
    f14: bool,
}

impl<T: UsbContext> TasollerPlus<T> {
    pub fn from_device(dev: DeviceHandle<T>) -> anyhow::Result<Self> {
        dev.set_active_configuration(1)?;
        dev.claim_interface(DEVICE_INTERFACE)?;

        let tpi = TasollerPlusInput::new_zeroed();
        let f14 = false;

        Ok(TasollerPlus { dev, tpi, f14 })
    }
}

impl<T: UsbContext> InputDevice for TasollerPlus<T> {
    fn poll(&mut self) -> anyhow::Result<()> {
        let tpi = self.tpi.as_mut_bytes();
        self.dev.read_interrupt(R_ENDPOINT, tpi, R_TIMEOUT)?;

        self.f14 = unsafe { GetKeyState(VK_F14 as i32) < 0 };

        Ok(())
    }

    fn poll_jvs(&mut self) -> anyhow::Result<(u8, u8)> {
        let bits = self.tpi.ir_buttons.reverse_bits();

        let ir_bits = (bits & 0b1111_1100) >> 2;
        let fn_bits = (bits & 0b0000_0011) >> 0;

        Ok((fn_bits, ir_bits))
    }

    fn poll_coin(&mut self) -> anyhow::Result<bool> {
        Ok(self.f14)
    }

    fn poll_slider(&mut self) -> anyhow::Result<[u8; 32]> {
        Ok(self.tpi.slider_pressure)
    }

    fn set_leds(&mut self, slider: &[Rgb; 31], tower_l: &[Rgb; 3], tower_r: &[Rgb; 3]) -> anyhow::Result<()> {
        let mut output = TasollerPlusOutput::new_zeroed();
        output.magic = [0x44, 0x4C];
        output.protocol_version = 0x02;

        let mut rgb = *slider;
        for v in rgb.iter_mut() {
            let [b, r, g] = *v;
            *v = [r, g, b];
        }

        output.led_slider = rgb;
        output.led_tower_l = *tower_l;
        output.led_tower_r = *tower_r;

        self.dev.write_bulk(W_ENDPOINT, output.as_bytes(), W_TIMEOUT)?;
        Ok(())
    }
}
