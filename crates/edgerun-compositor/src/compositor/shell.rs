//! xdg-shell window management.

use std::collections::{HashMap, HashSet};

use crate::protocol::dispatch::linux_ext::SyncobjState;

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
    /// Window geometry (from SET_WINDOW_GEOMETRY).
    pub window_x: i32,
    pub window_y: i32,
    pub window_width: i32,
    pub window_height: i32,
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
    /// Whether the window is minimized.
    pub minimized: bool,
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

/// A layer surface (zwlr_layer_surface_v1).
#[derive(Debug)]
pub struct LayerSurface {
    /// zwlr_layer_surface_v1 object id.
    pub id: u32,
    /// Associated wl_surface id.
    pub surface_id: u32,
    /// Client that created this surface.
    pub client_id: u32,
    /// Layer: 0=background, 1=bottom, 2=top, 3=overlay.
    pub layer: u32,
    /// Anchor bitmask: top=1, bottom=2, left=4, right=8.
    pub anchor: u32,
    /// Exclusive zone. -1 = auto (use size), >= 0 = explicit zone.
    pub exclusive_zone: i32,
    /// Margins: top, right, bottom, left.
    pub margin_top: i32,
    pub margin_right: i32,
    pub margin_bottom: i32,
    pub margin_left: i32,
    /// Keyboard interactivity: 0=none, 1=exclusive, 2=on_demand.
    pub keyboard_interactivity: u32,
    /// Desired size from client (0, 0) means stretch to fill available space.
    pub desired_width: u32,
    pub desired_height: u32,
    /// Computed position and size after layout.
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    /// Configure state.
    pub configured: bool,
    pub configure_serial: Option<u32>,
    /// Whether the surface has been closed.
    pub closed: bool,
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
    /// Output scale factor (integer, e.g., 1, 2).
    pub output_scale: i32,
    /// Positioner state (accumulated before GET_POPUP is called).
    pub positioners: std::collections::HashMap<u32, PositionerState>,

    // Idle inhibit state
    /// Surface IDs that are currently inhibiting idle/sleep.
    pub idle_inhibitors: HashSet<u32>,
    /// Map from inhibitor object ID to surface ID.
    pub inhibitor_to_surface: HashMap<u32, u32>,

    // Activation token state
    /// Registered activation tokens and their originating client.
    pub activation_tokens: HashMap<String, u32>,
    /// Pending token metadata keyed by token object ID.
    pub pending_token_serial: HashMap<u32, u64>,
    pub pending_token_app_id: HashMap<u32, String>,
    pub pending_token_surface: HashMap<u32, u32>,

    // DRM syncobj state
    pub syncobj_state: SyncobjState,
    /// Map from syncobj_surface object ID to wl_surface ID.
    pub syncobj_surface_map: HashMap<u32, u32>,

