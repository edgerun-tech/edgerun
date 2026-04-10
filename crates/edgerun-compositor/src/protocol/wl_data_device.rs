//! wl_data_device_manager, wl_data_device, wl_data_source, wl_data_offer.
//!
//! Clipboard and drag-and-drop protocol implementation.

use crate::wire::{ArgType, Message};
use crate::wire::encode::*;

pub const WL_DATA_DEVICE_MANAGER: &str = "wl_data_device_manager";
pub const WL_DATA_DEVICE_MANAGER_VERSION: u32 = 3;

/// Data device manager opcodes.
pub mod dnd_manager_request {
    use super::*;

    pub const CREATE_DATA_SOURCE: u16 = 0;
    pub const CREATE_DATA_SOURCE_SIG: &[ArgType] = &[ArgType::NewId];

    pub const GET_DATA_DEVICE: u16 = 1;
    pub const GET_DATA_DEVICE_SIG: &[ArgType] = &[ArgType::NewId, ArgType::Object]; // id, seat

    pub const DESTROY: u16 = 2;
    pub const DESTROY_SIG: &[ArgType] = &[];
}

/// Create a data source object.
pub fn create_data_source_event(manager_id: u32, new_id: u32) -> Message {
    // This is implicit — client creates via new_id, no server event needed
    message_empty(manager_id, 0)
}

// ─── wl_data_source ──────────────────────────────────────────

pub const WL_DATA_SOURCE: &str = "wl_data_source";
pub const WL_DATA_SOURCE_VERSION: u32 = 3;

pub mod data_source_request {
    use super::*;

    pub const OFFER: u16 = 0;
    pub const OFFER_SIG: &[ArgType] = &[ArgType::String]; // mime_type

    pub const DESTROY: u16 = 1;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const SET_ACTIONS: u16 = 2; // version 3+
    pub const SET_ACTIONS_SIG: &[ArgType] = &[ArgType::Uint, ArgType::Uint]; // dnd_action, preferred_action
}

pub mod data_source_event {
    pub const TARGET: u16 = 0;
    // sig: string(mime_type) or empty string for no target

    pub const SEND: u16 = 1;
    // sig: string(mime_type), fd

    pub const CANCELLED: u16 = 2;
    // sig: none

    pub const DND_DROP_PERFORMED: u16 = 3; // version 3+
    // sig: none

    pub const DND_FINISHED: u16 = 4; // version 3+
    // sig: none
}

