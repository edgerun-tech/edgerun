//! zwp_text_input_v1 — text input method protocol.
//!
//! This protocol provides text input method support which browsers use for
//! IME (Input Method Editor) composition, especially important for CJK
//! (Chinese/Japanese/Korean) input.

use crate::wire::{ArgType, Message};
use crate::wire::encode::encode_string;

pub const ZWP_TEXT_INPUT_MANAGER_V1: &str = "zwp_text_input_manager_v1";
pub const ZWP_TEXT_INPUT_MANAGER_V1_VERSION: u32 = 1;

pub mod text_input_manager_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const CREATE_TEXT_INPUT: u16 = 1;
    pub const CREATE_TEXT_INPUT_SIG: &[ArgType] = &[ArgType::NewId]; // zwp_text_input_v1
}

// ─── zwp_text_input_v1 ──────────────────────────────────────

pub const ZWP_TEXT_INPUT_V1: &str = "zwp_text_input_v1";
pub const ZWP_TEXT_INPUT_V1_VERSION: u32 = 1;

pub mod text_input_request {
    use super::*;

    pub const ACTIVATE: u16 = 0;
    pub const ACTIVATE_SIG: &[ArgType] = &[ArgType::Object, ArgType::Object]; // seat, surface

    pub const DEACTIVATE: u16 = 1;
    pub const DEACTIVATE_SIG: &[ArgType] = &[ArgType::Object]; // seat

    pub const SHOW_INPUT_PANEL: u16 = 2;
    pub const SHOW_INPUT_PANEL_SIG: &[ArgType] = &[];

    pub const HIDE_INPUT_PANEL: u16 = 3;
    pub const HIDE_INPUT_PANEL_SIG: &[ArgType] = &[];

    pub const RESET: u16 = 4;
    pub const RESET_SIG: &[ArgType] = &[];

    pub const SET_SURROUNDING_TEXT: u16 = 5;
    pub const SET_SURROUNDING_TEXT_SIG: &[ArgType] = &[ArgType::String, ArgType::Uint, ArgType::Uint];

    pub const SET_CONTENT_TYPE: u16 = 6;
    pub const SET_CONTENT_TYPE_SIG: &[ArgType] = &[ArgType::Uint, ArgType::Uint];

    pub const SET_CURSOR_RECTANGLE: u16 = 7;
    pub const SET_CURSOR_RECTANGLE_SIG: &[ArgType] = &[ArgType::Int, ArgType::Int, ArgType::Int, ArgType::Int];

    pub const SET_PREFERRED_LANGUAGE: u16 = 8;
    pub const SET_PREFERRED_LANGUAGE_SIG: &[ArgType] = &[ArgType::String];

    pub const COMMIT_STATE: u16 = 9;
    pub const COMMIT_STATE_SIG: &[ArgType] = &[ArgType::Uint];

    pub const INVOKE_ACTION: u16 = 10;
    pub const INVOKE_ACTION_SIG: &[ArgType] = &[ArgType::Uint, ArgType::Uint];

    pub const DESTROY: u16 = 11;
    pub const DESTROY_SIG: &[ArgType] = &[];
}

pub mod text_input_event {
    pub const ENTER: u16 = 0;
    pub const LEAVE: u16 = 1;
    pub const MODIFIERS_MAP: u16 = 2;
    pub const INPUT_PANEL_STATE: u16 = 3;
    pub const PREEDIT_STRING: u16 = 4;
    pub const PREEDIT_STYLING: u16 = 5;
    pub const PREEDIT_CURSOR: u16 = 6;
    pub const COMMIT_STRING: u16 = 7;
    pub const CURSOR_POSITION: u16 = 8;
    pub const DELETE_SURROUNDING_TEXT: u16 = 9;
    pub const KEYSYM: u16 = 10;
    pub const LANGUAGE: u16 = 11;
}

/// Build an enter event.
pub fn enter_event(
    text_input_id: u32,
    surface_id: u32,
) -> Message {
    use text_input_event::ENTER;
    let mut args = Vec::new();
    args.extend_from_slice(&surface_id.to_le_bytes());
    Message {
        sender_id: text_input_id,
        opcode: ENTER,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build a leave event.
pub fn leave_event(
    text_input_id: u32,
    surface_id: u32,
) -> Message {
    use text_input_event::LEAVE;
    let mut args = Vec::new();
    args.extend_from_slice(&surface_id.to_le_bytes());
    Message {
        sender_id: text_input_id,
        opcode: LEAVE,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build a preedit_string event.
pub fn preedit_string_event(
    text_input_id: u32,
    serial: u32,
    text: &str,
    commit: &str,
) -> Message {
    use text_input_event::PREEDIT_STRING;
    let mut args = Vec::new();
    args.extend_from_slice(&serial.to_le_bytes());
    encode_string(&mut args, text);
    encode_string(&mut args, commit);
    Message {
        sender_id: text_input_id,
        opcode: PREEDIT_STRING,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build a commit_string event.
pub fn commit_string_event(
    text_input_id: u32,
    serial: u32,
    text: &str,
) -> Message {
    use text_input_event::COMMIT_STRING;
    let mut args = Vec::new();
    args.extend_from_slice(&serial.to_le_bytes());
    encode_string(&mut args, text);
    Message {
        sender_id: text_input_id,
        opcode: COMMIT_STRING,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build a cursor_position event.
pub fn cursor_position_event(
    text_input_id: u32,
    index: u32,
    anchor: u32,
) -> Message {
    use text_input_event::CURSOR_POSITION;
    let mut args = Vec::new();
    args.extend_from_slice(&index.to_le_bytes());
    args.extend_from_slice(&anchor.to_le_bytes());
    Message {
        sender_id: text_input_id,
        opcode: CURSOR_POSITION,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build a delete_surrounding_text event.
pub fn delete_surrounding_text_event(
    text_input_id: u32,
    index: u32,
    length: u32,
) -> Message {
    use text_input_event::DELETE_SURROUNDING_TEXT;
    let mut args = Vec::new();
    args.extend_from_slice(&index.to_le_bytes());
    args.extend_from_slice(&length.to_le_bytes());
    Message {
        sender_id: text_input_id,
        opcode: DELETE_SURROUNDING_TEXT,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}
