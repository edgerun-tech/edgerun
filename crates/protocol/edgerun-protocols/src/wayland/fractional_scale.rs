//! wp_fractional_scale_v1 — fractional display scaling.
//!
//! Allows clients to request non-integer scale factors (1.25x, 1.5x, etc.)
//! for HiDPI multi-monitor setups.

use alloc::vec::Vec;
use edgerun_encoding::byteorder::push_u32_le;

use crate::wayland::{ArgType, Message};

pub const WP_FRACTIONAL_SCALE_MANAGER_V1: &str = "wp_fractional_scale_manager_v1";
pub const WP_FRACTIONAL_SCALE_MANAGER_V1_VERSION: u32 = 1;

pub mod manager_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const GET_FRACTIONAL_SCALE: u16 = 1;
    pub const GET_FRACTIONAL_SCALE_SIG: &[ArgType] = &[ArgType::NewId, ArgType::Object];
    // id: new_id, surface: object
}

// ─── wp_fractional_scale_v1 ────────────────────────────────

pub const WP_FRACTIONAL_SCALE_V1: &str = "wp_fractional_scale_v1";
pub const WP_FRACTIONAL_SCALE_V1_VERSION: u32 = 1;

pub mod fractional_scale_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];
}

pub mod fractional_scale_event {
    pub const PREFERRED_SCALE: u16 = 0;
    // sig: uint(scale) — scale * 120 (e.g., 1.5x = 180)
}

/// Build preferred_scale event.
pub fn preferred_scale_event(fractional_scale_id: u32, scale: u32) -> Message {
    let mut args = Vec::new();
    push_u32_le(&mut args, scale);
    Message {
        sender_id: fractional_scale_id,
        opcode: fractional_scale_event::PREFERRED_SCALE,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}
