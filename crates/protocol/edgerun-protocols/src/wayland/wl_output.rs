//! wl_output interface.

use alloc::vec::Vec;
use edgerun_encoding::byteorder::{push_i32_le, push_u32_le};

use crate::wayland::encode::*;
use crate::wayland::{ArgType, Message};
use output_event::*;

pub const WL_OUTPUT: &str = "wl_output";
pub const WL_OUTPUT_VERSION: u32 = 4;

/// Output subpixel geometry.
pub mod subpixel {
    pub const UNKNOWN: i32 = 0;
    pub const NONE: i32 = 1;
    pub const HORIZONTAL_RGB: i32 = 2;
    pub const HORIZONTAL_BGR: i32 = 3;
    pub const VERTICAL_RGB: i32 = 4;
    pub const VERTICAL_BGR: i32 = 5;
}

/// Output transform.
pub mod transform {
    pub const NORMAL: i32 = 0;
    pub const _90: i32 = 1;
    pub const _180: i32 = 2;
    pub const _270: i32 = 3;
    pub const FLIPPED: i32 = 4;
    pub const FLIPPED_90: i32 = 5;
    pub const FLIPPED_180: i32 = 6;
    pub const FLIPPED_270: i32 = 7;
}

/// Output mode flags.
pub mod mode {
    pub const CURRENT: u32 = 0x1;
    pub const PREFERRED: u32 = 0x2;
}

pub mod output_request {
    use super::*;

    pub const RELEASE: u16 = 0;
    pub const RELEASE_SIG: &[ArgType] = &[];
}

pub mod output_event {
    /// geometry — physical properties of the output.
    pub const GEOMETRY: u16 = 0;
    // sig: int(x), int(y), int(physical_width), int(physical_height),
    //      int(subpixel), string(make), string(model), int(transform)

    /// mode — mode information.
    pub const MODE: u16 = 1;
    // sig: uint(flags), int(width), int(height), int(refresh)

    /// done — sent after geometry + mode + scale events.
    pub const DONE: u16 = 2;
    // sig: none

    /// scale — output scaling factor.
    pub const SCALE: u16 = 3;
    // sig: int

    /// name — unique output name.
    pub const NAME: u16 = 4;
    // sig: string

    /// description — human-readable description.
    pub const DESCRIPTION: u16 = 5;
    // sig: string
}

/// Build a geometry event.
pub fn output_geometry_event(
    output_id: u32,
    x: i32,
    y: i32,
    physical_w: i32,
    physical_h: i32,
    subpixel: i32,
    make: &str,
    model: &str,
    transform: i32,
) -> Message {
    // int(x) + int(y) + int(phys_w) + int(phys_h) + int(subpixel) + string(make) + string(model) + int(transform)
    // = 4*5 + string_len1 + string_len2 + 4
    let mut args = Vec::new();
    push_i32_le(&mut args, x);
    push_i32_le(&mut args, y);
    push_i32_le(&mut args, physical_w);
    push_i32_le(&mut args, physical_h);
    push_i32_le(&mut args, subpixel);
    crate::wayland::encode::encode_string(&mut args, make);
    crate::wayland::encode::encode_string(&mut args, model);
    push_i32_le(&mut args, transform);
    Message {
        sender_id: output_id,
        opcode: GEOMETRY,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build a mode event.
pub fn output_mode_event(
    output_id: u32,
    flags: u32,
    width: i32,
    height: i32,
    refresh: i32,
) -> Message {
    let mut args = Vec::new();
    push_u32_le(&mut args, flags);
    push_i32_le(&mut args, width);
    push_i32_le(&mut args, height);
    push_i32_le(&mut args, refresh);
    Message {
        sender_id: output_id,
        opcode: MODE,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build a done event.
pub fn output_done_event(output_id: u32) -> Message {
    message_empty(output_id, DONE)
}

/// Build a scale event.
pub fn output_scale_event(output_id: u32, factor: i32) -> Message {
    message_uint(output_id, SCALE, factor as u32)
}

/// Build a name event.
pub fn output_name_event(output_id: u32, name: &str) -> Message {
    let mut args = Vec::new();
    crate::wayland::encode::encode_string(&mut args, name);
    Message {
        sender_id: output_id,
        opcode: NAME,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build a description event.
pub fn output_description_event(output_id: u32, desc: &str) -> Message {
    let mut args = Vec::new();
    crate::wayland::encode::encode_string(&mut args, desc);
    Message {
        sender_id: output_id,
        opcode: DESCRIPTION,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}
