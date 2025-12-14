use std::sync::mpsc::Receiver;

use tracing::{error, info};

use crate::{InputDevice, OutputState, Rgb, SharedState};

pub struct DeviceCollection {
    coin: bool,
    recv: Receiver<Box<dyn InputDevice + 'static>>,
    devices: Vec<(Box<dyn InputDevice>, u32)>, // device + consecutive error count
}

const MAX_CONSECUTIVE_ERRORS: u32 = 10;

impl DeviceCollection {
    pub fn new(recv: Receiver<Box<dyn InputDevice + 'static>>) -> Self {
        Self {
            coin: false,
            recv,
            devices: Vec::new(),
        }
    }
}

struct DeviceUpdate {
    jvs: (u8, u8),
    val: [u8; 32],
    coin: bool,
}

fn update_device(device: &mut Box<dyn InputDevice + 'static>, o: &OutputState) -> anyhow::Result<DeviceUpdate> {
    let slider: &[Rgb; 31] = &o.slider;
    let tower_l = &o.board_0_air_tower;
    let tower_r = &o.board_1_air_tower;

    device.set_leds(slider, tower_l, tower_r)?;
    device.poll()?;

    let jvs = device.poll_jvs()?;
    let val = device.poll_slider()?;
    let coin = device.poll_coin()?;

    Ok(DeviceUpdate { jvs, val, coin })
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

        let mut coin = false;

        // Update devices and track failures
        self.devices.retain_mut(|(device, error_count)| {
            match update_device(device, o) {
                Ok(new) => {
                    *error_count = 0; // Reset error count on success

                    coin |= new.coin;

                    jvs.0 |= new.jvs.0; // FN buttons
                    jvs.1 |= new.jvs.1; // IR beams

                    for i in 0..32 {
                        val[i] = std::cmp::max(val[i], new.val[i]);
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

        i.coin_count = i.coin_count.wrapping_add(u16::from(!self.coin && coin));
        i.fn_buttons = jvs.0;
        i.ir_sensors = jvs.1;
        i.slider_pressure = val;

        self.coin = coin;
    }
}
