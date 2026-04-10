//! zwlr_data_control_v1 — headless clipboard access.
//!
//! Used by wl-copy/wl-paste, cliphist, Waybar.

use crate::wire::{ArgType, Message};
use crate::wire::encode::*;

pub const ZWLR_DATA_CONTROL_MANAGER_V1: &str = "zwlr_data_control_manager_v1";
pub const ZWLR_DATA_CONTROL_MANAGER_V1_VERSION: u32 = 2;

pub mod manager_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const CREATE_DATA_SOURCE: u16 = 1;
    pub const CREATE_DATA_SOURCE_SIG: &[ArgType] = &[ArgType::NewId];

    pub const GET_DATA_DEVICE: u16 = 2;
    pub const GET_DATA_DEVICE_SIG: &[ArgType] = &[ArgType::NewId, ArgType::Object];
    // id: new_id, seat: object
}

// ─── zwlr_data_control_device_v1 ──────────────────────────

pub const ZWLR_DATA_CONTROL_DEVICE_V1: &str = "zwlr_data_control_device_v1";
pub const ZWLR_DATA_CONTROL_DEVICE_V1_VERSION: u32 = 2;

pub mod data_control_device_request {
    use super::*;

    pub const SET_SELECTION: u16 = 0;
    pub const SET_SELECTION_SIG: &[ArgType] = &[ArgType::Object]; // source(nullable)

    pub const RELEASE: u16 = 1;
    pub const RELEASE_SIG: &[ArgType] = &[];
}

pub mod data_control_device_event {
    pub const DATA_OFFER: u16 = 0;
    // sig: new_id(wlr_data_control_offer_v1)

    pub const SELECTION: u16 = 1;
    // sig: object(id)

    pub const FINISHED: u16 = 2;
}

/// Build data offer event.
pub fn data_control_device_data_offer_event(device_id: u32, offer_id: u32) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&offer_id.to_le_bytes());
    Message {
        sender_id: device_id,
        opcode: data_control_device_event::DATA_OFFER,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build selection event.
pub fn data_control_device_selection_event(device_id: u32, offer_id: u32) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&offer_id.to_le_bytes());
    Message {
        sender_id: device_id,
        opcode: data_control_device_event::SELECTION,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

// ─── zwlr_data_control_offer_v1 ───────────────────────────

pub const ZWLR_DATA_CONTROL_OFFER_V1: &str = "zwlr_data_control_offer_v1";
pub const ZWLR_DATA_CONTROL_OFFER_V1_VERSION: u32 = 1;

pub mod data_control_offer_request {
    use super::*;

    pub const RECEIVE: u16 = 0;
    pub const RECEIVE_SIG: &[ArgType] = &[ArgType::String, ArgType::Fd];

    pub const DESTROY: u16 = 1;
    pub const DESTROY_SIG: &[ArgType] = &[];
}

pub mod data_control_offer_event {
    pub const OFFER: u16 = 0;
    // sig: string(mime_type)
}

/// Build offer event.
pub fn data_control_offer_event(offer_id: u32, mime_type: &str) -> Message {
    let mut args = Vec::new();
    encode_string(&mut args, mime_type);
    Message {
        sender_id: offer_id,
        opcode: data_control_offer_event::OFFER,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

// ─── zwlr_data_control_source_v1 ──────────────────────────

pub const ZWLR_DATA_CONTROL_SOURCE_V1: &str = "zwlr_data_control_source_v1";
pub const ZWLR_DATA_CONTROL_SOURCE_V1_VERSION: u32 = 1;

pub mod data_control_source_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const OFFER: u16 = 1;
    pub const OFFER_SIG: &[ArgType] = &[ArgType::String];
}

pub mod data_control_source_event {
    pub const SEND: u16 = 0;
    // sig: string(mime_type), fd

    pub const CANCELLED: u16 = 1;
}

/// Build send event.
pub fn data_control_source_send_event(source_id: u32, mime_type: &str, fd: i32) -> Message {
    let mut args = Vec::new();
    encode_string(&mut args, mime_type);
    Message {
        sender_id: source_id,
        opcode: data_control_source_event::SEND,
        size: (8 + args.len()) as u16,
        args,
        fds: vec![fd],
    }
}
