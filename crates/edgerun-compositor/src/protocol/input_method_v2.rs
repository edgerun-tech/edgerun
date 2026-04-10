//! zwp_input_method_v2 — input method protocol (v2).
//!
//! Used by Fcitx5, SwayOSK, and modern IME frameworks.

use crate::wire::{ArgType, Message};
use crate::wire::encode::*;

pub const ZWP_INPUT_METHOD_MANAGER_V2: &str = "zwp_input_method_manager_v2";
pub const ZWP_INPUT_METHOD_MANAGER_V2_VERSION: u32 = 1;

pub mod manager_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const GET_INPUT_METHOD: u16 = 1;
    pub const GET_INPUT_METHOD_SIG: &[ArgType] = &[ArgType::NewId, ArgType::Object];
    // id: new_id, seat: object
}

// ─── zwp_input_method_v2 ───────────────────────────────────

pub const ZWP_INPUT_METHOD_V2: &str = "zwp_input_method_v2";
pub const ZWP_INPUT_METHOD_V2_VERSION: u32 = 1;

pub mod input_method_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const COMMIT_STRING: u16 = 1;
    pub const COMMIT_STRING_SIG: &[ArgType] = &[ArgType::String];

    pub const COMMIT_PREEDIT: u16 = 2;
    pub const COMMIT_PREEDIT_SIG: &[ArgType] = &[ArgType::String, ArgType::Int, ArgType::Int];
    // text, cursor_begin, cursor_end

    pub const DELETE_SURROUNDING_TEXT: u16 = 3;
    pub const DELETE_SURROUNDING_TEXT_SIG: &[ArgType] = &[ArgType::Int, ArgType::Int];
    // before_length, after_length

    pub const COMMIT: u16 = 4;
    pub const COMMIT_SIG: &[ArgType] = &[];

    pub const GRAB_KEYBOARD: u16 = 5;
    pub const GRAB_KEYBOARD_SIG: &[ArgType] = &[ArgType::NewId];
    // keyboard: new_id(zwp_input_method_keyboard_grab_v2)

    pub const SET_SURROUNDING_TEXT: u16 = 6;
    pub const SET_SURROUNDING_TEXT_SIG: &[ArgType] = &[ArgType::String, ArgType::Uint, ArgType::Uint];

    pub const SET_TEXT_CHANGE_CAUSE: u16 = 7;
    pub const SET_TEXT_CHANGE_CAUSE_SIG: &[ArgType] = &[ArgType::Uint];

    pub const SET_CONTENT_TYPE: u16 = 8;
    pub const SET_CONTENT_TYPE_SIG: &[ArgType] = &[ArgType::Uint, ArgType::Uint];
    // hint, purpose

    pub const AVAILABLE: u16 = 9;
    pub const AVAILABLE_SIG: &[ArgType] = &[];
}

pub mod input_method_event {
    /// activate — input method should activate for a surface.
    pub const ACTIVATE: u16 = 0;
    // sig: object(zwp_text_input_v3)

    /// deactivate — input method should deactivate.
    pub const DEACTIVATE: u16 = 1;
    // sig: object(zwp_text_input_v3)

    /// surround_text — surrounding text changed.
    pub const SURROUND_TEXT: u16 = 2;
    // sig: string(text), uint(cursor), uint(anchor)

    /// text_change_cause — cause of text change.
    pub const TEXT_CHANGE_CAUSE: u16 = 3;
    // sig: uint(cause)

    /// content_type — content type hint changed.
    pub const CONTENT_TYPE: u16 = 4;
    // sig: uint(hint), uint(purpose)

    /// done — all events for this commit sent.
    pub const DONE: u16 = 5;
    // sig: none
}

/// Build activate event.
pub fn input_method_activate_event(input_method_id: u32, text_input_id: u32) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&text_input_id.to_le_bytes());
    Message {
        sender_id: input_method_id,
        opcode: input_method_event::ACTIVATE,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build deactivate event.
pub fn input_method_deactivate_event(input_method_id: u32, text_input_id: u32) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&text_input_id.to_le_bytes());
    Message {
        sender_id: input_method_id,
        opcode: input_method_event::DEACTIVATE,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build done event.
pub fn input_method_done_event(input_method_id: u32) -> Message {
    message_empty(input_method_id, input_method_event::DONE)
}

// ─── zwp_input_method_keyboard_grab_v2 ────────────────────

pub const ZWP_INPUT_METHOD_KEYBOARD_GRAB_V2: &str = "zwp_input_method_keyboard_grab_v2";
pub const ZWP_INPUT_METHOD_KEYBOARD_GRAB_V2_VERSION: u32 = 1;

pub mod keyboard_grab_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const RELEASE: u16 = 1;
    pub const RELEASE_SIG: &[ArgType] = &[];
}

pub mod keyboard_grab_event {
    pub const KEY: u16 = 0;
    // sig: uint(serial), uint(time), uint(key), uint(state)

    pub const MODIFIERS: u16 = 1;
    // sig: uint(serial), uint(depressed), uint(latched), uint(locked), uint(group)

    pub const KEYMAP: u16 = 2;
    // sig: uint(format), fd, uint(size)

    pub const REPEAT_INFO: u16 = 3;
    // sig: int(rate), int(delay)
}
