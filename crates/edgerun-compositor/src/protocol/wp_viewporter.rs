//! wp_viewporter — surface viewporting protocol.
//!
//! Used by Chromium for HiDPI and fractional scaling.

use crate::wire::ArgType;

pub const WP_VIEWPORTER: &str = "wp_viewporter";
pub const WP_VIEWPORTER_VERSION: u32 = 1;

pub mod viewporter_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const GET_VIEWPORT: u16 = 1;
    pub const GET_VIEWPORT_SIG: &[ArgType] = &[ArgType::NewId, ArgType::Object];
    // viewport, surface
}

// ─── wp_viewport ─────────────────────────────────────────────

pub const WP_VIEWPORT: &str = "wp_viewport";
pub const WP_VIEWPORT_VERSION: u32 = 1;

pub mod viewport_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const SET_SOURCE: u16 = 1;
    pub const SET_SOURCE_SIG: &[ArgType] = &[
        ArgType::Fixed,
        ArgType::Fixed,
        ArgType::Fixed,
        ArgType::Fixed,
    ];
    // x, y, width, height (all fixed point)

    pub const SET_DESTINATION: u16 = 2;
    pub const SET_DESTINATION_SIG: &[ArgType] = &[ArgType::Int, ArgType::Int]; // width, height (-1 = use buffer size)
}

// ─── Helper to convert fixed-point s15.16 to f64 ────────────

pub fn fixed_to_f64(v: i32) -> f64 {
    v as f64 / 65536.0
}