/// Build target event.
pub fn data_source_target_event(source_id: u32, mime_type: &str) -> Message {
    let mut args = Vec::new();
    encode_string(&mut args, mime_type);
    Message {
        sender_id: source_id,
        opcode: data_source_event::TARGET,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build send event.
pub fn data_source_send_event(source_id: u32, mime_type: &str, fd: i32) -> Message {
    let mut args = Vec::new();
    encode_string(&mut args, mime_type);
    Message {
        sender_id: source_id,
        opcode: data_source_event::SEND,
        size: (8 + args.len()) as u16,
        args,
        fds: vec![fd],
    }
}

/// Build cancelled event.
pub fn data_source_cancelled_event(source_id: u32) -> Message {
    message_empty(source_id, data_source_event::CANCELLED)
}

// ─── wl_data_offer ───────────────────────────────────────────

pub const WL_DATA_OFFER: &str = "wl_data_offer";
pub const WL_DATA_OFFER_VERSION: u32 = 3;

pub mod data_offer_request {
    use super::*;

    pub const ACCEPT: u16 = 0;
    pub const ACCEPT_SIG: &[ArgType] = &[ArgType::Uint, ArgType::String]; // serial, mime_type (or empty)

    pub const RECEIVE: u16 = 1;
    pub const RECEIVE_SIG: &[ArgType] = &[ArgType::String, ArgType::Fd]; // mime_type, fd

    pub const DISCRIPTION: u16 = 2; // note: typo in wayland protocol
    pub const DISCRIPTION_SIG: &[ArgType] = &[];

    pub const SET_ACTIONS: u16 = 3; // version 3+
    pub const SET_ACTIONS_SIG: &[ArgType] = &[ArgType::Uint, ArgType::Uint]; // dnd_actions, preferred_action
}

pub mod data_offer_event {
    pub const OFFER: u16 = 0;
    // sig: string(mime_type)

    pub const SOURCE_ACTIONS: u16 = 1; // version 3+
    // sig: uint(actions)
}

/// Build offer event (mime type available from source).
pub fn data_offer_offer_event(offer_id: u32, mime_type: &str) -> Message {
    let mut args = Vec::new();
    encode_string(&mut args, mime_type);
    Message {
        sender_id: offer_id,
        opcode: data_offer_event::OFFER,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build source actions event.
pub fn data_offer_source_actions_event(offer_id: u32, actions: u32) -> Message {
    message_uint(offer_id, data_offer_event::SOURCE_ACTIONS, actions)
}

// ─── wl_data_device ──────────────────────────────────────────

pub const WL_DATA_DEVICE: &str = "wl_data_device";
pub const WL_DATA_DEVICE_VERSION: u32 = 3;

/// DnD action flags.
pub mod dnd_action {
    pub const NONE: u32 = 0;
    pub const COPY: u32 = 1;
    pub const MOVE: u32 = 2;
    pub const ASK: u32 = 4;
}

pub mod data_device_request {
    use super::*;

    pub const START_DRAG: u16 = 0;
    pub const START_DRAG_SIG: &[ArgType] = &[ArgType::Object, ArgType::Object, ArgType::Object, ArgType::Uint];
    // source, surface, icon(nullable), serial

    pub const SET_SELECTION: u16 = 1;
    pub const SET_SELECTION_SIG: &[ArgType] = &[ArgType::Object, ArgType::Uint]; // source(nullable), serial

    pub const RELEASE: u16 = 2; // version 3+
    pub const RELEASE_SIG: &[ArgType] = &[];
}

pub mod data_device_event {
    pub const DATA_OFFER: u16 = 0;
    // sig: new_id(wl_data_offer)

    pub const ENTER: u16 = 1;
    // sig: uint(serial), object(surface), fixed(x), fixed(y), object(wl_data_offer or nil)

    pub const LEAVE: u16 = 2;
    // sig: none

    pub const MOTION: u16 = 3;
    // sig: uint(time), fixed(x), fixed(y)

    pub const DROP: u16 = 4;
    // sig: none

    pub const SELECTION: u16 = 5;
    // sig: object(wl_data_offer or nil)
}

/// Build data_offer event.
pub fn data_device_data_offer_event(device_id: u32, offer_id: u32) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&offer_id.to_le_bytes());
    Message {
        sender_id: device_id,
        opcode: data_device_event::DATA_OFFER,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build enter event.
pub fn data_device_enter_event(
    device_id: u32,
    serial: u32,
    surface_id: u32,
    x: f64,
    y: f64,
    offer_id: u32,
) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&serial.to_le_bytes());
    args.extend_from_slice(&surface_id.to_le_bytes());
    let fx = (x * 65536.0).round() as i32;
    let fy = (y * 65536.0).round() as i32;
    args.extend_from_slice(&fx.to_le_bytes());
    args.extend_from_slice(&fy.to_le_bytes());
    args.extend_from_slice(&offer_id.to_le_bytes());
    Message {
        sender_id: device_id,
        opcode: data_device_event::ENTER,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build leave event.
pub fn data_device_leave_event(device_id: u32) -> Message {
    message_empty(device_id, data_device_event::LEAVE)
}

/// Build motion event.
pub fn data_device_motion_event(device_id: u32, time: u32, x: f64, y: f64) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&time.to_le_bytes());
    let fx = (x * 65536.0).round() as i32;
    let fy = (y * 65536.0).round() as i32;
    args.extend_from_slice(&fx.to_le_bytes());
    args.extend_from_slice(&fy.to_le_bytes());
    Message {
        sender_id: device_id,
        opcode: data_device_event::MOTION,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build drop event.
pub fn data_device_drop_event(device_id: u32) -> Message {
    message_empty(device_id, data_device_event::DROP)
}

/// Build selection event.
pub fn data_device_selection_event(device_id: u32, offer_id: u32) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&offer_id.to_le_bytes());
    Message {
        sender_id: device_id,
        opcode: data_device_event::SELECTION,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}
