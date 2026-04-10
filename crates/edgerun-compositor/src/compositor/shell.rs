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
    /// Positioner state (from xdg_positioner).
    pub anchor_rect_x: i32,
    pub anchor_rect_y: i32,
    pub anchor_rect_width: i32,
    pub anchor_rect_height: i32,
    pub anchor: u32,
    pub gravity: u32,
    pub offset_x: i32,
    pub offset_y: i32,
    /// Whether this popup has an active grab.
    pub grabbed: bool,
    /// The seat that owns the grab (if grabbed).
    pub grab_seat_id: Option<u32>,
    /// Serial at grab time.
    pub grab_serial: Option<u32>,
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
    /// Output refresh rate in mHz.
    pub output_refresh_mhz: i32,
    /// Physical dimensions in mm.
    pub output_mm_width: i32,
    pub output_mm_height: i32,
    /// Positioner state (accumulated before GET_POPUP is called).
    pub positioners: std::collections::HashMap<u32, PositionerState>,
}

/// Positioner state from xdg_positioner protocol.
#[derive(Debug, Default)]
pub struct PositionerState {
    pub width: i32,
    pub height: i32,
    pub anchor_rect_x: i32,
    pub anchor_rect_y: i32,
    pub anchor_rect_width: i32,
    pub anchor_rect_height: i32,
    pub anchor: u32,
    pub gravity: u32,
    pub offset_x: i32,
    pub offset_y: i32,
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
            output_refresh_mhz: 60000,
            output_mm_width: 0,
            output_mm_height: 0,
            positioners: std::collections::HashMap::new(),
        }
    }

    /// Set output dimensions and physical properties.
    pub fn set_output_size(&mut self, width: i32, height: i32, refresh_mhz: i32, mm_width: i32, mm_height: i32) {
        self.output_width = width;
        self.output_height = height;
        self.output_refresh_mhz = refresh_mhz;
        self.output_mm_width = mm_width;
        self.output_mm_height = mm_height;
    }

    /// Create or update positioner state.
    pub fn set_positioner(&mut self, positioner_id: u32, state: PositionerState) {
        self.positioners.insert(positioner_id, state);
    }

    /// Destroy positioner state.
    pub fn remove_positioner(&mut self, positioner_id: u32) {
        self.positioners.remove(&positioner_id);
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
            anchor_rect_x: 0,
            anchor_rect_y: 0,
            anchor_rect_width: 0,
            anchor_rect_height: 0,
            anchor: 0,
            gravity: 0,
            offset_x: 0,
            offset_y: 0,
            grabbed: false,
            grab_seat_id: None,
            grab_serial: None,
        });
    }

    /// Update popup positioner state.
    pub fn set_popup_positioner(
        &mut self,
        popup_id: u32,
        anchor_rect_x: i32,
        anchor_rect_y: i32,
        anchor_rect_width: i32,
        anchor_rect_height: i32,
        anchor: u32,
        gravity: u32,
        offset_x: i32,
        offset_y: i32,
    ) {
        if let Some(popup) = self.popups.get_mut(&popup_id) {
            popup.anchor_rect_x = anchor_rect_x;
            popup.anchor_rect_y = anchor_rect_y;
            popup.anchor_rect_width = anchor_rect_width;
            popup.anchor_rect_height = anchor_rect_height;
            popup.anchor = anchor;
            popup.gravity = gravity;
            popup.offset_x = offset_x;
            popup.offset_y = offset_y;

            // Calculate popup position from positioner state
            // Simple implementation: position relative to parent's anchor rect + offset
            popup.x = anchor_rect_x + offset_x;
            popup.y = anchor_rect_y + offset_y;
        }
    }

    /// Start a grab on a popup.
    pub fn grab_popup(&mut self, popup_id: u32, seat_id: u32, serial: u32) {
        if let Some(popup) = self.popups.get_mut(&popup_id) {
            popup.grabbed = true;
            popup.grab_seat_id = Some(seat_id);
            popup.grab_serial = Some(serial);
        }
    }

    /// End a grab on a popup.
    pub fn ungrab_popup(&mut self, popup_id: u32) {
        if let Some(popup) = self.popups.get_mut(&popup_id) {
            popup.grabbed = false;
            popup.grab_seat_id = None;
            popup.grab_serial = None;
        }
    }

    /// Dismiss all grabs (e.g., on pointer button release).
    pub fn dismiss_all_grabs(&mut self) {
        for (_, popup) in self.popups.iter_mut() {
            popup.grabbed = false;
            popup.grab_seat_id = None;
            popup.grab_serial = None;
        }
    }

    /// Send configure to a toplevel. Returns the configure serial.
    pub fn configure_toplevel(
        &mut self,
        toplevel_id: u32,
        _width: i32,
        _height: i32,
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
        _width: i32,
        _height: i32,
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
    pub fn toplevel_state_bytes(&self, toplevel_id: u32) -> &[u8] {
        static DEFAULT_STATE: [u8; 4] = protocol::xdg_shell::toplevel_state::ACTIVATED.to_le_bytes();
        self.toplevels.get(&toplevel_id)
            .map(|tl| tl.states.as_slice())
            .unwrap_or(&DEFAULT_STATE)
    }
}

