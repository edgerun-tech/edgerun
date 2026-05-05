//! Seat management — keyboard, pointer, touch, focus tracking.

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
    /// Currently focused touch surface.
    touch_focus: Option<u32>,
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
            touch_focus: None,
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

    /// Set touch focus to a surface. Returns true if focus changed.
    pub fn set_touch_focus(&mut self, surface_id: Option<u32>) -> bool {
        if self.touch_focus != surface_id {
            self.touch_focus = surface_id;
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

    /// Get the currently focused touch surface.
    pub fn touch_focus(&self) -> Option<u32> {
        self.touch_focus
    }
}

/// Pointer constraint state — tracks locked/confined pointers.
#[derive(Debug, Clone)]
pub struct PointerConstraint {
    /// The constraint object ID (locked_pointer_v1 or confined_pointer_v1).
    pub constraint_id: u32,
    /// The surface being locked/confined to.
    pub surface_id: u32,
    /// The pointer object ID.
    pub pointer_id: u32,
    /// Lifetime: 0 = oneshot (auto-release), 1 = persistent.
    pub lifetime: u32,
    /// Optional confine region (x, y, width, height in surface-local coords).
    pub region: Option<(i32, i32, i32, i32)>,
    /// Whether this constraint has been activated (sent locked/confined event).
    pub activated: bool,
}

/// Type of pointer constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintType {
    Lock,
    Confine,
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
        assert!(seat.touch_focus.is_none());
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
        assert!(!seat.set_keyboard_focus(Some(5)));
        assert!(seat.set_keyboard_focus(Some(6)));
        assert_eq!(seat.keyboard_focus(), Some(6));
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

    #[test]
    fn test_touch_focus_change() {
        let mut seat = Seat::new(1);
        assert!(seat.touch_focus().is_none());
        assert!(seat.set_touch_focus(Some(5)));
        assert_eq!(seat.touch_focus(), Some(5));
        assert!(!seat.set_touch_focus(Some(5)));
        assert!(seat.set_touch_focus(Some(6)));
        assert!(seat.set_touch_focus(None));
        assert!(seat.touch_focus().is_none());
    }
}
