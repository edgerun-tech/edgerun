//! xdg-shell protocol (xdg_wm_base, xdg_surface, xdg_toplevel, xdg_positioner, xdg_popup).
//!
//! This is the stable version of xdg-shell (version 6).

use crate::wire::{ArgType, Message};
use crate::wire::encode::*;

pub const XDG_WM_BASE: &str = "xdg_wm_base";
pub const XDG_WM_BASE_VERSION: u32 = 6;

// ─── xdg_wm_base ─────────────────────────────────────────────

pub mod xdg_wm_base_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    /// create_positioner
    pub const CREATE_POSITIONER: u16 = 1;
    pub const CREATE_POSITIONER_SIG: &[ArgType] = &[ArgType::NewId];

    /// get_xdg_surface
    pub const GET_XDG_SURFACE: u16 = 2;
    pub const GET_XDG_SURFACE_SIG: &[ArgType] = &[ArgType::NewId, ArgType::Object];
    // id: new_id, surface: object

    /// pong — response to server's ping.
    pub const PONG: u16 = 3;
    pub const PONG_SIG: &[ArgType] = &[ArgType::Uint]; // serial
}

pub mod xdg_wm_base_event {
    /// ping — check if client is alive.
    pub const PING: u16 = 0;
    // sig: uint (serial)
}

/// Build a ping event.
pub fn xdg_wm_base_ping_event(base_id: u32, serial: u32) -> Message {
    message_uint(base_id, xdg_wm_base_event::PING, serial)
}

// ─── xdg_surface ─────────────────────────────────────────────

pub const XDG_SURFACE: &str = "xdg_surface";
pub const XDG_SURFACE_VERSION: u32 = 6;

pub mod xdg_surface_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    /// get_toplevel
    pub const GET_TOPLEVEL: u16 = 1;
    pub const GET_TOPLEVEL_SIG: &[ArgType] = &[ArgType::NewId];

    /// get_popup
    pub const GET_POPUP: u16 = 2;
    pub const GET_POPUP_SIG: &[ArgType] = &[ArgType::NewId, ArgType::Object, ArgType::Object];
    // id: new_id, parent: object(nullable), positioner: object

    /// set_window_geometry
    pub const SET_WINDOW_GEOMETRY: u16 = 3;
    pub const SET_WINDOW_GEOMETRY_SIG: &[ArgType] = &[ArgType::Int, ArgType::Int, ArgType::Int, ArgType::Int];
    // x, y, width, height

    /// ack_configure — acknowledge a configure event.
    pub const ACK_CONFIGURE: u16 = 4;
    pub const ACK_CONFIGURE_SIG: &[ArgType] = &[ArgType::Uint]; // serial
}

pub mod xdg_surface_event {
    /// configure — suggested surface state change.
    pub const CONFIGURE: u16 = 0;
    // sig: uint (serial)
}

/// Build a configure event.
pub fn xdg_surface_configure_event(surface_id: u32, serial: u32) -> Message {
    message_uint(surface_id, xdg_surface_event::CONFIGURE, serial)
}

// ─── xdg_toplevel ────────────────────────────────────────────

pub const XDG_TOPLEVEL: &str = "xdg_toplevel";
pub const XDG_TOPLEVEL_VERSION: u32 = 6;

/// Toplevel state flags.
pub mod toplevel_state {
    pub const MAXIMIZED: u32 = 1;
    pub const FULLSCREEN: u32 = 2;
    pub const RESIZING: u32 = 3;
    pub const ACTIVATED: u32 = 4;
    pub const TILE_LEFT: u32 = 5;
    pub const TILE_RIGHT: u32 = 6;
    pub const TILE_TOP: u32 = 7;
    pub const TILE_BOTTOM: u32 = 8;
    pub const SUSPENDED: u32 = 9;
}

