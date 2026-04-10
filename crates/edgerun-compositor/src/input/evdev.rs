//! evdev input device manager — reuses edgerun-evdev-input crate.

use std::collections::HashMap;
use std::io;

use edgerun_evdev_input::{discover_evdev_devices, EvdevInputBackend};
use edgerun_input::{InputDevice, InputDeviceKind};

/// Manager for evdev input devices.
pub struct EvdevManager {
    devices: HashMap<u32, EvdevInputBackend>,
    next_id: u32,
    /// Map from device id to our internal id.
    device_map: HashMap<String, u32>,
}

/// A discovered input device.
#[derive(Debug, Clone)]
pub struct InputDeviceInfo {
    pub id: u32,
    pub name: String,
    pub kind: InputDeviceKind,
    pub event_node: String,
}

impl EvdevManager {
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
            next_id: 1,
            device_map: HashMap::new(),
        }
    }

    /// Discover and open all evdev devices.
    /// Only opens keyboard, pointer, touch, and switch devices.
    /// Skips Other devices (e.g. rotary encoders, gamepads) that can spam events.
    pub fn discover(&mut self) -> io::Result<Vec<InputDeviceInfo>> {
        let devs = discover_evdev_devices()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("evdev discovery failed: {e:?}")))?;

        let mut info = Vec::new();
        for dev_info in devs {
            let node = dev_info.event_node.clone();
            if self.device_map.contains_key(&node) {
                continue;
            }

            // Skip devices that aren't useful for the compositor
            let kind = &dev_info.kind;
            let name_lower = dev_info.device_name.to_lowercase();

            match kind {
                InputDeviceKind::Keyboard
                | InputDeviceKind::Pointer
                | InputDeviceKind::Touch
                | InputDeviceKind::Switch => {} // OK, open below
                _ => {
                    // Allow devices by name even if kind is "Other"
                    if !name_lower.contains("keyboard") {
                        continue;
                    }
                }
            }

            // Skip devices by name that are known non-keyboard HID consumers
            // (e.g. rotary encoder volume knobs that misreport BTN_TOUCH as "Touch")
            if name_lower.contains("hid device") || name_lower.contains("consumer control") {
                continue;
            }

            match EvdevInputBackend::open(dev_info) {
                Ok(backend) => {
                    let dev_kind = backend.info.kind.clone();
                    let dev_name = backend.info.device_name.clone();
                    let id = self.next_id;
                    self.next_id += 1;

                    self.device_map.insert(node.clone(), id);
                    self.devices.insert(id, backend);

                    info.push(InputDeviceInfo {
                        id,
                        name: dev_name,
                        kind: dev_kind,
                        event_node: node,
                    });
                }
                Err(_) => {
                    // Skip devices we can't open (permissions, etc.)
                }
            }
        }

        Ok(info)
    }

    /// Read events from a specific device.
    pub fn read_events(&mut self, id: u32, max: usize) -> Vec<edgerun_input::InputEventRecord> {
        if let Some(dev) = self.devices.get_mut(&id) {
            match dev.read_events(max) {
                Ok(events) => events,
                Err(_) => Vec::new(),
            }
        } else {
            Vec::new()
        }
    }

    /// Get all device IDs.
    pub fn device_ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.devices.keys().copied()
    }

    /// Get the file descriptor for a specific device.
    pub fn device_fd(&self, id: u32) -> Option<i32> {
        self.devices.get(&id).map(|d| d.fd())
    }

    /// Get info about a device.
    pub fn device_info(&self, id: u32) -> Option<InputDeviceInfo> {
        self.devices.get(&id).map(|d| InputDeviceInfo {
            id,
            name: d.info.device_name.clone(),
            kind: d.info.kind.clone(),
            event_node: d.info.event_node.clone(),
        })
    }
}
