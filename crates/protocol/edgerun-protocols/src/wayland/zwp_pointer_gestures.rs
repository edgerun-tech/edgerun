//! zwp_pointer_gestures_v1 — touchpad swipe and pinch gesture events.
//!
//! This protocol provides touchpad gesture events (swipe and pinch) which
//! browsers use for navigation (back/forward/zoom gestures).

use alloc::vec::Vec;
use edgerun_encoding::byteorder::push_u32_le;

use crate::wayland::{ArgType, Message};

pub const ZWP_POINTER_GESTURES_V1: &str = "zwp_pointer_gestures_v1";
pub const ZWP_POINTER_GESTURES_V1_VERSION: u32 = 1;

pub mod pointer_gestures_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const GET_SWIPE_GESTURE: u16 = 1;
    pub const GET_SWIPE_GESTURE_SIG: &[ArgType] = &[ArgType::Object, ArgType::NewId]; // wl_pointer, zwp_gesture_swipe_v1

    pub const GET_PINCH_GESTURE: u16 = 2;
    pub const GET_PINCH_GESTURE_SIG: &[ArgType] = &[ArgType::Object, ArgType::NewId];
    // wl_pointer, zwp_gesture_pinch_v1
}

// ─── zwp_gesture_swipe_v1 ────────────────────────────────────

pub const ZWP_GESTURE_SWIPE_V1: &str = "zwp_gesture_swipe_v1";
pub const ZWP_GESTURE_SWIPE_V1_VERSION: u32 = 1;

pub mod gesture_swipe_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];
}

pub mod gesture_swipe_event {
    pub const BEGIN: u16 = 0;
    pub const UPDATE: u16 = 1;
    pub const END: u16 = 2;
}

/// Build a swipe_begin event.
pub fn swipe_begin_event(
    gesture_swipe_id: u32,
    serial: u32,
    time: u32,
    surface_id: u32,
    fingers: u32,
) -> Message {
    use gesture_swipe_event::BEGIN;
    let mut args = Vec::new();
    push_u32_le(&mut args, serial);
    push_u32_le(&mut args, time);
    push_u32_le(&mut args, surface_id);
    push_u32_le(&mut args, fingers);
    Message {
        sender_id: gesture_swipe_id,
        opcode: BEGIN,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build a swipe_update event.
pub fn swipe_update_event(gesture_swipe_id: u32, time: u32, dx: u32, dy: u32) -> Message {
    use gesture_swipe_event::UPDATE;
    let mut args = Vec::new();
    push_u32_le(&mut args, time);
    push_u32_le(&mut args, dx);
    push_u32_le(&mut args, dy);
    Message {
        sender_id: gesture_swipe_id,
        opcode: UPDATE,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build a swipe_end event.
pub fn swipe_end_event(gesture_swipe_id: u32, serial: u32, time: u32, cancelled: u32) -> Message {
    use gesture_swipe_event::END;
    let mut args = Vec::new();
    push_u32_le(&mut args, serial);
    push_u32_le(&mut args, time);
    push_u32_le(&mut args, cancelled);
    Message {
        sender_id: gesture_swipe_id,
        opcode: END,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

// ─── zwp_gesture_pinch_v1 ────────────────────────────────────

pub const ZWP_GESTURE_PINCH_V1: &str = "zwp_gesture_pinch_v1";
pub const ZWP_GESTURE_PINCH_V1_VERSION: u32 = 1;

pub mod gesture_pinch_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];
}

pub mod gesture_pinch_event {
    pub const BEGIN: u16 = 0;
    pub const UPDATE: u16 = 1;
    pub const END: u16 = 2;
}

/// Build a pinch_begin event.
pub fn pinch_begin_event(
    gesture_pinch_id: u32,
    serial: u32,
    time: u32,
    surface_id: u32,
    fingers: u32,
) -> Message {
    use gesture_pinch_event::BEGIN;
    let mut args = Vec::new();
    push_u32_le(&mut args, serial);
    push_u32_le(&mut args, time);
    push_u32_le(&mut args, surface_id);
    push_u32_le(&mut args, fingers);
    Message {
        sender_id: gesture_pinch_id,
        opcode: BEGIN,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build a pinch_update event.
pub fn pinch_update_event(
    gesture_pinch_id: u32,
    time: u32,
    dx: u32,
    dy: u32,
    scale: u32,
    rotation: u32,
) -> Message {
    use gesture_pinch_event::UPDATE;
    let mut args = Vec::new();
    push_u32_le(&mut args, time);
    push_u32_le(&mut args, dx);
    push_u32_le(&mut args, dy);
    push_u32_le(&mut args, scale);
    push_u32_le(&mut args, rotation);
    Message {
        sender_id: gesture_pinch_id,
        opcode: UPDATE,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build a pinch_end event.
pub fn pinch_end_event(gesture_pinch_id: u32, serial: u32, time: u32, cancelled: u32) -> Message {
    use gesture_pinch_event::END;
    let mut args = Vec::new();
    push_u32_le(&mut args, serial);
    push_u32_le(&mut args, time);
    push_u32_le(&mut args, cancelled);
    Message {
        sender_id: gesture_pinch_id,
        opcode: END,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}