/// Toplevel edges for resize.
pub mod toplevel_edge {
    pub const NONE: u32 = 0;
    pub const TOP: u32 = 1;
    pub const BOTTOM: u32 = 2;
    pub const LEFT: u32 = 4;
    pub const TOP_LEFT: u32 = 5;
    pub const BOTTOM_LEFT: u32 = 6;
    pub const RIGHT: u32 = 8;
    pub const TOP_RIGHT: u32 = 9;
    pub const BOTTOM_RIGHT: u32 = 10;
}

pub mod xdg_toplevel_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const SET_PARENT: u16 = 1;
    pub const SET_PARENT_SIG: &[ArgType] = &[ArgType::Object]; // nullable

    pub const SET_TITLE: u16 = 2;
    pub const SET_TITLE_SIG: &[ArgType] = &[ArgType::String];

    pub const SET_APP_ID: u16 = 3;
    pub const SET_APP_ID_SIG: &[ArgType] = &[ArgType::String];

    pub const SHOW_WINDOW_MENU: u16 = 4;
    pub const SHOW_WINDOW_MENU_SIG: &[ArgType] = &[ArgType::Object, ArgType::Uint, ArgType::Int, ArgType::Int];

    pub const MOVE: u16 = 5;
    pub const MOVE_SIG: &[ArgType] = &[ArgType::Object, ArgType::Uint];
    // seat: object, serial: uint

    pub const RESIZE: u16 = 6;
    pub const RESIZE_SIG: &[ArgType] = &[ArgType::Object, ArgType::Uint, ArgType::Uint];
    // seat: object, serial: uint, edges: uint

    pub const SET_MAX_SIZE: u16 = 7;
    pub const SET_MAX_SIZE_SIG: &[ArgType] = &[ArgType::Int, ArgType::Int];

    pub const SET_MIN_SIZE: u16 = 8;
    pub const SET_MIN_SIZE_SIG: &[ArgType] = &[ArgType::Int, ArgType::Int];

    pub const MAXIMIZE: u16 = 9;
    pub const MAXIMIZE_SIG: &[ArgType] = &[];

    pub const UNMAXIMIZE: u16 = 10;
    pub const UNMAXIMIZE_SIG: &[ArgType] = &[];

    pub const MINIMIZE: u16 = 11;
    pub const MINIMIZE_SIG: &[ArgType] = &[];

    // --- version 4 ---
    pub const SET_FULLSCREEN: u16 = 12;
    pub const SET_FULLSCREEN_SIG: &[ArgType] = &[ArgType::Object]; // output(nullable)

    pub const UNSET_FULLSCREEN: u16 = 13;
    pub const UNSET_FULLSCREEN_SIG: &[ArgType] = &[];

    // --- version 5 ---
    pub const SET_MAXIMIZED: u16 = 14;
    pub const SET_MAXIMIZED_SIG: &[ArgType] = &[];

    pub const UNSET_MAXIMIZED: u16 = 15;
    pub const UNSET_MAXIMIZED_SIG: &[ArgType] = &[];

    // --- version 6 ---
    pub const SET_MINIMIZED: u16 = 16; // renamed from MINIMIZE
    pub const SET_MINIMIZED_SIG: &[ArgType] = &[];
}

pub mod xdg_toplevel_event {
    /// configure — suggest new size/state.
    pub const CONFIGURE: u16 = 0;
    // sig: int(width), int(height), array(states)

    /// close — server requests toplevel close.
    pub const CLOSE: u16 = 1;
    // sig: none

    /// configure_bounds — configure bounds (v4+).
    pub const CONFIGURE_BOUNDS: u16 = 2;
    // sig: int(width), int(height)

    /// wm_capabilities (v5+).
    pub const WM_CAPABILITIES: u16 = 3;
    // sig: array(capabilities)
}

