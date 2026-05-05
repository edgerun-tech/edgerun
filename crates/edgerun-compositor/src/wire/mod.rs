//! Wayland wire protocol types.

pub mod decode;
pub mod encode;
pub mod fd;

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
