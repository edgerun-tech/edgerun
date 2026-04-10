//! xdg-decoration — server-side window decorations.
//!
//! Allows the compositor to control whether clients draw their own decorations
//! or whether the compositor draws them (CSD vs SSD).

use crate::wire::{ArgType, Message};
use crate::wire::encode::*;

pub const ZXDG_DECORATION_MANAGER_V1: &str = "zxdg_decoration_manager_v1";
pub const ZXDG_DECORATION_MANAGER_V1_VERSION: u32 = 1;

pub mod decoration_manager_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const GET_TOPLEVEL_DECORATION: u16 = 1;
    pub const GET_TOPLEVEL_DECORATION_SIG: &[ArgType] = &[ArgType::NewId, ArgType::Object];
    // toplevel_decoration: new_id, toplevel: object(xdg_toplevel)
}

pub mod decoration_manager_event {
    // No events from the manager itself
}

/// Build dummy event (manager has no events).
pub fn decoration_manager_noop_event(manager_id: u32) -> Message {
    message_empty(manager_id, 0)
}

// ─── zxdg_toplevel_decoration_v1 ─────────────────────────────

pub const ZXDG_TOPLEVEL_DECORATION_V1: &str = "zxdg_toplevel_decoration_v1";
pub const ZXDG_TOPLEVEL_DECORATION_V1_VERSION: u32 = 1;

/// Decoration mode.
pub mod decoration_mode {
    /// Client-side decorations.
    pub const CLIENT_SIDE: u32 = 1;
    /// Server-side decorations.
    pub const SERVER_SIDE: u32 = 2;
}

pub mod toplevel_decoration_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const SET_MODE: u16 = 1;
    pub const SET_MODE_SIG: &[ArgType] = &[ArgType::Uint]; // mode

    pub const UNSET_MODE: u16 = 2;
    pub const UNSET_MODE_SIG: &[ArgType] = &[];
}

pub mod toplevel_decoration_event {
    pub const CONFIGURE: u16 = 0;
    // sig: uint(mode)

    pub const REMOVE: u16 = 1;
    // sig: none
}

/// Build configure event — tell the client which mode to use.
pub fn toplevel_decoration_configure_event(decoration_id: u32, mode: u32) -> Message {
    message_uint(decoration_id, toplevel_decoration_event::CONFIGURE, mode)
}

/// Build remove event — compositor no longer wants to manage decorations.
pub fn toplevel_decoration_remove_event(decoration_id: u32) -> Message {
    message_empty(decoration_id, toplevel_decoration_event::REMOVE)
}
