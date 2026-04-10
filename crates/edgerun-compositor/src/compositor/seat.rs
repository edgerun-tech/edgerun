//! Seat management — keyboard, pointer, focus tracking.

/// Input seat state.
pub struct Seat {
    /// wl_seat object id.
    pub id: u32,
    /// Capabilities (bitmask).
    pub capabilities: u32,

    // Focus tracking
    /// Currently focused keyboard surface.
    keyboard_focus: Option<u32>,
    /// Currently focused pointer surface.
    pointer_focus: Option<u32>,
    /// Pointer position (compositor-global).
    pub pointer_x: f64,
    pub pointer_y: f64,

    /// Serial counter.
    serial: u32,
}

impl Seat {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            capabilities: 0,
            keyboard_focus: None,
            pointer_focus: None,
            pointer_x: 0.0,
            pointer_y: 0.0,
            serial: 1,
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

    /// Get the currently focused keyboard surface.
    pub fn keyboard_focus(&self) -> Option<u32> {
        self.keyboard_focus
    }

    /// Get the currently focused pointer surface.
    pub fn pointer_focus(&self) -> Option<u32> {
        self.pointer_focus
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_seat() {
        let seat = Seat::new(1);
        assert_eq!(seat.id, 1);
        assert_eq!(seat.capabilities, 0);
        assert_eq!(seat.pointer_x, 0.0);
        assert_eq!(seat.pointer_y, 0.0);
        assert!(seat.keyboard_focus.is_none());
        assert!(seat.pointer_focus.is_none());
    }

    #[test]
    fn test_serial_counter() {
        let mut seat = Seat::new(1);
        assert_eq!(seat.next_serial(), 1);
        assert_eq!(seat.next_serial(), 2);
        assert_eq!(seat.next_serial(), 3);
    }

    #[test]
    fn test_keyboard_focus_change() {
        let mut seat = Seat::new(1);
        assert!(seat.keyboard_focus().is_none());
        assert!(seat.set_keyboard_focus(Some(5)));
        assert_eq!(seat.keyboard_focus(), Some(5));
        // Setting same focus returns false
        assert!(!seat.set_keyboard_focus(Some(5)));
        // Setting different focus returns true
        assert!(seat.set_keyboard_focus(Some(6)));
        assert_eq!(seat.keyboard_focus(), Some(6));
        // Clearing focus returns true
        assert!(seat.set_keyboard_focus(None));
        assert!(seat.keyboard_focus().is_none());
    }

    #[test]
    fn test_pointer_focus_change() {
        let mut seat = Seat::new(1);
        assert!(seat.pointer_focus().is_none());
        assert!(seat.set_pointer_focus(Some(5)));
        assert_eq!(seat.pointer_focus(), Some(5));
        assert!(!seat.set_pointer_focus(Some(5)));
        assert!(seat.set_pointer_focus(Some(6)));
    }
}