use crate::protocol;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_toplevel() {
        let mut shell = Shell::new(100);
        shell.create_toplevel(1, 5);
        let tl = shell.get_toplevel(1).unwrap();
        assert_eq!(tl.id, 1);
        assert_eq!(tl.surface_id, 5);
        assert!(shell.stack.contains(&1));
    }

    #[test]
    fn test_destroy_toplevel() {
        let mut shell = Shell::new(100);
        shell.create_toplevel(1, 5);
        shell.destroy_toplevel(1);
        assert!(shell.get_toplevel(1).is_none());
        assert!(!shell.stack.contains(&1));
    }

    #[test]
    fn test_toplevel_z_order() {
        let mut shell = Shell::new(100);
        shell.create_toplevel(1, 5);
        shell.create_toplevel(2, 6);
        shell.create_toplevel(3, 7);

        // Front to back: last created is frontmost
        let z_order: Vec<u32> = shell.toplevels_z_order().map(|tl| tl.id).collect();
        assert_eq!(z_order, vec![3, 2, 1]);
    }

    #[test]
    fn test_activate_moves_to_front() {
        let mut shell = Shell::new(100);
        shell.create_toplevel(1, 5);
        shell.create_toplevel(2, 6);
        shell.create_toplevel(3, 7);

        shell.activate(1); // move 1 to front
        let z_order: Vec<u32> = shell.toplevels_z_order().map(|tl| tl.id).collect();
        assert_eq!(z_order, vec![1, 3, 2]);
    }

    #[test]
    fn test_configure_toplevel() {
        let mut shell = Shell::new(100);
        shell.create_toplevel(1, 5);
        let serial = shell.configure_toplevel(1, 800, 600);
        assert!(serial > 0);
        // After configure, it's not configured until ack
        assert!(!shell.get_toplevel(1).unwrap().configured);
    }

    #[test]
    fn test_ack_configure() {
        let mut shell = Shell::new(100);
        shell.create_toplevel(1, 5);
        let serial = shell.configure_toplevel(1, 800, 600);
        shell.ack_configure(1, serial);
        assert!(shell.get_toplevel(1).unwrap().configured);
    }

    #[test]
    fn test_toplevel_state_bytes() {
        let mut shell = Shell::new(100);
        shell.create_toplevel(1, 5);
        let state = shell.toplevel_state_bytes(1);
        // Should be ACTIVATED state (4 bytes)
        assert_eq!(state.len(), 4);
        let state_val = u32::from_le_bytes(state.try_into().unwrap());
        assert_eq!(state_val, protocol::xdg_shell::toplevel_state::ACTIVATED);
    }

    #[test]
    fn test_subsurface_manager() {
        let mut shell = Shell::new(100);
        shell.subsurfaces.create(10, 5);
        shell.subsurfaces.create(11, 5);
        shell.subsurfaces.create(12, 6);

        let mut parent_5: Vec<_> = shell.subsurfaces.for_parent(5).iter().map(|s| s.surface_id).collect();
        parent_5.sort();
        assert_eq!(parent_5, vec![10, 11]);

        let mut parent_6: Vec<_> = shell.subsurfaces.for_parent(6).iter().map(|s| s.surface_id).collect();
        parent_6.sort();
        assert_eq!(parent_6, vec![12]);
    }

    #[test]
    fn test_popup_grab() {
        let mut shell = Shell::new(100);
        shell.create_popup(1, 5, Some(100));
        assert!(!shell.popups.get(&1).unwrap().grabbed);

        shell.grab_popup(1, 1, 42);
        assert!(shell.popups.get(&1).unwrap().grabbed);
        assert_eq!(shell.popups.get(&1).unwrap().grab_serial, Some(42));

        shell.ungrab_popup(1);
        assert!(!shell.popups.get(&1).unwrap().grabbed);
    }
}
