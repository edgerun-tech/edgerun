//! Core Wayland protocol interfaces: wl_display, wl_registry, wl_callback.

use crate::wire::{ArgType, Message};
use crate::wire::encode::*;

// ─── wl_display ──────────────────────────────────────────────

/// wl_display interface.
pub const WL_DISPLAY: &str = "wl_display";
pub const WL_DISPLAY_VERSION: u32 = 1;

/// wl_display requests (client → server).
pub mod display_request {
    use super::*;

    /// sync — ask server to emit a `done` event on the callback.
    pub const SYNC: u16 = 0;
    pub const SYNC_SIG: &[ArgType] = &[ArgType::NewId]; // callback: new_id

    /// get_registry — create a registry object.
    pub const GET_REGISTRY: u16 = 1;
    pub const GET_REGISTRY_SIG: &[ArgType] = &[ArgType::NewId]; // registry: new_id
}

/// wl_display events (server → client).
pub mod display_event {
    

    /// error — fatal error from the server.
    pub const ERROR: u16 = 0;
    // sig: object, uint, string

    /// delete_id — confirm object deletion.
    pub const DELETE_ID: u16 = 1;
    // sig: uint
}

// ─── wl_registry ─────────────────────────────────────────────

/// wl_registry interface.
pub const WL_REGISTRY: &str = "wl_registry";
pub const WL_REGISTRY_VERSION: u32 = 1;

/// wl_registry requests (client → server).
pub mod registry_request {
    use super::*;

    /// bind — bind a global object.
    pub const BIND: u16 = 0;
    pub const BIND_SIG: &[ArgType] = &[ArgType::Uint, ArgType::String, ArgType::Uint, ArgType::NewId];
    // name: u32, interface: string, version: u32, id: new_id

    /// get_global — deprecated, do not use.
    pub const GET_GLOBAL: u16 = 1;
    pub const GET_GLOBAL_SIG: &[ArgType] = &[ArgType::String];
}

/// wl_registry events (server → client).
pub mod registry_event {
    /// global — advertise a global object.
    pub const GLOBAL: u16 = 0;
    // sig: uint (name), string (interface), uint (version)

    /// global_remove — a global object has been removed.
    pub const GLOBAL_REMOVE: u16 = 1;
    // sig: uint (name)
}

/// Create a `global` event.
pub fn global_event(name: u32, interface: &str, version: u32) -> Message {
    use registry_event::GLOBAL;
    let mut args = Vec::new();
    args.extend_from_slice(&name.to_le_bytes());
    crate::wire::encode::encode_string(&mut args, interface);
    args.extend_from_slice(&version.to_le_bytes());
    Message {
        sender_id: 0,
        opcode: GLOBAL,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Create a `global_remove` event.
pub fn global_remove_event(name: u32) -> Message {
    use registry_event::GLOBAL_REMOVE;
    message_uint(0, GLOBAL_REMOVE, name)
}

// ─── wl_callback ─────────────────────────────────────────────

/// wl_callback interface.
pub const WL_CALLBACK: &str = "wl_callback";
pub const WL_CALLBACK_VERSION: u32 = 1;

/// wl_callback events (server → client).
pub mod callback_event {
    /// done — notify client that the request is complete.
    pub const DONE: u16 = 0;
    // sig: uint (callback_data, typically the serial)
}

/// Create a `done` event for a callback.
pub fn callback_done_event(callback_id: u32, serial: u32) -> Message {
    use callback_event::DONE;
    message_uint(callback_id, DONE, serial)
}
