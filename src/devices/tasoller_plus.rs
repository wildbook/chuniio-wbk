use std::time::Duration;

use rusb::{DeviceHandle, UsbContext};
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
    slider_led: [Rgb; 31],
    left_air_tower_led: [u8; 9],
    right_air_tower_led: [u8; 9],
}

pub struct TasollerPlus<T: UsbContext> {
    dev: DeviceHandle<T>,
    tpi: TasollerPlusInput,
}

impl<T: UsbContext> TasollerPlus<T> {
    pub fn from_device(dev: DeviceHandle<T>) -> anyhow::Result<Self> {
        dev.set_active_configuration(1)?;
        dev.claim_interface(DEVICE_INTERFACE)?;

        Ok(TasollerPlus {
            dev,
            tpi: TasollerPlusInput::new_zeroed(),
        })
    }
}

impl<T: UsbContext> InputDevice for TasollerPlus<T> {
    fn poll(&mut self) -> anyhow::Result<()> {
        let tpi = self.tpi.as_mut_bytes();
        self.dev.read_interrupt(R_ENDPOINT, tpi, R_TIMEOUT)?;

        Ok(())
    }

    fn poll_jvs(&mut self) -> anyhow::Result<(u8, u8)> {
        let bits = self.tpi.ir_buttons.reverse_bits();

        let ir_bits = (bits & 0b1111_1100) >> 2;
        let fn_bits = (bits & 0b0000_0011) >> 0;

        Ok((fn_bits, ir_bits))
    }

    fn poll_slider(&mut self) -> anyhow::Result<[u8; 32]> {
        Ok(self.tpi.slider_pressure)
    }

    fn set_leds(&mut self, brg: &[Rgb; 31]) -> anyhow::Result<()> {
        let mut output = TasollerPlusOutput::new_zeroed();
        output.magic = [0x44, 0x4C];
        output.protocol_version = 0x02;

        let mut pxl = *brg;
        for px in pxl.iter_mut() {
            let b = px[0];
            let r = px[1];
            let g = px[2];
            *px = [r, g, b];
        }

        output.slider_led.copy_from_slice(&pxl);

        self.dev.write_bulk(W_ENDPOINT, output.as_bytes(), W_TIMEOUT)?;
        Ok(())
    }
}
