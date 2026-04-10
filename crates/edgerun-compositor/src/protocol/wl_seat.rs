//! wl_seat, wl_keyboard, wl_pointer, wl_touch.

use crate::wire::{ArgType, Message};
use crate::wire::encode::*;
use seat_event::*;

pub const WL_SEAT: &str = "wl_seat";
pub const WL_SEAT_VERSION: u32 = 7;

/// Seat capability flags.
pub mod capability {
    pub const POINTER: u32 = 1;
    pub const KEYBOARD: u32 = 2;
    pub const TOUCH: u32 = 4;
}

pub mod seat_request {
    use super::*;

    pub const GET_KEYBOARD: u16 = 0;
    pub const GET_KEYBOARD_SIG: &[ArgType] = &[ArgType::NewId];

    pub const GET_POINTER: u16 = 1;
    pub const GET_POINTER_SIG: &[ArgType] = &[ArgType::NewId];

    pub const GET_TOUCH: u16 = 2;
    pub const GET_TOUCH_SIG: &[ArgType] = &[ArgType::NewId];

    pub const RELEASE: u16 = 3;
    pub const RELEASE_SIG: &[ArgType] = &[];
}

pub mod seat_event {
    /// capabilities — seat gained or lost capabilities.
    pub const CAPABILITIES: u16 = 0;
    // sig: uint (capabilities bitmask)

    /// name — seat identifier.
    pub const NAME: u16 = 1;
    // sig: string
}

/// Build capabilities event.
pub fn seat_capabilities_event(seat_id: u32, caps: u32) -> Message {
    message_uint(seat_id, CAPABILITIES, caps)
}

