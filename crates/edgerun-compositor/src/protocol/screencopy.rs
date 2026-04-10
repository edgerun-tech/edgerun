//! zwlr_screencopy_v1 — screen capture protocol.
//!
//! Used by grim (screenshots), OBS, wf-recorder, etc.

use crate::wire::{ArgType, Message};
use crate::wire::encode::*;

pub const ZWLR_SCREENCOPY_MANAGER_V1: &str = "zwlr_screencopy_manager_v1";
pub const ZWLR_SCREENCOPY_MANAGER_V1_VERSION: u32 = 3;

pub mod manager_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const CAPTURE_OUTPUT: u16 = 1;
    pub const CAPTURE_OUTPUT_SIG: &[ArgType] = &[ArgType::NewId, ArgType::Uint, ArgType::Uint, ArgType::Object];
    // frame: new_id, overlay_cursor: uint, type: uint, output: object

    pub const CAPTURE_OUTPUT_REGION: u16 = 2; // v2+
    pub const CAPTURE_OUTPUT_REGION_SIG: &[ArgType] = &[ArgType::NewId, ArgType::Uint, ArgType::Uint, ArgType::Object, ArgType::Int, ArgType::Int, ArgType::Int, ArgType::Int];
}

/// Capture types.
pub mod capture_type {
    pub const OUTPUT: u32 = 0;
}

// ─── zwlr_screencopy_frame_v1 ──────────────────────────────

pub const ZWLR_SCREENCOPY_FRAME_V1: &str = "zwlr_screencopy_frame_v1";
pub const ZWLR_SCREENCOPY_FRAME_V1_VERSION: u32 = 3;

pub mod frame_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const COPY: u16 = 1;
    pub const COPY_SIG: &[ArgType] = &[ArgType::Object]; // wl_buffer

    pub const COPY_WITH_DAMAGE: u16 = 2; // v3+ — same as COPY
    pub const COPY_WITH_DAMAGE_SIG: &[ArgType] = &[ArgType::Object]; // wl_buffer
}

pub mod frame_event {
    /// buffer — wl_shm buffer information.
    pub const BUFFER: u16 = 0;
    // sig: uint(format), uint(width), uint(height), uint(stride)

    /// failed — capture failed.
    pub const FAILED: u16 = 1;

    /// ready — frame copy completed.
    pub const READY: u16 = 2;
    // sig: none

    /// stopped — capture source was destroyed.
    pub const STOPPED: u16 = 3; // v2+

    /// damage — damaged region (v3+).
    pub const DAMAGE: u16 = 4;
    // sig: uint(x), uint(y), uint(width), uint(height)

    /// linux_dmabuf — dmabuf format info (v3+).
    pub const LINUX_DMABUF: u16 = 5;
    // sig: uint(format), uint(width), uint(height)

    /// flags — frame flags after copy (v3+).
    pub const FLAGS: u16 = 6;
    // sig: uint(flags)
}

/// Frame flags.
pub mod frame_flags {
    /// Y axis is inverted (typical for GL rendering).
    pub const Y_INVERT: u32 = 0x1;
}

/// Build buffer event (sent after frame creation, tells client the buffer params).
pub fn frame_buffer_event(frame_id: u32, format: u32, width: u32, height: u32, stride: u32) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&format.to_le_bytes());
    args.extend_from_slice(&width.to_le_bytes());
    args.extend_from_slice(&height.to_le_bytes());
    args.extend_from_slice(&stride.to_le_bytes());
    Message {
        sender_id: frame_id,
        opcode: frame_event::BUFFER,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build failed event.
pub fn frame_failed_event(frame_id: u32) -> Message {
    message_empty(frame_id, frame_event::FAILED)
}

/// Build ready event (copy complete).
pub fn frame_ready_event(frame_id: u32) -> Message {
    message_empty(frame_id, frame_event::READY)
}

/// Build stopped event.
pub fn frame_stopped_event(frame_id: u32) -> Message {
    message_empty(frame_id, frame_event::STOPPED)
}

/// Build flags event.
pub fn frame_flags_event(frame_id: u32, flags: u32) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&flags.to_le_bytes());
    Message {
        sender_id: frame_id,
        opcode: frame_event::FLAGS,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build damage event.
pub fn frame_damage_event(frame_id: u32, x: u32, y: u32, width: u32, height: u32) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&x.to_le_bytes());
    args.extend_from_slice(&y.to_le_bytes());
    args.extend_from_slice(&width.to_le_bytes());
    args.extend_from_slice(&height.to_le_bytes());
    Message {
        sender_id: frame_id,
        opcode: frame_event::DAMAGE,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Screencopy frame state — tracks pending copy requests.
#[derive(Debug, Clone)]
pub struct ScreencopyFrame {
    pub frame_id: u32,
    pub client_id: u32,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: u32,
    /// Region to capture (x, y, width, height). None = full output.
    pub region: Option<(i32, i32, i32, i32)>,
    /// Whether COPY has been called.
    pub copy_requested: bool,
    /// Client's target SHM pool FD (set when client creates buffer for capture).
    pub target_pool_fd: Option<i32>,
    /// Client's target buffer offset in the SHM pool.
    pub target_offset: u32,
    /// Client's target buffer size.
    pub target_size: usize,
}