    // Layer shell state
    /// All layer surfaces keyed by layer_surface object id.
    pub layer_surfaces: HashMap<u32, LayerSurface>,
    /// Map from zwlr_layer_surface_v1 object id to associated xdg_popup object id
    /// (for popups anchored to layer surfaces).
    pub layer_popup_map: HashMap<u32, u32>,
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
    // v6+ fields
    pub constraint_adjustment: u32,
    pub reactive: bool,
    pub parent_width: i32,
    pub parent_height: i32,
    pub parent_configure_serial: u32,
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
            output_scale: 1,
            positioners: std::collections::HashMap::new(),
            idle_inhibitors: HashSet::new(),
            inhibitor_to_surface: HashMap::new(),
            activation_tokens: HashMap::new(),
            pending_token_serial: HashMap::new(),
            pending_token_app_id: HashMap::new(),
            pending_token_surface: HashMap::new(),
            syncobj_state: SyncobjState::new(),
            syncobj_surface_map: HashMap::new(),
            layer_surfaces: HashMap::new(),
            layer_popup_map: HashMap::new(),
        }
    }

    /// Check if any surface is inhibiting idle.
    pub fn idle_inhibited(&self) -> bool {
        !self.idle_inhibitors.is_empty()
    }

    /// Check if a specific surface is inhibiting idle.
    pub fn surface_inhibits_idle(&self, surface_id: u32) -> bool {
        self.idle_inhibitors.contains(&surface_id)
    }

    /// Add an idle inhibitor for a surface.
    pub fn add_idle_inhibitor(&mut self, inhibitor_id: u32, surface_id: u32) {
        self.idle_inhibitors.insert(surface_id);
        self.inhibitor_to_surface.insert(inhibitor_id, surface_id);
    }

    /// Remove an idle inhibitor for a surface.
    pub fn remove_idle_inhibitor(&mut self, inhibitor_id: u32) {
        if let Some(surface_id) = self.inhibitor_to_surface.remove(&inhibitor_id) {
            self.idle_inhibitors.remove(&surface_id);
        }
    }

    /// Set output dimensions and physical properties.
    pub fn set_output_size(&mut self, width: i32, height: i32, refresh_mhz: i32, mm_width: i32, mm_height: i32, scale: i32) {
        self.output_width = width;
        self.output_height = height;
        self.output_refresh_mhz = refresh_mhz;
        self.output_mm_width = mm_width;
        self.output_mm_height = mm_height;
        self.output_scale = scale.max(1);
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
            window_x: 0,
            window_y: 0,
            window_width: 0,
            window_height: 0,
            states: protocol::xdg_shell::toplevel_state::ACTIVATED.to_le_bytes().to_vec(),
            configured: false,
            configure_serial: None,
            wants_close: false,
            maximized: false,
            fullscreen: false,
            minimized: false,
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

    /// Acknowledge a ping response from a client.
    pub fn ack_ping(&mut self, serial: u32) {
        if self.ping_serial == Some(serial) {
            self.ping_serial = None;
        }
    }

    /// Check if a ping is currently pending.
    pub fn ping_pending(&self) -> bool {
        self.ping_serial.is_some()
    }

    // Activation token methods
    pub fn set_pending_token_serial(&mut self, token_id: u32, serial: u64) {
        self.pending_token_serial.insert(token_id, serial);
    }
    pub fn set_pending_token_app_id(&mut self, token_id: u32, app_id: &str) {
        self.pending_token_app_id.insert(token_id, app_id.to_string());
    }
    pub fn set_pending_token_surface(&mut self, token_id: u32, surface_id: u32) {
        self.pending_token_surface.insert(token_id, surface_id);
    }
    pub fn register_activation_token(&mut self, token: &str, client_id: u32) {
        self.activation_tokens.insert(token.to_string(), client_id);
    }
    pub fn validate_activation_token(&self, token: &str) -> Option<u32> {
        self.activation_tokens.get(token).copied()
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

    // ─── Layer shell methods ───────────────────────────────

    /// Create a new layer surface.
    pub fn create_layer_surface(
        &mut self,
        layer_surface_id: u32,
        surface_id: u32,
        layer: u32,
        anchor: u32,
        exclusive_zone: i32,
        margin_top: i32,
        margin_right: i32,
        margin_bottom: i32,
        margin_left: i32,
        keyboard_interactivity: u32,
        desired_width: u32,
        desired_height: u32,
    ) {
        self.layer_surfaces.insert(layer_surface_id, LayerSurface {
            id: layer_surface_id,
            surface_id,
            client_id: 0, // set by caller if needed
            layer,
            anchor,
            exclusive_zone,
            margin_top,
            margin_right,
            margin_bottom,
            margin_left,
            keyboard_interactivity,
            desired_width,
            desired_height,
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            configured: false,
            configure_serial: None,
            closed: false,
        });
    }

    /// Send configure to a layer surface and compute its position.
    pub fn configure_layer_surface(&mut self, layer_surface_id: u32, _width: u32, _height: u32) -> u32 {
        let serial = self.configure_serial;
        self.configure_serial += 1;
        let output_w = self.output_width as u32;
        let output_h = self.output_height as u32;

        // Gather data needed for layout, then release borrow
        let (anchor, margin_top, margin_right, margin_bottom, margin_left, desired_width, desired_height) =
            if let Some(ls) = self.layer_surfaces.get(&layer_surface_id) {
                (ls.anchor, ls.margin_top, ls.margin_right, ls.margin_bottom, ls.margin_left,
                 ls.desired_width, ls.desired_height)
            } else {
                return serial;
            };

        // Compute layout
        let margin_t = margin_top as i32;
        let margin_r = margin_right as i32;
        let margin_b = margin_bottom as i32;
        let margin_l = margin_left as i32;
        let mut w = if desired_width > 0 { desired_width }
            else if anchor & crate::protocol::layer_shell::anchor::LEFT != 0
                && anchor & crate::protocol::layer_shell::anchor::RIGHT != 0
            { (output_w as i32 - margin_l - margin_r).max(0) as u32 }
            else { output_w };
        let mut h = if desired_height > 0 { desired_height }
            else if anchor & crate::protocol::layer_shell::anchor::TOP != 0
                && anchor & crate::protocol::layer_shell::anchor::BOTTOM != 0
            { (output_h as i32 - margin_t - margin_b).max(0) as u32 }
            else { output_h };
        let h_center = (output_w as i32 - w as i32) / 2;
        let v_center = (output_h as i32 - h as i32) / 2;
        let x = if anchor & crate::protocol::layer_shell::anchor::LEFT != 0 { margin_l }
            else if anchor & crate::protocol::layer_shell::anchor::RIGHT != 0
            { output_w as i32 - w as i32 - margin_r }
            else { h_center };
        let y = if anchor & crate::protocol::layer_shell::anchor::TOP != 0 { margin_t }
            else if anchor & crate::protocol::layer_shell::anchor::BOTTOM != 0
            { output_h as i32 - h as i32 - margin_b }
            else { v_center };

        // Apply to the layer surface
        if let Some(ls) = self.layer_surfaces.get_mut(&layer_surface_id) {
            ls.configure_serial = Some(serial);
            ls.configured = false;
            ls.x = x;
            ls.y = y;
            ls.width = w;
            ls.height = h;
        }

        serial
    }

    /// Acknowledge a configure from a client.
    pub fn ack_layer_configure(&mut self, layer_surface_id: u32, serial: u32) {
        if let Some(ls) = self.layer_surfaces.get_mut(&layer_surface_id) {
            if ls.configure_serial == Some(serial) {
                ls.configured = true;
            }
        }
    }

    /// Set layer surface size.
    pub fn set_layer_surface_size(&mut self, layer_surface_id: u32, width: u32, height: u32) {
        if let Some(ls) = self.layer_surfaces.get_mut(&layer_surface_id) {
            ls.desired_width = width;
            ls.desired_height = height;
        }
    }

    /// Set layer surface anchor.
    pub fn set_layer_surface_anchor(&mut self, layer_surface_id: u32, anchor: u32) {
        if let Some(ls) = self.layer_surfaces.get_mut(&layer_surface_id) {
            ls.anchor = anchor;
        }
    }

    /// Set layer surface exclusive zone.
    pub fn set_layer_surface_exclusive_zone(&mut self, layer_surface_id: u32, zone: i32) {
        if let Some(ls) = self.layer_surfaces.get_mut(&layer_surface_id) {
            ls.exclusive_zone = zone;
        }
    }

    /// Set layer surface margins.
    pub fn set_layer_surface_margin(
        &mut self, layer_surface_id: u32,
        top: i32, right: i32, bottom: i32, left: i32,
    ) {
        if let Some(ls) = self.layer_surfaces.get_mut(&layer_surface_id) {
            ls.margin_top = top;
            ls.margin_right = right;
            ls.margin_bottom = bottom;
            ls.margin_left = left;
        }
    }

    /// Set layer surface keyboard interactivity.
    pub fn set_layer_surface_keyboard_interactivity(&mut self, layer_surface_id: u32, value: u32) {
        if let Some(ls) = self.layer_surfaces.get_mut(&layer_surface_id) {
            ls.keyboard_interactivity = value;
        }
    }

    /// Set layer surface layer.
    pub fn set_layer_surface_layer(&mut self, layer_surface_id: u32, layer: u32) {
        if let Some(ls) = self.layer_surfaces.get_mut(&layer_surface_id) {
            ls.layer = layer;
        }
    }

    /// Set a popup's parent to be this layer surface.
    pub fn set_layer_popup_parent(&mut self, layer_surface_id: u32, popup_id: u32) {
        self.layer_popup_map.insert(popup_id, layer_surface_id);
    }

    /// Destroy a layer surface.
    pub fn destroy_layer_surface(&mut self, layer_surface_id: u32) {
        if let Some(ls) = self.layer_surfaces.remove(&layer_surface_id) {
            // Send keyboard focus away if this surface had it
        }
    }

    /// Get layer surface IDs for a client.
    pub fn layer_surfaces_for_client(&self, client_id: u32) -> Vec<u32> {
        self.layer_surfaces.iter()
            .filter(|(_, ls)| ls.client_id == client_id)
            .map(|(&id, _)| id)
            .collect()
    }

    /// Get all layer surfaces sorted by layer (background → overlay).
    pub fn layer_surfaces_z_order(&self) -> Vec<(u32, i32, i32, u32, u32)> {
        let mut result: Vec<_> = self.layer_surfaces.values()
            .filter(|ls| !ls.closed && ls.configured)
            .map(|ls| (ls.surface_id, ls.x, ls.y, ls.width, ls.height, ls.layer))
            .collect();
        result.sort_by_key(|&(_, _, _, _, _, layer)| layer);
        result.into_iter().map(|(sid, x, y, w, h, _)| (sid, x, y, w, h)).collect()
    }

    /// Get all layer surfaces in layer order (for rendering).
    pub fn layer_surfaces_render_order(&self) -> Vec<&LayerSurface> {
        let mut surfaces: Vec<_> = self.layer_surfaces.values()
            .filter(|ls| !ls.closed && ls.configured)
            .collect();
        surfaces.sort_by_key(|ls| ls.layer);
        surfaces
    }

    /// Check if any layer surface wants keyboard focus.
    pub fn layer_keyboard_focus(&self) -> Option<u32> {
        // Return the highest-layer surface with exclusive keyboard interactivity
        self.layer_surfaces.values()
            .filter(|ls| !ls.closed && ls.configured && ls.keyboard_interactivity == 1)
            .max_by_key(|ls| ls.layer)
            .map(|ls| ls.surface_id)
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

    #[test]
    fn test_layer_surface_create() {
        let mut shell = Shell::new(100);
        shell.create_layer_surface(
            1, 5, 2,  // layer_surface_id, surface_id, layer (top)
            0, 0, 0, 0, 0, 0,  // anchor, exclusive_zone, margins(4)
            0, 0, 0,  // keyboard_interactivity, desired_width, desired_height
        );
        assert!(shell.layer_surfaces.contains_key(&1));
        let ls = shell.layer_surfaces.get(&1).unwrap();
        assert_eq!(ls.layer, 2);
        assert_eq!(ls.surface_id, 5);
    }

    #[test]
    fn test_layer_surface_configure_and_layout() {
        let mut shell = Shell::new(100);
        shell.set_output_size(1920, 1080, 60000, 520, 290, 1);
        shell.create_layer_surface(1, 5, 2, 1, 0, 10, 20, 30, 40, 0, 0, 0);
        let serial = shell.configure_layer_surface(1, 1920, 1080);
        assert!(serial > 0);
        let ls = shell.layer_surfaces.get(&1).unwrap();
        // Top anchor = 1, so y should be margin_top = 10
        assert_eq!(ls.y, 10);
        // Not stretched horizontally, so x should be centered
        assert_eq!(ls.x, 0); // centered: (1920 - 1920) / 2 = 0
    }

    #[test]
    fn test_layer_surface_anchor_stretch() {
        let mut shell = Shell::new(100);
        shell.set_output_size(1920, 1080, 60000, 520, 290, 1);
        // Anchor top + bottom, left + right = stretch both ways
        let anchor = 1 | 2 | 4 | 8; // top | bottom | left | right
        shell.create_layer_surface(1, 5, 1, anchor, 0, 50, 60, 70, 80, 0, 0, 0);
        shell.configure_layer_surface(1, 1920, 1080);
        let ls = shell.layer_surfaces.get(&1).unwrap();
        assert_eq!(ls.width, 1920 - 80 - 60); // output - left_margin - right_margin
        assert_eq!(ls.height, 1080 - 50 - 70); // output - top_margin - bottom_margin
        assert_eq!(ls.x, 80);
        assert_eq!(ls.y, 50);
    }

    #[test]
    fn test_layer_surface_render_order() {
        let mut shell = Shell::new(100);
        shell.set_output_size(1920, 1080, 60000, 520, 290, 1);
        shell.create_layer_surface(1, 5, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0); // overlay
        shell.create_layer_surface(2, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0); // background
        shell.create_layer_surface(3, 7, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0); // top
        // Configure and ack all to make them renderable
        let s1 = shell.configure_layer_surface(1, 1920, 1080);
        let s2 = shell.configure_layer_surface(2, 1920, 1080);
        let s3 = shell.configure_layer_surface(3, 1920, 1080);
        shell.ack_layer_configure(1, s1);
        shell.ack_layer_configure(2, s2);
        shell.ack_layer_configure(3, s3);
        let order = shell.layer_surfaces_render_order();
        // background(0) -> top(2) -> overlay(3)
        assert_eq!(order[0].layer, 0);
        assert_eq!(order[1].layer, 2);
        assert_eq!(order[2].layer, 3);
    }

    #[test]
    fn test_layer_surface_destroy() {
        let mut shell = Shell::new(100);
        shell.create_layer_surface(1, 5, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0);
        assert!(shell.layer_surfaces.contains_key(&1));
        shell.destroy_layer_surface(1);
        assert!(!shell.layer_surfaces.contains_key(&1));
    }

    #[test]
    fn test_layer_keyboard_focus() {
        let mut shell = Shell::new(100);
        shell.set_output_size(1920, 1080, 60000, 520, 290, 1);
        // Layer surface with exclusive keyboard interactivity
        shell.create_layer_surface(1, 5, 2, 0, 0, 0, 0, 0, 0, 1, 0, 0);
        let s1 = shell.configure_layer_surface(1, 1920, 1080);
        shell.ack_layer_configure(1, s1);
        assert_eq!(shell.layer_keyboard_focus(), Some(5));
        // Without keyboard interactivity
        shell.create_layer_surface(2, 6, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0);
        let s2 = shell.configure_layer_surface(2, 1920, 1080);
        shell.ack_layer_configure(2, s2);
        // Only layer 1 has keyboard interactivity
        assert_eq!(shell.layer_keyboard_focus(), Some(5));
    }
}