/// Build name event.
pub fn seat_name_event(seat_id: u32, name: &str) -> Message {
    let mut args = Vec::new();
    crate::wire::encode::encode_string(&mut args, name);
    Message {
        sender_id: seat_id,
        opcode: seat_event::NAME,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

// ─── wl_keyboard ─────────────────────────────────────────────

pub const WL_KEYBOARD: &str = "wl_keyboard";
pub const WL_KEYBOARD_VERSION: u32 = 7;

/// Keymap format.
pub mod keymap_format {
    /// No keymap — client must interpret key events itself.
    pub const NO_KEYMAP: u32 = 0;
    /// libxkbcommon-compatible text format (XKB keymap).
    pub const XKB_V1: u32 = 1;
}

/// Key state.
pub mod key_state {
    pub const RELEASED: u32 = 0;
    pub const PRESSED: u32 = 1;
}

pub mod keyboard_request {
    use super::*;

    pub const RELEASE: u16 = 0;
    pub const RELEASE_SIG: &[ArgType] = &[];
}

pub mod keyboard_event {
    /// keymap — keyboard layout description.
    pub const KEYMAP: u16 = 0;
    // sig: uint(format), fd, uint(size)

    /// enter — keyboard focus entered a surface.
    pub const ENTER: u16 = 1;
    // sig: uint(serial), object(surface), array(keys)

    /// leave — keyboard focus left a surface.
    pub const LEAVE: u16 = 2;
    // sig: uint(serial), object(surface)

    /// key — a key was pressed or released.
    pub const KEY: u16 = 3;
    // sig: uint(serial), uint(time), uint(key), uint(state)

    /// modifiers — modifier state changed.
    pub const MODIFIERS: u16 = 4;
    // sig: uint(serial), uint(mods_depressed), uint(mods_latched), uint(mods_locked), uint(group)

    /// repeat_info — autorepeat rate and delay.
    pub const REPEAT_INFO: u16 = 5;
    // sig: int(rate), int(delay)
}

/// Build keymap event.
pub fn keyboard_keymap_event(kb_id: u32, format: u32, fd: i32, size: u32) -> Message {
    // format: uint, fd: 4 bytes placeholder, size: uint
    let mut args = Vec::new();
    args.extend_from_slice(&format.to_le_bytes());
    args.extend_from_slice(&[0u8; 4]); // fd placeholder
    args.extend_from_slice(&size.to_le_bytes());
    Message {
        sender_id: kb_id,
        opcode: keyboard_event::KEYMAP,
        size: (8 + args.len()) as u16,
        args,
        fds: vec![fd],
    }
}

/// Build enter event.
pub fn keyboard_enter_event(
    kb_id: u32,
    serial: u32,
    surface_id: u32,
    keys: &[u8], // raw key state array (8 bytes per key: u32 key + u32 state)
) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&serial.to_le_bytes());
    args.extend_from_slice(&surface_id.to_le_bytes());
    crate::wire::encode::encode_array(&mut args, keys);
    Message {
        sender_id: kb_id,
        opcode: keyboard_event::ENTER,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build leave event.
pub fn keyboard_leave_event(kb_id: u32, serial: u32, surface_id: u32) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&serial.to_le_bytes());
    args.extend_from_slice(&surface_id.to_le_bytes());
    Message {
        sender_id: kb_id,
        opcode: keyboard_event::LEAVE,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build key event.
pub fn keyboard_key_event(
    kb_id: u32,
    serial: u32,
    time: u32,
    key: u32,
    state: u32,
) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&serial.to_le_bytes());
    args.extend_from_slice(&time.to_le_bytes());
    args.extend_from_slice(&key.to_le_bytes());
    args.extend_from_slice(&state.to_le_bytes());
    Message {
        sender_id: kb_id,
        opcode: keyboard_event::KEY,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build modifiers event.
pub fn keyboard_modifiers_event(
    kb_id: u32,
    serial: u32,
    mods_depressed: u32,
    mods_latched: u32,
    mods_locked: u32,
    group: u32,
) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&serial.to_le_bytes());
    args.extend_from_slice(&mods_depressed.to_le_bytes());
    args.extend_from_slice(&mods_latched.to_le_bytes());
    args.extend_from_slice(&mods_locked.to_le_bytes());
    args.extend_from_slice(&group.to_le_bytes());
    Message {
        sender_id: kb_id,
        opcode: keyboard_event::MODIFIERS,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build repeat info event.
pub fn keyboard_repeat_info_event(kb_id: u32, rate: i32, delay: i32) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&rate.to_le_bytes());
    args.extend_from_slice(&delay.to_le_bytes());
    Message {
        sender_id: kb_id,
        opcode: keyboard_event::REPEAT_INFO,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

// ─── wl_pointer ──────────────────────────────────────────────

pub const WL_POINTER: &str = "wl_pointer";
pub const WL_POINTER_VERSION: u32 = 7;

/// Pointer axis source.
pub mod axis_source {
    pub const WHEEL: u32 = 0;
    pub const FINGER: u32 = 1;
    pub const CONTINUOUS: u32 = 2;
    pub const WHEEL_TILT: u32 = 3;
}

/// Pointer button state.
pub mod button_state {
    pub const RELEASED: u32 = 0;
    pub const PRESSED: u32 = 1;
}

pub mod pointer_request {
    use super::*;

    pub const SET_CURSOR: u16 = 0;
    pub const SET_CURSOR_SIG: &[ArgType] = &[ArgType::Uint, ArgType::Object, ArgType::Int, ArgType::Int];
    // serial: uint, surface: object(nullable), hotspot_x: int, hotspot_y: int

    pub const RELEASE: u16 = 1;
    pub const RELEASE_SIG: &[ArgType] = &[];
}

pub mod pointer_event {
    pub const ENTER: u16 = 0;
    // sig: uint(serial), object(surface), fixed(surface_x), fixed(surface_y)

    pub const LEAVE: u16 = 1;
    // sig: uint(serial), object(surface)

    pub const MOTION: u16 = 2;
    // sig: uint(time), fixed(surface_x), fixed(surface_y)

    pub const BUTTON: u16 = 3;
    // sig: uint(serial), uint(time), uint(button), uint(state)

    pub const AXIS: u16 = 4;
    // sig: uint(time), uint(axis), fixed(value)

    pub const FRAME: u16 = 5;
    // sig: none

    pub const AXIS_SOURCE: u16 = 6;
    // sig: uint(axis_source)

    pub const AXIS_STOP: u16 = 7;
    // sig: uint(time), uint(axis)

    pub const AXIS_DISCRETE: u16 = 8;
    // sig: uint(axis), int(discrete)
}

/// Build pointer enter event.
pub fn pointer_enter_event(
    ptr_id: u32,
    serial: u32,
    surface_id: u32,
    surface_x: f64,
    surface_y: f64,
) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&serial.to_le_bytes());
    args.extend_from_slice(&surface_id.to_le_bytes());
    // Fixed point: s15.16 — multiply by 65536 and round
    let fx = (surface_x * 256.0 * 256.0).round() as i32;
    let fy = (surface_y * 256.0 * 256.0).round() as i32;
    args.extend_from_slice(&fx.to_le_bytes());
    args.extend_from_slice(&fy.to_le_bytes());
    Message {
        sender_id: ptr_id,
        opcode: pointer_event::ENTER,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build pointer leave event.
pub fn pointer_leave_event(ptr_id: u32, serial: u32, surface_id: u32) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&serial.to_le_bytes());
    args.extend_from_slice(&surface_id.to_le_bytes());
    Message {
        sender_id: ptr_id,
        opcode: keyboard_event::LEAVE,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build pointer motion event.
pub fn pointer_motion_event(ptr_id: u32, time: u32, surface_x: f64, surface_y: f64) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&time.to_le_bytes());
    let fx = (surface_x * 256.0 * 256.0).round() as i32;
    let fy = (surface_y * 256.0 * 256.0).round() as i32;
    args.extend_from_slice(&fx.to_le_bytes());
    args.extend_from_slice(&fy.to_le_bytes());
    Message {
        sender_id: ptr_id,
        opcode: pointer_event::MOTION,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build pointer button event.
pub fn pointer_button_event(
    ptr_id: u32,
    serial: u32,
    time: u32,
    button: u32,
    state: u32,
) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&serial.to_le_bytes());
    args.extend_from_slice(&time.to_le_bytes());
    args.extend_from_slice(&button.to_le_bytes());
    args.extend_from_slice(&state.to_le_bytes());
    Message {
        sender_id: ptr_id,
        opcode: pointer_event::BUTTON,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build pointer axis event.
pub fn pointer_axis_event(ptr_id: u32, time: u32, axis: u32, value: f64) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&time.to_le_bytes());
    args.extend_from_slice(&axis.to_le_bytes());
    let fv = (value * 256.0 * 256.0).round() as i32;
    args.extend_from_slice(&fv.to_le_bytes());
    Message {
        sender_id: ptr_id,
        opcode: pointer_event::AXIS,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build pointer frame event.
pub fn pointer_frame_event(ptr_id: u32) -> Message {
    message_empty(ptr_id, pointer_event::FRAME)
}

// ─── wl_touch ────────────────────────────────────────────────

pub const WL_TOUCH: &str = "wl_touch";
pub const WL_TOUCH_VERSION: u32 = 7;

pub mod touch_request {
    use super::*;

    pub const RELEASE: u16 = 0;
    pub const RELEASE_SIG: &[ArgType] = &[];
}

pub mod touch_event {
    pub const DOWN: u16 = 0;
    // sig: uint(serial), uint(time), object(surface), int(id), fixed(x), fixed(y)

    pub const UP: u16 = 1;
    // sig: uint(serial), uint(time), int(id)

    pub const MOTION: u16 = 2;
    // sig: uint(time), int(id), fixed(x), fixed(y)

    pub const SHAPE: u16 = 3;
    // sig: int(id), fixed(major), fixed(minor)

    pub const ORIENTATION: u16 = 4;
    // sig: int(id), fixed(orientation)

    pub const CANCEL: u16 = 5;
    // sig: none

    pub const FRAME: u16 = 6;
    // sig: none
}
