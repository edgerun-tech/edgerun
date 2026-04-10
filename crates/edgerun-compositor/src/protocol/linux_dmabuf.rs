//! linux-dmabuf-v1 protocol — DMA-BUF buffer sharing.
//!
//! This is a minimal stub that advertises support for XRGB8888 and ARGB8888 formats.
//! The compositor doesn't actually use dmabuf for compositing (yet), so clients
//! that send dmabuf buffers will have their buffers accepted but not rendered.

use crate::wire::{ArgType, Message};
use crate::wire::encode::*;
use dmabuf_event::*;

pub const ZWP_LINUX_DMABUF_V1: &str = "zwp_linux_dmabuf_v1";
pub const ZWP_LINUX_DMABUF_V1_VERSION: u32 = 4;

/// Supported dmabuf formats.
pub mod dmabuf_format {
    pub const XRGB8888: u32 = 0x34325258;
    pub const ARGB8888: u32 = 0x34325241;
    pub const XBGR8888: u32 = 0x34324258;
    pub const ABGR8888: u32 = 0x34324241;
}

/// DRM modifier: LINEAR (no tiling).
pub const DRM_FORMAT_MOD_LINEAR: u64 = 0x00_00_00_00_00_00_00_00;
/// DRM modifier: INVALID (for formats without modifiers).
pub const DRM_FORMAT_MOD_INVALID: u64 = 0x00_ff_ffff_ffff_ffff;

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
    /// format — advertise a supported format.
    pub const FORMAT: u16 = 0;
    // sig: uint (format)

    /// modifier — advertise a supported format+modifier combo.
    pub const MODIFIER: u16 = 1;
    // sig: uint (format), uint (modifier_hi), uint (modifier_lo)
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
