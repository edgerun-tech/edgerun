//! xdg-shell window management.

use std::collections::HashMap;

/// A toplevel window.
#[derive(Debug)]
pub struct Toplevel {
    /// xdg_toplevel object id.
    pub id: u32,
    /// Associated wl_surface id.
    pub surface_id: u32,
    /// Application ID (e.g., "firefox").
    pub app_id: Option<String>,
    /// Window title.
    pub title: Option<String>,
    /// Parent toplevel (for dialogs).
    pub parent: Option<u32>,
    /// Min size.
    pub min_width: i32,
    pub min_height: i32,
    /// Max size.
    pub max_width: i32,
    pub max_height: i32,
    /// Current state (byte array of u32 state flags, little-endian).
    pub states: Vec<u8>,
    /// Whether the surface has ack'd the latest configure.
    pub configured: bool,
    /// Pending configure serial.
    pub configure_serial: Option<u32>,
    /// Whether the window wants to be closed.
    pub wants_close: bool,
    /// Whether the window is maximized.
    pub maximized: bool,
    /// Whether the window is fullscreen.
    pub fullscreen: bool,
    /// Whether the window is being resized by the compositor.
    pub resizing: bool,
    /// xdg_decoration object id (if any).
    pub decoration_id: Option<u32>,
    /// Preferred decoration mode: 1 = CSD, 2 = SSD.
    pub decoration_mode: Option<u32>,
    /// Sub-surfaces belonging to this toplevel.
    pub subsurfaces: Vec<u32>, // surface ids
}

/// A popup window.
#[derive(Debug)]
pub struct Popup {
    pub id: u32,
    pub surface_id: u32,
    pub parent: Option<u32>,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// A subsurface.
#[derive(Debug)]
pub struct Subsurface {
    pub surface_id: u32,
    pub parent_surface_id: u32,
    pub x: i32,
    pub y: i32,
    /// Sync mode: commit is deferred until parent commits.
    pub sync: bool,
    /// Z-order relative to parent: siblings ordered before/after this one.
    pub stack_index: usize,
}

/// Subsurface manager.
#[derive(Debug, Default)]
pub struct SubsurfaceManager {
    pub subsurfaces: HashMap<u32, Subsurface>, // surface_id -> Subsurface
}

impl SubsurfaceManager {
    pub fn create(&mut self, surface_id: u32, parent_surface_id: u32) {
        self.subsurfaces.insert(surface_id, Subsurface {
            surface_id,
            parent_surface_id,
            x: 0,
            y: 0,
            sync: true, // default to sync mode
            stack_index: 0,
        });
    }

    pub fn destroy(&mut self, surface_id: u32) {
        self.subsurfaces.remove(&surface_id);
    }

    pub fn get(&self, surface_id: u32) -> Option<&Subsurface> {
        self.subsurfaces.get(&surface_id)
    }

    pub fn get_mut(&mut self, surface_id: u32) -> Option<&mut Subsurface> {
        self.subsurfaces.get_mut(&surface_id)
    }

    /// Get all subsurfaces for a given parent surface.
    pub fn for_parent(&self, parent_surface_id: u32) -> Vec<&Subsurface> {
        self.subsurfaces.values()
            .filter(|s| s.parent_surface_id == parent_surface_id)
            .collect()
    }
}

/// Shell state — manages xdg_wm_base and all toplevels/popups.
pub struct Shell {
    /// xdg_wm_base object id.
    pub base_id: u32,
    /// Ping serial.
    ping_serial: Option<u32>,
    /// All toplevel windows.
    pub toplevels: HashMap<u32, Toplevel>,
    /// All popups.
    pub popups: HashMap<u32, Popup>,
    /// Configure serial counter.
    configure_serial: u32,
    /// Stack of toplevels (z-order, front to back).
    pub stack: Vec<u32>,
    /// Subsurface manager.
    pub subsurfaces: SubsurfaceManager,
    /// Output dimensions (for configure).
    pub output_width: i32,
    pub output_height: i32,
}

impl Shell {
    pub fn new(base_id: u32) -> Self {
        Self {
            base_id,
            ping_serial: None,
            toplevels: HashMap::new(),
            popups: HashMap::new(),
            configure_serial: 1,
            stack: Vec::new(),
            subsurfaces: SubsurfaceManager::default(),
            output_width: 0,
            output_height: 0,
        }
    }

    /// Set output dimensions.
    pub fn set_output_size(&mut self, width: i32, height: i32) {
        self.output_width = width;
        self.output_height = height;
    }

    /// Create a toplevel for a surface.
    pub fn create_toplevel(&mut self, id: u32, surface_id: u32) {
        self.toplevels.insert(id, Toplevel {
            id,
            surface_id,
            app_id: None,
            title: None,
            parent: None,
            min_width: 0,
            min_height: 0,
            max_width: 0,
            max_height: 0,
            states: protocol::xdg_shell::toplevel_state::ACTIVATED.to_le_bytes().to_vec(),
            configured: false,
            configure_serial: None,
            wants_close: false,
            maximized: false,
            fullscreen: false,
            resizing: false,
            decoration_id: None,
            decoration_mode: None,
            subsurfaces: Vec::new(),
        });
        self.stack.push(id);
    }

