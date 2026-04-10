//! xdg-output-v1 protocol — output description for clients.
//!
//! Provides logical output information (name, description, position, scale)
//! that Chromium and other clients use for proper window placement.

use crate::wire::{ArgType, Message};
use crate::wire::encode::*;

pub const ZXDG_OUTPUT_MANAGER_V1: &str = "zxdg_output_manager_v1";
pub const ZXDG_OUTPUT_MANAGER_V1_VERSION: u32 = 3;

pub mod output_manager_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const GET_XDG_OUTPUT: u16 = 1;
    pub const GET_XDG_OUTPUT_SIG: &[ArgType] = &[ArgType::NewId, ArgType::Object];
    // id: new_id, output: object(wl_output)
}

// ─── zxdg_output_v1 ──────────────────────────────────────────

pub const ZXDG_OUTPUT_V1: &str = "zxdg_output_v1";
pub const ZXDG_OUTPUT_V1_VERSION: u32 = 3;

pub mod xdg_output_event {
    /// logical_position — output position in global compositor space.
    pub const LOGICAL_POSITION: u16 = 0;
    // sig: int(x), int(y)

    /// logical_size — output size in global compositor space.
    pub const LOGICAL_SIZE: u16 = 1;
    // sig: int(width), int(height)

    /// done — all information has been sent.
    pub const DONE: u16 = 2;
    // sig: none

    /// name — output name (v3+).
    pub const NAME: u16 = 3;
    // sig: string

    /// description — human-readable output description (v3+).
    pub const DESCRIPTION: u16 = 4;
    // sig: string
}

/// Build logical_position event.
pub fn xdg_output_logical_position(output_id: u32, x: i32, y: i32) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&x.to_le_bytes());
    args.extend_from_slice(&y.to_le_bytes());
    Message {
        sender_id: output_id,
        opcode: xdg_output_event::LOGICAL_POSITION,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build logical_size event.
pub fn xdg_output_logical_size(output_id: u32, width: i32, height: i32) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&width.to_le_bytes());
    args.extend_from_slice(&height.to_le_bytes());
    Message {
        sender_id: output_id,
        opcode: xdg_output_event::LOGICAL_SIZE,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build done event.
pub fn xdg_output_done_event(output_id: u32) -> Message {
    message_empty(output_id, xdg_output_event::DONE)
}

/// Build name event (v3+).
pub fn xdg_output_name_event(output_id: u32, name: &str) -> Message {
    let mut args = Vec::new();
    encode_string(&mut args, name);
    Message {
        sender_id: output_id,
        opcode: xdg_output_event::NAME,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build description event (v3+).
pub fn xdg_output_description_event(output_id: u32, description: &str) -> Message {
    let mut args = Vec::new();
    encode_string(&mut args, description);
    Message {
        sender_id: output_id,
        opcode: xdg_output_event::DESCRIPTION,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}
