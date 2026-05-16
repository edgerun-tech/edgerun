//! Wayland wire protocol types.

use alloc::vec::Vec;

pub mod data_control;
pub mod decode;
pub mod encode;
pub mod fractional_scale;
pub mod input_method_v2;
pub mod layer_shell;
pub mod linux_dmabuf;
pub mod linux_drm_syncobj;
pub mod primary_selection;
pub mod screencopy;
pub mod single_pixel_buffer;
pub mod tearing_control;
pub mod text_input_v3;
pub mod wl_compositor;
pub mod wl_core;
pub mod wl_data_device;
pub mod wl_output;
pub mod wl_seat;
pub mod wl_shm;
pub mod wl_subcompositor;
pub mod wp_cursor_shape;
pub mod wp_presentation_time;
pub mod wp_viewporter;
pub mod xdg_activation;
pub mod xdg_decoration;
pub mod xdg_foreign;
pub mod xdg_output;
pub mod xdg_shell;
pub mod zwp_pointer_constraints;
pub mod zwp_pointer_gestures;
pub mod zwp_relative_pointer;
pub mod zwp_text_input;
pub mod zxdg_idle_inhibit;

pub use decode::*;
pub use encode::*;

/// Well-known object IDs.
pub const WL_DISPLAY_ID: u32 = 1;

/// Maximum message size we'll accept (protects against OOM).
pub const MAX_MESSAGE_SIZE: usize = 64 * 1024;

/// Wire argument type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgType {
    Int,
    Uint,
    Fixed,
    String,
    Object,
    NewId,
    Array,
    Fd,
}

/// A single protocol argument descriptor.
#[derive(Debug, Clone)]
pub struct ArgSpec {
    pub name: &'static str,
    pub ty: ArgType,
    pub nullable: bool,
    pub summary: &'static str,
}

/// A decoded Wayland message.
#[derive(Debug)]
pub struct Message {
    pub sender_id: u32,
    pub opcode: u16,
    pub size: u16,
    pub args: Vec<u8>,
    pub fds: Vec<i32>,
}

/// Align `n` up to the nearest multiple of 4.
#[inline]
pub fn align4(n: usize) -> usize {
    (n + 3) & !3
}
