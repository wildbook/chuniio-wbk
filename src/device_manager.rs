use std::sync::mpsc::Receiver;

use log::{error, info};

use crate::{InputDevice, Rgb, SharedState};

pub struct DeviceCollection {
    recv: Receiver<Box<dyn InputDevice + 'static>>,
    devices: Vec<(Box<dyn InputDevice>, u32)>, // device + consecutive error count
}

const MAX_CONSECUTIVE_ERRORS: u32 = 10;

impl DeviceCollection {
    pub fn new(recv: Receiver<Box<dyn InputDevice + 'static>>) -> Self {
        Self {
            recv,
            devices: Vec::new(),
        }
    }
}

fn update_device(device: &mut Box<dyn InputDevice + 'static>, led: &[Rgb; 31]) -> anyhow::Result<((u8, u8), [u8; 32])> {
    device.set_leds(led)?;
    device.poll()?;
    let jvs = device.poll_jvs()?;
    let val = device.poll_slider()?;
    Ok((jvs, val))
}

impl DeviceCollection {
    pub fn update_devices(&mut self, state: *mut SharedState) {
        // Add newly connected devices
        while let Ok(device) = self.recv.try_recv() {
            info!("New device connected");
            self.devices.push((device, 0));
        }

        let i = unsafe { &mut (*state).i };
        let o = unsafe { &(*state).o };

        let mut jvs = (0, 0);
        let mut val = [0u8; 32];

        // Update devices and track failures
        self.devices.retain_mut(|(device, error_count)| {
            match update_device(device, &o.slider) {
                Ok((new_jvs, new_val)) => {
                    *error_count = 0; // Reset error count on success

                    jvs.0 |= new_jvs.0; // FN buttons
                    jvs.1 |= new_jvs.1; // IR beams

                    for i in 0..32 {
                        val[i] = std::cmp::max(val[i], new_val[i]);
                    }

                    true // Keep device
                }

                Err(e) => {
                    *error_count += 1;

                    if *error_count >= MAX_CONSECUTIVE_ERRORS {
                        error!("Device failed {MAX_CONSECUTIVE_ERRORS} times, removing: {e}");
                        false // Remove device
                    } else {
                        error!("Failed to update device (error {error_count}/{MAX_CONSECUTIVE_ERRORS}): {e}");
                        true // Keep device for now
                    }
                }
            }
        });

        i.fn_buttons = jvs.0;
        i.ir_sensors = jvs.1;
        i.slider_pressure = val;
    }
}