    /// Get a toplevel.
    pub fn get_toplevel(&self, id: u32) -> Option<&Toplevel> {
        self.toplevels.get(&id)
    }

    /// Get a mutable toplevel.
    pub fn get_toplevel_mut(&mut self, id: u32) -> Option<&mut Toplevel> {
        self.toplevels.get_mut(&id)
    }

    /// Create a popup.
    pub fn create_popup(&mut self, id: u32, surface_id: u32, parent: Option<u32>) {
        self.popups.insert(id, Popup {
            id,
            surface_id,
            parent,
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        });
    }

    /// Send configure to a toplevel. Returns the configure serial.
    pub fn configure_toplevel(
        &mut self,
        toplevel_id: u32,
        width: i32,
        height: i32,
    ) -> u32 {
        let serial = self.configure_serial;
        self.configure_serial += 1;

        if let Some(tl) = self.toplevels.get_mut(&toplevel_id) {
            tl.configured = false;
            tl.configure_serial = Some(serial);
        }

        serial
    }

    /// Acknowledge a configure from a client.
    pub fn ack_configure(&mut self, toplevel_id: u32, serial: u32) {
        if let Some(tl) = self.toplevels.get_mut(&toplevel_id) {
            if tl.configure_serial == Some(serial) {
                tl.configured = true;
            }
        }
    }

    /// Destroy a toplevel.
    pub fn destroy_toplevel(&mut self, id: u32) {
        self.toplevels.remove(&id);
        self.stack.retain(|&x| x != id);
    }

    /// Destroy a popup.
    pub fn destroy_popup(&mut self, id: u32) {
        self.popups.remove(&id);
    }

    /// Get toplevel at the top of the stack (frontmost).
    pub fn frontmost(&self) -> Option<&Toplevel> {
        self.stack.last().and_then(|&id| self.toplevels.get(&id))
    }

    /// Get all toplevels in z-order (front to back).
    pub fn toplevels_z_order(&self) -> impl Iterator<Item = &Toplevel> {
        self.stack.iter().rev().filter_map(|&id| self.toplevels.get(&id))
    }

    /// Activate a toplevel (bring to front).
    pub fn activate(&mut self, id: u32) {
        self.stack.retain(|&x| x != id);
        self.stack.push(id);
    }

    /// Find the toplevel that owns a surface.
    pub fn toplevel_for_surface(&self, surface_id: u32) -> Option<&Toplevel> {
        self.toplevels.values().find(|tl| tl.surface_id == surface_id)
    }

    /// Send a ping to check if a client is alive.
    pub fn ping(&mut self) -> u32 {
        let serial = self.configure_serial;
        self.configure_serial += 1;
        self.ping_serial = Some(serial);
        serial
    }

    /// Send configure with current state to a toplevel.
    pub fn configure_toplevel_with_state(
        &mut self,
        toplevel_id: u32,
        width: i32,
        height: i32,
    ) -> u32 {
        let serial = self.configure_serial;
        self.configure_serial += 1;

        if let Some(tl) = self.toplevels.get_mut(&toplevel_id) {
            tl.configured = false;
            tl.configure_serial = Some(serial);
            // Build state array (each state is a u32)
            tl.states.clear();
            if tl.fullscreen {
                tl.states.extend_from_slice(&protocol::xdg_shell::toplevel_state::FULLSCREEN.to_le_bytes());
            }
            if tl.maximized {
                tl.states.extend_from_slice(&protocol::xdg_shell::toplevel_state::MAXIMIZED.to_le_bytes());
            }
            if tl.resizing {
                tl.states.extend_from_slice(&protocol::xdg_shell::toplevel_state::RESIZING.to_le_bytes());
            }
            if !tl.fullscreen && !tl.maximized {
                tl.states.extend_from_slice(&protocol::xdg_shell::toplevel_state::ACTIVATED.to_le_bytes());
            }
        }

        serial
    }

    /// Maximize a toplevel.
    pub fn maximize(&mut self, id: u32) {
        if let Some(tl) = self.toplevels.get_mut(&id) {
            tl.maximized = true;
        }
    }

    /// Unmaximize a toplevel.
    pub fn unmaximize(&mut self, id: u32) {
        if let Some(tl) = self.toplevels.get_mut(&id) {
            tl.maximized = false;
        }
    }

    /// Set fullscreen on a toplevel.
    pub fn set_fullscreen(&mut self, id: u32, fullscreen: bool) {
        if let Some(tl) = self.toplevels.get_mut(&id) {
            tl.fullscreen = fullscreen;
        }
    }

    /// Get the current state bytes for a toplevel.
    pub fn toplevel_state_bytes(&self, toplevel_id: u32) -> Vec<u8> {
        if let Some(tl) = self.toplevels.get(&toplevel_id) {
            tl.states.clone()
        } else {
            protocol::xdg_shell::toplevel_state::ACTIVATED.to_le_bytes().to_vec()
        }
    }
}

use crate::protocol;
