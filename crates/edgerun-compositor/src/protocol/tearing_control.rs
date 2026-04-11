//! wp_tearing_control_v1 — variable refresh rate / tear-free control.
//!
//! Allows clients to request tearing-free presentation or variable refresh
//! rate for gaming/video playback.

use crate::wire::ArgType;

pub const WP_TEARING_CONTROL_MANAGER_V1: &str = "wp_tearing_control_manager_v1";
pub const WP_TEARING_CONTROL_MANAGER_V1_VERSION: u32 = 1;

pub mod manager_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const GET_TEARING_CONTROL: u16 = 1;
    pub const GET_TEARING_CONTROL_SIG: &[ArgType] = &[ArgType::NewId, ArgType::Object];
    // id: new_id, surface: object
}

// ─── wp_tearing_control_v1 ─────────────────────────────────

pub const WP_TEARING_CONTROL_V1: &str = "wp_tearing_control_v1";
pub const WP_TEARING_CONTROL_V1_VERSION: u32 = 1;

pub mod tearing_control_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const SET_PRESENTATION_HINT: u16 = 1;
    pub const SET_PRESENTATION_HINT_SIG: &[ArgType] = &[ArgType::Uint];
    // hint: uint (0 = default, 1 = sync, 2 = async)
}

/// Presentation hints.
pub mod hint {
    pub const DEFAULT: u32 = 0;
    pub const SYNC: u32 = 1;   // VSync, tear-free
    pub const ASYNC: u32 = 2;  // Allow tearing for low latency
}
