//! zwlr_primary_selection_v1 — X11-style primary selection.
//!
//! Used by GTK apps for middle-click paste.

use crate::wire::encode::*;
use crate::wire::{ArgType, Message};

pub const ZWLR_PRIMARY_SELECTION_MANAGER_V1: &str = "zwlr_primary_selection_manager_v1";
pub const ZWLR_PRIMARY_SELECTION_MANAGER_V1_VERSION: u32 = 1;

pub mod manager_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const CREATE_DATA_SOURCE: u16 = 1;
    pub const CREATE_DATA_SOURCE_SIG: &[ArgType] = &[ArgType::NewId];

    pub const GET_PRIMARY_SELECTION: u16 = 2;
    pub const GET_PRIMARY_SELECTION_SIG: &[ArgType] = &[ArgType::NewId, ArgType::Object];
    // id: new_id, seat: object
}

// ─── zwlr_primary_selection_v1 (device) ─────────────────────

pub const ZWLR_PRIMARY_SELECTION_DEVICE_V1: &str = "zwlr_primary_selection_device_v1";
pub const ZWLR_PRIMARY_SELECTION_DEVICE_V1_VERSION: u32 = 1;

pub mod device_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const SET_SELECTION: u16 = 1;
    pub const SET_SELECTION_SIG: &[ArgType] = &[ArgType::Object]; // source or null
}

pub mod device_event {
    pub const SELECTION: u16 = 0;
    // sig: object(id)
}

/// Build selection event.
pub fn device_selection_event(device_id: u32, offer_id: u32) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&offer_id.to_le_bytes());
    Message {
        sender_id: device_id,
        opcode: device_event::SELECTION,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

// ─── zwlr_primary_selection_v1 ─────────────────────────────

pub const ZWLR_PRIMARY_SELECTION_V1: &str = "zwlr_primary_selection_v1";
pub const ZWLR_PRIMARY_SELECTION_V1_VERSION: u32 = 1;

pub mod primary_selection_event {
    pub const PRIMARY_SELECTION: u16 = 0;
    // sig: object(id)
}

/// Build primary selection event.
pub fn primary_selection_event(device_id: u32, offer_id: u32) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&offer_id.to_le_bytes());
    Message {
        sender_id: device_id,
        opcode: primary_selection_event::PRIMARY_SELECTION,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

// ─── zwlr_primary_selection_offer_v1 ──────────────────────

pub const ZWLR_PRIMARY_SELECTION_OFFER_V1: &str = "zwlr_primary_selection_offer_v1";
pub const ZWLR_PRIMARY_SELECTION_OFFER_V1_VERSION: u32 = 1;

pub mod offer_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const RECEIVE: u16 = 1;
    pub const RECEIVE_SIG: &[ArgType] = &[ArgType::String, ArgType::Fd];
}

pub mod offer_event {
    pub const OFFER: u16 = 0;
    // sig: string(mime_type)
}

/// Build offer event.
pub fn offer_offer_event(offer_id: u32, mime_type: &str) -> Message {
    let mut args = Vec::new();
    encode_string(&mut args, mime_type);
    Message {
        sender_id: offer_id,
        opcode: offer_event::OFFER,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

// ─── zwlr_primary_selection_source_v1 ─────────────────────

pub const ZWLR_PRIMARY_SELECTION_SOURCE_V1: &str = "zwlr_primary_selection_source_v1";
pub const ZWLR_PRIMARY_SELECTION_SOURCE_V1_VERSION: u32 = 1;

pub mod source_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const OFFER: u16 = 1;
    pub const OFFER_SIG: &[ArgType] = &[ArgType::String];
}

pub mod source_event {
    pub const SEND: u16 = 0;
    // sig: string(mime_type), fd

    pub const CANCELLED: u16 = 1;
}

/// Build send event.
pub fn source_send_event(source_id: u32, mime_type: &str, fd: i32) -> Message {
    let mut args = Vec::new();
    encode_string(&mut args, mime_type);
    Message {
        sender_id: source_id,
        opcode: source_event::SEND,
        size: (8 + args.len()) as u16,
        args,
        fds: vec![fd],
    }
}
