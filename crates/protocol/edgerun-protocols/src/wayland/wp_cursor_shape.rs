//! wp_cursor_shape_v1 — cursor shape management protocol.
//!
//! Allows clients to request named cursor shapes.

use crate::wayland::ArgType;

pub const WP_CURSOR_SHAPE_MANAGER_V1: &str = "wp_cursor_shape_manager_v1";
pub const WP_CURSOR_SHAPE_MANAGER_V1_VERSION: u32 = 1;

pub mod cursor_shape_manager_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const GET_POINTER_SHAPE: u16 = 1;
    pub const GET_POINTER_SHAPE_SIG: &[ArgType] = &[ArgType::NewId, ArgType::Uint];
    // wp_cursor_shape_device_v1, serial
}

// ─── wp_cursor_shape_device_v1 ───────────────────────────────

pub const WP_CURSOR_SHAPE_DEVICE_V1: &str = "wp_cursor_shape_device_v1";
pub const WP_CURSOR_SHAPE_DEVICE_V1_VERSION: u32 = 1;

/// Standard cursor shapes.
pub mod shape {
    pub const DEFAULT: u32 = 1;
    pub const CONTEXT_MENU: u32 = 2;
    pub const HELP: u32 = 3;
    pub const POINTER: u32 = 4;
    pub const PROGRESS: u32 = 5;
    pub const WAIT: u32 = 6;
    pub const CELL: u32 = 7;
    pub const CROSSHAIR: u32 = 8;
    pub const TEXT: u32 = 9;
    pub const VERTICAL_TEXT: u32 = 10;
    pub const ALIAS: u32 = 11;
    pub const COPY: u32 = 12;
    pub const MOVE: u32 = 13;
    pub const NO_DROP: u32 = 14;
    pub const NOT_ALLOWED: u32 = 15;
    pub const GRAB: u32 = 16;
    pub const GRABBING: u32 = 17;
    pub const E_RESIZE: u32 = 18;
    pub const N_RESIZE: u32 = 19;
    pub const NE_RESIZE: u32 = 20;
    pub const NW_RESIZE: u32 = 21;
    pub const S_RESIZE: u32 = 22;
    pub const SE_RESIZE: u32 = 23;
    pub const SW_RESIZE: u32 = 24;
    pub const W_RESIZE: u32 = 25;
    pub const EW_RESIZE: u32 = 26;
    pub const NS_RESIZE: u32 = 27;
    pub const NESW_RESIZE: u32 = 28;
    pub const NWSE_RESIZE: u32 = 29;
    pub const ZOOM_IN: u32 = 30;
    pub const ZOOM_OUT: u32 = 31;
}

pub mod cursor_shape_device_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const SET_SHAPE: u16 = 1;
    pub const SET_SHAPE_SIG: &[ArgType] = &[ArgType::Uint, ArgType::Uint]; // serial, shape
}
