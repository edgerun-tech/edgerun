//! zwp_text_input_v3 — text input protocol (v3).
//!
//! Used by GTK4, Qt6, and modern toolkits for IME support.

use crate::wire::{ArgType, Message};
use crate::wire::encode::*;

pub const ZWP_TEXT_INPUT_MANAGER_V3: &str = "zwp_text_input_manager_v3";
pub const ZWP_TEXT_INPUT_MANAGER_V3_VERSION: u32 = 1;

pub mod manager_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const GET_TEXT_INPUT: u16 = 1;
    pub const GET_TEXT_INPUT_SIG: &[ArgType] = &[ArgType::NewId, ArgType::Object];
    // id: new_id, seat: object
}

// ─── zwp_text_input_v3 ─────────────────────────────────────

pub const ZWP_TEXT_INPUT_V3: &str = "zwp_text_input_v3";
pub const ZWP_TEXT_INPUT_V3_VERSION: u32 = 1;

pub mod text_input_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const ENABLE: u16 = 1;
    pub const ENABLE_SIG: &[ArgType] = &[ArgType::Object, ArgType::Uint];
    // surface: object, serial: uint

    pub const DISABLE: u16 = 2;
    pub const DISABLE_SIG: &[ArgType] = &[ArgType::Object, ArgType::Uint];
    // surface: object, serial: uint

    pub const SET_SURROUNDING_TEXT: u16 = 3;
    pub const SET_SURROUNDING_TEXT_SIG: &[ArgType] = &[ArgType::String, ArgType::Uint, ArgType::Uint];
    // text, cursor, anchor

    pub const SET_TEXT_CHANGE_CAUSE: u16 = 4;
    pub const SET_TEXT_CHANGE_CAUSE_SIG: &[ArgType] = &[ArgType::Uint];
    // cause: uint (0=input_method, 1=other)

    pub const COMMIT: u16 = 5;
    pub const COMMIT_SIG: &[ArgType] = &[];

    pub const GET_SURROUNDING_TEXT: u16 = 6;
    pub const GET_SURROUNDING_TEXT_SIG: &[ArgType] = &[ArgType::Uint];
    // before_length, after_length (packed in one uint)
}

pub mod text_input_event {
    /// enter — input focus entered a surface.
    pub const ENTER: u16 = 0;
    // sig: object(surface)

    /// leave — input focus left a surface.
    pub const LEAVE: u16 = 1;
    // sig: object(surface), uint(serial)

    /// preedit_string — pre-edit text changed.
    pub const PREEDIT_STRING: u16 = 2;
    // sig: string(text), int(cursor_begin), int(cursor_end)

    /// commit_string — committed text.
    pub const COMMIT_STRING: u16 = 3;
    // sig: string(text)

    /// delete_surrounding_text — delete text around cursor.
    pub const DELETE_SURROUNDING_TEXT: u16 = 4;
    // sig: int(before_length), int(after_length)

    /// done — all events for this commit are sent.
    pub const DONE: u16 = 5;
    // sig: uint(serial)
}

/// Build enter event.
pub fn text_input_enter_event(text_input_id: u32, surface_id: u32) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&surface_id.to_le_bytes());
    Message {
        sender_id: text_input_id,
        opcode: text_input_event::ENTER,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build leave event.
pub fn text_input_leave_event(text_input_id: u32, surface_id: u32, serial: u32) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&surface_id.to_le_bytes());
    args.extend_from_slice(&serial.to_le_bytes());
    Message {
        sender_id: text_input_id,
        opcode: text_input_event::LEAVE,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build preedit_string event.
pub fn text_input_preedit_string_event(text_input_id: u32, text: &str, cursor_begin: i32, cursor_end: i32) -> Message {
    let mut args = Vec::new();
    encode_string(&mut args, text);
    args.extend_from_slice(&cursor_begin.to_le_bytes());
    args.extend_from_slice(&cursor_end.to_le_bytes());
    Message {
        sender_id: text_input_id,
        opcode: text_input_event::PREEDIT_STRING,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build commit_string event.
pub fn text_input_commit_string_event(text_input_id: u32, text: &str) -> Message {
    let mut args = Vec::new();
    encode_string(&mut args, text);
    Message {
        sender_id: text_input_id,
        opcode: text_input_event::COMMIT_STRING,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build done event.
pub fn text_input_done_event(text_input_id: u32, serial: u32) -> Message {
    message_uint(text_input_id, text_input_event::DONE, serial)
}
