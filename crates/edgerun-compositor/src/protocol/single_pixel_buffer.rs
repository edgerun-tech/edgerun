//! wp_single_pixel_buffer_v1 — 1x1 solid color buffers.
//!
//! Used by clients to create solid-color decorations without allocating
//! full SHM buffers. Compositor creates a 1x1 buffer filled with the
//! specified color.

use crate::wire::ArgType;

pub const WP_SINGLE_PIXEL_BUFFER_MANAGER_V1: &str = "wp_single_pixel_buffer_manager_v1";
pub const WP_SINGLE_PIXEL_BUFFER_MANAGER_V1_VERSION: u32 = 1;

pub mod manager_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const CREATE_SRGB32_BUFFER: u16 = 1;
    pub const CREATE_SRGB32_BUFFER_SIG: &[ArgType] = &[ArgType::NewId, ArgType::Uint, ArgType::Uint, ArgType::Uint, ArgType::Uint];
    // id: new_id, red: uint, green: uint, blue: uint, alpha: uint
}

// ─── wp_single_pixel_buffer_v1 ─────────────────────────────

pub const WP_SINGLE_PIXEL_BUFFER_V1: &str = "wp_single_pixel_buffer_v1";
pub const WP_SINGLE_PIXEL_BUFFER_V1_VERSION: u32 = 1;

pub mod buffer_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];
}

/// RGBA color for a single pixel buffer.
#[derive(Debug, Clone, Copy)]
pub struct SinglePixelColor {
    pub red: u32,
    pub green: u32,
    pub blue: u32,
    pub alpha: u32,
}
