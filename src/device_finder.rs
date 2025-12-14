use std::sync::mpsc::Sender;

use anyhow::bail;
use rusb::{Hotplug, UsbContext};
use tracing::{error, info, trace};

use crate::{InputDevice, devices::TasollerPlus};

pub struct DeviceFinder(pub Sender<Box<dyn InputDevice + 'static>>);

impl DeviceFinder {
    pub fn try_handle<T: UsbContext + 'static>(&mut self, device: rusb::Device<T>) -> anyhow::Result<()> {
        let Ok(desc) = device.device_descriptor() else {
            bail!("Failed to get device descriptor");
        };

        let device = match (desc.vendor_id(), desc.product_id()) {
            (0x0E8F, 0x1231) => Some(TasollerPlus::from_device(device.open()?)),
            _ => None,
        };

        let Some(device) = device else {
            return Ok(());
        };

        info!("Recognized device!");

        if let Err(e) = self.0.send(Box::new(device?)) {
            error!("Failed to send device: {e:?}");
        }

        Ok(())
    }
}

impl<T: UsbContext + 'static> Hotplug<T> for DeviceFinder {
    fn device_arrived(&mut self, device: rusb::Device<T>) {
        trace!("Device arrived: {device:?}");
        if let Err(e) = self.try_handle(device) {
            error!("Failed to handle device: {e:?}");
        }
    }

    fn device_left(&mut self, device: rusb::Device<T>) {
        info!("Device disconnected: {device:?}");
    }
}
