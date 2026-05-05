//! zwlr_layer_shell_v1 — layer shell protocol.
//!
//! Used by Waybar, wofi, swaylock, notification daemons, and other
//! desktop shell components to create surfaces on specific layers
//! (background, bottom, top, overlay) with anchoring and exclusive zones.
//!
//! Protocol version: 4

use crate::wire::encode::*;
use crate::wire::{ArgType, Message};

pub const ZWLR_LAYER_SHELL_V1: &str = "zwlr_layer_shell_v1";
pub const ZWLR_LAYER_SHELL_V1_VERSION: u32 = 4;

/// Layer enum values.
pub mod layer {
    pub const BACKGROUND: u32 = 0;
    pub const BOTTOM: u32 = 1;
    pub const TOP: u32 = 2;
    pub const OVERLAY: u32 = 3;
}

pub mod layer_shell_request {
    use super::*;

    pub const GET_LAYER_SURFACE: u16 = 0;
    pub const GET_LAYER_SURFACE_SIG: &[ArgType] = &[
        ArgType::NewId,  // id: zwlr_layer_surface_v1
        ArgType::Object, // surface: wl_surface
        ArgType::Object, // output: wl_output (nullable)
        ArgType::Uint,   // layer: enum layer
        ArgType::String, // namespace
    ];

    pub const DESTROY: u16 = 1;
    pub const DESTROY_SIG: &[ArgType] = &[];
}

// ─── zwlr_layer_surface_v1 ──────────────────────────────────

pub const ZWLR_LAYER_SURFACE_V1: &str = "zwlr_layer_surface_v1";
pub const ZWLR_LAYER_SURFACE_V1_VERSION: u32 = 4;

/// Anchor bitfield values.
pub mod anchor {
    pub const TOP: u32 = 1;
    pub const BOTTOM: u32 = 2;
    pub const LEFT: u32 = 4;
    pub const RIGHT: u32 = 8;
}

/// Keyboard interactivity values.
pub mod keyboard_interactivity {
    pub const NONE: u32 = 0;
    pub const EXCLUSIVE: u32 = 1;
    pub const ON_DEMAND: u32 = 2;
}

/// Error codes.
pub mod error {
    pub const ROLE: u32 = 0;
    pub const INVALID_LAYER: u32 = 1;
    pub const INVALID_ANCHOR: u32 = 2;
    pub const INVALID_KEYBOARD_INTERACTIVITY: u32 = 3;
    pub const INVALID_EXCLUSIVE_EDGE: u32 = 4;
}

pub mod layer_surface_request {
    use super::*;

    pub const SET_SIZE: u16 = 0;
    pub const SET_SIZE_SIG: &[ArgType] = &[ArgType::Uint, ArgType::Uint];

    pub const SET_ANCHOR: u16 = 1;
    pub const SET_ANCHOR_SIG: &[ArgType] = &[ArgType::Uint];

    pub const SET_EXCLUSIVE_ZONE: u16 = 2;
    pub const SET_EXCLUSIVE_ZONE_SIG: &[ArgType] = &[ArgType::Int];

    pub const SET_MARGIN: u16 = 3;
    pub const SET_MARGIN_SIG: &[ArgType] =
        &[ArgType::Int, ArgType::Int, ArgType::Int, ArgType::Int];

    pub const SET_KEYBOARD_INTERACTIVITY: u16 = 4;
    pub const SET_KEYBOARD_INTERACTIVITY_SIG: &[ArgType] = &[ArgType::Uint];

    pub const GET_POPUP: u16 = 5;
    pub const GET_POPUP_SIG: &[ArgType] = &[ArgType::Object]; // xdg_popup

    pub const ACK_CONFIGURE: u16 = 6;
    pub const ACK_CONFIGURE_SIG: &[ArgType] = &[ArgType::Uint];

    pub const SET_LAYER: u16 = 7;
    pub const SET_LAYER_SIG: &[ArgType] = &[ArgType::Uint];

    pub const DESTROY: u16 = 8;
    pub const DESTROY_SIG: &[ArgType] = &[];
}

pub mod layer_surface_event {
    use super::*;

    pub const CONFIGURE: u16 = 0;
    pub const CONFIGURE_SIG: &[ArgType] = &[ArgType::Uint, ArgType::Uint, ArgType::Uint];

    pub const CLOSED: u16 = 1;
    pub const CLOSED_SIG: &[ArgType] = &[];
}

/// Build a configure event for a layer surface.
pub fn layer_surface_configure_event(
    layer_surface_id: u32,
    serial: u32,
    width: u32,
    height: u32,
) -> Message {
    use layer_surface_event::CONFIGURE;
    message_uint3(layer_surface_id, CONFIGURE, serial, width, height)
}

/// Build a closed event for a layer surface.
pub fn layer_surface_closed_event(layer_surface_id: u32) -> Message {
    use layer_surface_event::CLOSED;
    message_empty(layer_surface_id, CLOSED)
}