/// Build a toplevel configure event.
pub fn xdg_toplevel_configure_event(
    toplevel_id: u32,
    width: i32,
    height: i32,
    states: &[u8], // array of u32 state flags
) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&width.to_le_bytes());
    args.extend_from_slice(&height.to_le_bytes());
    crate::wire::encode::encode_array(&mut args, states);
    Message {
        sender_id: toplevel_id,
        opcode: xdg_toplevel_event::CONFIGURE,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build a close event.
pub fn xdg_toplevel_close_event(toplevel_id: u32) -> Message {
    message_empty(toplevel_id, xdg_toplevel_event::CLOSE)
}

/// Build a wm_capabilities event (v5+).
/// capabilities is an array of u32 capability flags.
pub fn xdg_toplevel_wm_capabilities_event(toplevel_id: u32, capabilities: &[u32]) -> Message {
    let mut args = Vec::new();
    // Serialize as array of u32
    let bytes: Vec<u8> = capabilities.iter().flat_map(|c| c.to_le_bytes()).collect();
    crate::wire::encode::encode_array(&mut args, &bytes);
    Message {
        sender_id: toplevel_id,
        opcode: xdg_toplevel_event::WM_CAPABILITIES,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

// ─── xdg_positioner ──────────────────────────────────────────

pub const XDG_POSITIONER: &str = "xdg_positioner";
pub const XDG_POSITIONER_VERSION: u32 = 6;

pub mod xdg_positioner_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const SET_SIZE: u16 = 1;
    pub const SET_SIZE_SIG: &[ArgType] = &[ArgType::Int, ArgType::Int];

    pub const SET_ANCHOR_RECT: u16 = 2;
    pub const SET_ANCHOR_RECT_SIG: &[ArgType] = &[ArgType::Int, ArgType::Int, ArgType::Int, ArgType::Int];

    pub const SET_ANCHOR: u16 = 3;
    pub const SET_ANCHOR_SIG: &[ArgType] = &[ArgType::Uint];

    pub const SET_GRAVITY: u16 = 4;
    pub const SET_GRAVITY_SIG: &[ArgType] = &[ArgType::Uint];

    pub const SET_CONSTRAINT_ADJUSTMENT: u16 = 5;
    pub const SET_CONSTRAINT_ADJUSTMENT_SIG: &[ArgType] = &[ArgType::Uint];

    pub const SET_OFFSET: u16 = 6;
    pub const SET_OFFSET_SIG: &[ArgType] = &[ArgType::Int, ArgType::Int];

    pub const SET_REACTIVE: u16 = 7;
    pub const SET_REACTIVE_SIG: &[ArgType] = &[];

    pub const SET_PARENT_SIZE: u16 = 8;
    pub const SET_PARENT_SIZE_SIG: &[ArgType] = &[ArgType::Int, ArgType::Int];

    pub const SET_PARENT_CONFIGURE: u16 = 9;
    pub const SET_PARENT_CONFIGURE_SIG: &[ArgType] = &[ArgType::Uint];
}

// ─── xdg_popup ───────────────────────────────────────────────

pub const XDG_POPUP: &str = "xdg_popup";
pub const XDG_POPUP_VERSION: u32 = 6;

pub mod xdg_popup_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const GRAB: u16 = 1;
    pub const GRAB_SIG: &[ArgType] = &[ArgType::Object, ArgType::Uint];
    // seat: object, serial: uint

    pub const REPOSITION: u16 = 2;
    pub const REPOSITION_SIG: &[ArgType] = &[ArgType::Object, ArgType::Uint];
    // positioner: object, token: uint
}

pub mod xdg_popup_event {
    pub const CONFIGURE: u16 = 0;
    // sig: int(x), int(y), int(width), int(height)

    pub const POPUP_DONE: u16 = 1;
    // sig: none

    pub const REPOSITIONED: u16 = 2;
    // sig: uint(token)
}

/// Build popup configure event.
pub fn xdg_popup_configure_event(
    popup_id: u32,
    x: i32, y: i32, width: i32, height: i32,
) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&x.to_le_bytes());
    args.extend_from_slice(&y.to_le_bytes());
    args.extend_from_slice(&width.to_le_bytes());
    args.extend_from_slice(&height.to_le_bytes());
    Message {
        sender_id: popup_id,
        opcode: xdg_popup_event::CONFIGURE,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build popup done event.
pub fn xdg_popup_done_event(popup_id: u32) -> Message {
    message_empty(popup_id, xdg_popup_event::POPUP_DONE)
}
