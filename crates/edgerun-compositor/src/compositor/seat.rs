//! Seat management — keyboard, pointer, focus tracking.

use std::collections::HashMap;

/// Input seat state.
pub struct Seat {
    /// wl_seat object id.
    pub id: u32,
    /// Seat name.
    pub name: String,
    /// Capabilities (bitmask).
    pub capabilities: u32,

    // Keyboard
    /// wl_keyboard object id.
    pub keyboard_id: Option<u32>,
    /// Keymap fd (for xkbcommon text).
    pub keymap_fd: Option<i32>,
    /// Keymap size in bytes.
    pub keymap_size: Option<u32>,
    /// Current modifier state.
    pub mods_depressed: u32,
    pub mods_latched: u32,
    pub mods_locked: u32,
    pub group: u32,

    // Pointer
    /// wl_pointer object id.
    pub pointer_id: Option<u32>,

    // Focus tracking
    /// Currently focused keyboard surface.
    pub keyboard_focus: Option<u32>,
    /// Currently focused pointer surface.
    pub pointer_focus: Option<u32>,
    /// Pointer position (compositor-global).
    pub pointer_x: f64,
    pub pointer_y: f64,

    /// Serial counter.
    serial: u32,

    /// Pending pointer motion events (buffered until frame).
    pending_motion: Option<(u32, f64, f64)>,
}

impl Seat {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            name: "seat0".to_string(),
            capabilities: 0,
            keyboard_id: None,
            keymap_fd: None,
            keymap_size: None,
            mods_depressed: 0,
            mods_latched: 0,
            mods_locked: 0,
            group: 0,
            pointer_id: None,
            keyboard_focus: None,
            pointer_focus: None,
            pointer_x: 0.0,
            pointer_y: 0.0,
            serial: 1,
            pending_motion: None,
        }
    }

    /// Get next serial number.
    pub fn next_serial(&mut self) -> u32 {
        let s = self.serial;
        self.serial += 1;
        s
    }

    /// Set keyboard focus to a surface. Returns true if focus changed.
    pub fn set_keyboard_focus(&mut self, surface_id: Option<u32>) -> bool {
        if self.keyboard_focus != surface_id {
            self.keyboard_focus = surface_id;
            true
        } else {
            false
        }
    }

    /// Set pointer focus to a surface. Returns true if focus changed.
    pub fn set_pointer_focus(&mut self, surface_id: Option<u32>) -> bool {
        if self.pointer_focus != surface_id {
            self.pointer_focus = surface_id;
            true
        } else {
            false
        }
    }

    /// Update pointer position. Returns (surface_x, surface_y) in the focused surface's coords.
    pub fn update_pointer(&mut self, x: f64, y: f64) -> (f64, f64) {
        self.pointer_x = x;
        self.pointer_y = y;
        (x, y)
    }

    /// Get the currently focused keyboard surface.
    pub fn keyboard_focus(&self) -> Option<u32> {
        self.keyboard_focus
    }

    /// Get the currently focused pointer surface.
    pub fn pointer_focus(&self) -> Option<u32> {
        self.pointer_focus
    }
}

/// Global serial counter for events.
static mut GLOBAL_SERIAL: u32 = 1;

/// Get the next global serial.
pub fn next_serial() -> u32 {
    unsafe {
        let s = GLOBAL_SERIAL;
        GLOBAL_SERIAL += 1;
        s
    }
}
