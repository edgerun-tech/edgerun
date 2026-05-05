//! wl_subcompositor — subsurface protocol.
//!
//! Used by Chromium for popups, tooltips, menus, and video overlays.

use crate::wire::encode::*;
use crate::wire::{ArgType, Message};

pub const WL_SUBCOMPOSITOR: &str = "wl_subcompositor";
pub const WL_SUBCOMPOSITOR_VERSION: u32 = 1;

pub mod subcompositor_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const GET_SUBSURFACE: u16 = 1;
    pub const GET_SUBSURFACE_SIG: &[ArgType] = &[ArgType::NewId, ArgType::Object, ArgType::Object];
    // subsurface: new_id, surface: object, parent: object
}

pub mod subcompositor_event {
    pub const INFERIOR: u16 = 0;
    // Deprecated, never sent
}

// ─── wl_subsurface ───────────────────────────────────────────

pub const WL_SUBSURFACE: &str = "wl_subsurface";
pub const WL_SUBSURFACE_VERSION: u32 = 1;

pub mod subsurface_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const SET_POSITION: u16 = 1;
    pub const SET_POSITION_SIG: &[ArgType] = &[ArgType::Int, ArgType::Int]; // x, y

    pub const PLACE_ABOVE: u16 = 2;
    pub const PLACE_ABOVE_SIG: &[ArgType] = &[ArgType::Object]; // sibling

    pub const PLACE_BELOW: u16 = 3;
    pub const PLACE_BELOW_SIG: &[ArgType] = &[ArgType::Object]; // sibling

    pub const SET_SYNC: u16 = 4;
    pub const SET_SYNC_SIG: &[ArgType] = &[];

    pub const SET_DESYNC: u16 = 5;
    pub const SET_DESYNC_SIG: &[ArgType] = &[];
}

/// Build empty event (subsurface has no events).
pub fn subsurface_noop_event(subsurface_id: u32) -> Message {
    message_empty(subsurface_id, 0)
}
