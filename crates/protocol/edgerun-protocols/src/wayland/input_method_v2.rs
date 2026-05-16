//! zwp_input_method_v2 — input method protocol (v2).
//!
//! Used by Fcitx5, SwayOSK, and modern IME frameworks.

use crate::wayland::encode::*;
use crate::wayland::{ArgType, Message};
use alloc::vec;
use alloc::vec::Vec;
use edgerun_encoding::byteorder::{push_i32_le, push_u32_le};

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
    pub const SET_SURROUNDING_TEXT_SIG: &[ArgType] =
        &[ArgType::String, ArgType::Uint, ArgType::Uint];

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
    push_u32_le(&mut args, text_input_id);
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
    push_u32_le(&mut args, text_input_id);
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

/// Build surround_text event.
pub fn input_method_surround_text_event(
    input_method_id: u32,
    text: &str,
    cursor: u32,
    anchor: u32,
) -> Message {
    let mut args = Vec::new();
    encode_string(&mut args, text);
    push_u32_le(&mut args, cursor);
    push_u32_le(&mut args, anchor);
    Message {
        sender_id: input_method_id,
        opcode: input_method_event::SURROUND_TEXT,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build text_change_cause event.
pub fn input_method_text_change_cause_event(input_method_id: u32, cause: u32) -> Message {
    message_uint(
        input_method_id,
        input_method_event::TEXT_CHANGE_CAUSE,
        cause,
    )
}

/// Build content_type event.
pub fn input_method_content_type_event(input_method_id: u32, hint: u32, purpose: u32) -> Message {
    let mut args = Vec::new();
    push_u32_le(&mut args, hint);
    push_u32_le(&mut args, purpose);
    Message {
        sender_id: input_method_id,
        opcode: input_method_event::CONTENT_TYPE,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
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

/// Build keyboard grab key event.
pub fn keyboard_grab_key_event(
    grab_id: u32,
    serial: u32,
    time: u32,
    key: u32,
    state: u32,
) -> Message {
    let mut args = Vec::new();
    push_u32_le(&mut args, serial);
    push_u32_le(&mut args, time);
    push_u32_le(&mut args, key);
    push_u32_le(&mut args, state);
    Message {
        sender_id: grab_id,
        opcode: keyboard_grab_event::KEY,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build keyboard grab modifiers event.
pub fn keyboard_grab_modifiers_event(
    grab_id: u32,
    serial: u32,
    depressed: u32,
    latched: u32,
    locked: u32,
    group: u32,
) -> Message {
    let mut args = Vec::new();
    push_u32_le(&mut args, serial);
    push_u32_le(&mut args, depressed);
    push_u32_le(&mut args, latched);
    push_u32_le(&mut args, locked);
    push_u32_le(&mut args, group);
    Message {
        sender_id: grab_id,
        opcode: keyboard_grab_event::MODIFIERS,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build keyboard grab keymap event.
pub fn keyboard_grab_keymap_event(grab_id: u32, format: u32, fd: i32, size: u32) -> Message {
    let mut args = Vec::new();
    push_u32_le(&mut args, format);
    push_u32_le(&mut args, size);
    Message {
        sender_id: grab_id,
        opcode: keyboard_grab_event::KEYMAP,
        size: (8 + args.len()) as u16,
        args,
        fds: vec![fd],
    }
}

/// Build keyboard grab repeat_info event.
pub fn keyboard_grab_repeat_info_event(grab_id: u32, rate: i32, delay: i32) -> Message {
    let mut args = Vec::new();
    push_i32_le(&mut args, rate);
    push_i32_le(&mut args, delay);
    Message {
        sender_id: grab_id,
        opcode: keyboard_grab_event::REPEAT_INFO,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// IME state — tracks the current input method server state.
#[derive(Debug)]
pub struct IMEState {
    /// Input method object ID.
    pub input_method_id: u32,
    /// Client ID of the IME server.
    pub client_id: u32,
    /// Currently active text input object.
    pub active_text_input_id: Option<u32>,
    /// Keyboard grab object.
    pub keyboard_grab_id: Option<u32>,
    /// Whether keyboard grab is active.
    pub keyboard_grab_active: bool,
}

impl IMEState {
    pub fn new(input_method_id: u32, client_id: u32) -> Self {
        Self {
            input_method_id,
            client_id,
            active_text_input_id: None,
            keyboard_grab_id: None,
            keyboard_grab_active: false,
        }
    }
}
