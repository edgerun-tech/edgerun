//! zwp_relative_pointer_v1 — relative pointer motion events.
//!
//! This protocol provides relative pointer motion events (delta values) which
//! browsers use for smooth scrolling and pointer-based interactions.

use crate::wire::{ArgType, Message};
use crate::wire::encode::*;

pub const ZWP_RELATIVE_POINTER_MANAGER_V1: &str = "zwp_relative_pointer_manager_v1";
pub const ZWP_RELATIVE_POINTER_MANAGER_V1_VERSION: u32 = 1;

pub mod relative_pointer_manager_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const GET_RELATIVE_POINTER: u16 = 1;
    pub const GET_RELATIVE_POINTER_SIG: &[ArgType] = &[ArgType::Object, ArgType::NewId]; // wl_pointer, zwp_relative_pointer_v1
}

// ─── zwp_relative_pointer_v1 ─────────────────────────────────

pub const ZWP_RELATIVE_POINTER_V1: &str = "zwp_relative_pointer_v1";
pub const ZWP_RELATIVE_POINTER_V1_VERSION: u32 = 1;

pub mod relative_pointer_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];
}

pub mod relative_pointer_event {
    pub const RELATIVE_MOTION: u16 = 0;
    // sig: uint (utime_hi), uint (utime_lo), fixed (dx), fixed (dy), fixed (dx_unaccel), fixed (dy_unaccel)
}

/// Build a relative_motion event.
pub fn relative_motion_event(
    relative_pointer_id: u32,
    utime_hi: u32,
    utime_lo: u32,
    dx: u32,
    dy: u32,
    dx_unacc: u32,
    dy_unacc: u32,
) -> Message {
    use relative_pointer_event::RELATIVE_MOTION;
    let mut args = Vec::new();
    args.extend_from_slice(&utime_hi.to_le_bytes());
    args.extend_from_slice(&utime_lo.to_le_bytes());
    args.extend_from_slice(&dx.to_le_bytes());
    args.extend_from_slice(&dy.to_le_bytes());
    args.extend_from_slice(&dx_unacc.to_le_bytes());
    args.extend_from_slice(&dy_unacc.to_le_bytes());
    Message {
        sender_id: relative_pointer_id,
        opcode: RELATIVE_MOTION,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}
