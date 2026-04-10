//! linux-dmabuf-v1 protocol — DMA-BUF buffer sharing.
//!
//! Advertises common formats with modifiers (LINEAR + implicit modifier).
//! Clients use this to negotiate buffer formats and import via EGL.

use crate::wire::{ArgType, Message};
use crate::wire::encode::*;

pub const ZWP_LINUX_DMABUF_V1: &str = "zwp_linux_dmabuf_v1";
pub const ZWP_LINUX_DMABUF_V1_VERSION: u32 = 4;

/// Supported dmabuf formats.
pub mod dmabuf_format {
    pub const XRGB8888: u32 = 0x34325258;
    pub const ARGB8888: u32 = 0x34325241;
    pub const XBGR8888: u32 = 0x34324258;
    pub const ABGR8888: u32 = 0x34324241;
    pub const RGB565: u32 = 0x34324752;
    pub const XRGB2101010: u32 = 0x30335258;
    pub const ARGB2101010: u32 = 0x30335241;
    pub const NV12: u32 = 0x3231564e;
}

/// DRM modifier: LINEAR (no tiling).
pub const DRM_FORMAT_MOD_LINEAR: u64 = 0x00_00_00_00_00_00_00_00;
/// DRM modifier: INVALID (for formats without modifiers — pre-modifier protocol).
pub const DRM_FORMAT_MOD_INVALID: u64 = 0x00_ff_ffff_ffff_ffff;
/// DRM vendor modifier: Intel i915 tiling (X).
pub const I915_FORMAT_MOD_X_TILED: u64 = 0x01_00_00_00_00_00_00_01;
/// DRM vendor modifier: Intel i915 tiling (Y).
pub const I915_FORMAT_MOD_Y_TILED: u64 = 0x01_00_00_00_00_00_00_02;
/// DRM vendor modifier: AMD tiling.
pub const AMD_FMT_MOD_TILE_VER_GFX9: u64 = 0x02_00_00_00_00_00_00_09;

pub mod dmabuf_request {
    use super::*;

    /// destroy
    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    /// create_params — create a buffer from dmabuf fds.
    pub const CREATE_PARAMS: u16 = 1;
    pub const CREATE_PARAMS_SIG: &[ArgType] = &[ArgType::NewId, ArgType::Int, ArgType::Int, ArgType::Uint];
    // id: new_id, width: int, height: int, format: uint
    // Then a variable number of (fd, plane_idx, offset, stride, modifier_hi, modifier_lo) tuples
    // Each tuple: fd, int, uint, uint, uint, uint

    /// create_immed — create a buffer directly from a single fd.
    pub const CREATE_IMMED: u16 = 2;
    pub const CREATE_IMMED_SIG: &[ArgType] = &[ArgType::NewId, ArgType::Fd, ArgType::Int, ArgType::Int, ArgType::Int, ArgType::Uint, ArgType::Uint];
}

pub mod dmabuf_event {
    /// format — advertise a supported format (v1).
    pub const FORMAT: u16 = 0;
    // sig: uint (format)

    /// modifier — advertise a supported format+modifier combo (v3+).
    pub const MODIFIER: u16 = 1;
    // sig: uint (format), uint (modifier_hi), uint (modifier_lo)

    /// format32 — same as format but for older clients (deprecated).
    pub const FORMAT32: u16 = 2;
}

/// Build a format event.
pub fn dmabuf_format_event(dmabuf_id: u32, format: u32) -> Message {
    use dmabuf_event::FORMAT;
    message_uint(dmabuf_id, FORMAT, format)
}

/// Build a modifier event.
pub fn dmabuf_modifier_event(dmabuf_id: u32, format: u32, modifier_hi: u32, modifier_lo: u32) -> Message {
    use dmabuf_event::MODIFIER;
    let mut args = Vec::new();
    args.extend_from_slice(&format.to_le_bytes());
    args.extend_from_slice(&modifier_hi.to_le_bytes());
    args.extend_from_slice(&modifier_lo.to_le_bytes());
    Message {
        sender_id: dmabuf_id,
        opcode: MODIFIER,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Common format + modifier pairs to advertise.
pub fn common_formats_with_modifiers() -> Vec<(u32, u64)> {
    vec![
        (dmabuf_format::XRGB8888, DRM_FORMAT_MOD_LINEAR),
        (dmabuf_format::XRGB8888, DRM_FORMAT_MOD_INVALID),
        (dmabuf_format::ARGB8888, DRM_FORMAT_MOD_LINEAR),
        (dmabuf_format::ARGB8888, DRM_FORMAT_MOD_INVALID),
        (dmabuf_format::XBGR8888, DRM_FORMAT_MOD_LINEAR),
        (dmabuf_format::XBGR8888, DRM_FORMAT_MOD_INVALID),
        (dmabuf_format::ABGR8888, DRM_FORMAT_MOD_LINEAR),
        (dmabuf_format::ABGR8888, DRM_FORMAT_MOD_INVALID),
    ]
}
