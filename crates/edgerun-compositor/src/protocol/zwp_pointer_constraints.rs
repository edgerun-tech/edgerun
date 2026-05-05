//! zwp_pointer_constraints_v1 — pointer confinement and locking.
//!
//! Used by browsers for pointer lock API and games for mouse capture.

use crate::wire::{ArgType, Message};

pub const ZWP_POINTER_CONSTRAINTS_V1: &str = "zwp_pointer_constraints_v1";
pub const ZWP_POINTER_CONSTRAINTS_V1_VERSION: u32 = 1;

/// Constraint lifetime.
pub mod lifetime {
    pub const ONESHOT: u32 = 1;
    pub const PERSISTENT: u32 = 2;
}

/// Constraint hint.
pub mod hint {
    pub const NONE: u32 = 0;
    pub const WARP: u32 = 1;
}

pub mod constraints_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const LOCK_POINTER: u16 = 1;
    pub const LOCK_POINTER_SIG: &[ArgType] = &[
        ArgType::NewId,
        ArgType::Object,
        ArgType::Object,
        ArgType::Uint,
        ArgType::Uint,
    ];
    // id: new_id, surface: object, pointer: object, lifetime: uint, region: object(nullable)

    pub const CONFINE_POINTER: u16 = 2;
    pub const CONFINE_POINTER_SIG: &[ArgType] = &[
        ArgType::NewId,
        ArgType::Object,
        ArgType::Object,
        ArgType::Uint,
        ArgType::Uint,
    ];
    // id: new_id, surface: object, pointer: object, lifetime: uint, region: object(nullable)
}

// ─── zwp_locked_pointer_v1 ─────────────────────────────────

pub const ZWP_LOCKED_POINTER_V1: &str = "zwp_locked_pointer_v1";
pub const ZWP_LOCKED_POINTER_V1_VERSION: u32 = 1;

pub mod locked_pointer_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const SET_CURSOR_POSITION_HINT: u16 = 1;
    pub const SET_CURSOR_POSITION_HINT_SIG: &[ArgType] = &[ArgType::Fixed, ArgType::Fixed];
    // surface_x, surface_y (fixed point)

    pub const SET_REGION: u16 = 2;
    pub const SET_REGION_SIG: &[ArgType] = &[ArgType::Object]; // region or null
}

pub mod locked_pointer_event {
    pub const LOCKED: u16 = 0;
    // sig: none

    pub const UNLOCKED: u16 = 1;
    // sig: none

    pub const MOTION: u16 = 2;
    // sig: uint(time_hi), uint(time_lo), fixed(dx), fixed(dy)
}

/// Build locked event.
pub fn locked_pointer_locked_event(locked_id: u32) -> Message {
    crate::wire::encode::message_empty(locked_id, locked_pointer_event::LOCKED)
}

/// Build unlocked event.
pub fn locked_pointer_unlocked_event(locked_id: u32) -> Message {
    crate::wire::encode::message_empty(locked_id, locked_pointer_event::UNLOCKED)
}

/// Build locked pointer motion event.
pub fn locked_pointer_motion_event(locked_id: u32, time: u32, dx: u32, dy: u32) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&0u32.to_le_bytes()); // time_hi (we use 32-bit time)
    args.extend_from_slice(&time.to_le_bytes()); // time_lo
    args.extend_from_slice(&dx.to_le_bytes()); // dx (fixed point)
    args.extend_from_slice(&dy.to_le_bytes()); // dy (fixed point)
    Message {
        sender_id: locked_id,
        opcode: locked_pointer_event::MOTION,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

// ─── zwp_confined_pointer_v1 ───────────────────────────────

pub const ZWP_CONFINED_POINTER_V1: &str = "zwp_confined_pointer_v1";
pub const ZWP_CONFINED_POINTER_V1_VERSION: u32 = 1;

pub mod confined_pointer_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const SET_REGION: u16 = 1;
    pub const SET_REGION_SIG: &[ArgType] = &[ArgType::Object]; // region or null
}

pub mod confined_pointer_event {
    pub const CONFINED: u16 = 0;
    // sig: none

    pub const UNCONFINED: u16 = 1;
    // sig: none
}

/// Build confined event.
pub fn confined_pointer_confined_event(confined_id: u32) -> Message {
    crate::wire::encode::message_empty(confined_id, confined_pointer_event::CONFINED)
}

/// Build unconfined event.
pub fn confined_pointer_unconfined_event(confined_id: u32) -> Message {
    crate::wire::encode::message_empty(confined_id, confined_pointer_event::UNCONFINED)
}
