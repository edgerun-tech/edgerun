//! wl_shm, wl_shm_pool, wl_buffer.

use crate::wire::{ArgType, Message};
use crate::wire::encode::*;
use shm_event::*;
use buffer_event::*;

// ─── wl_shm ──────────────────────────────────────────────────

pub const WL_SHM: &str = "wl_shm";
pub const WL_SHM_VERSION: u32 = 1;

/// SHM pixel formats.
pub mod format {
    pub const ARGB8888: u32 = 0;
    pub const XRGB8888: u32 = 1;
    pub const C8: u32 = 0x20203843;
    pub const RGB332: u32 = 0x38424752;
    pub const BGR233: u32 = 0x38524742;
    pub const XRGB4444: u32 = 0x32315258;
    pub const XBGR4444: u32 = 0x32314258;
    pub const RGBX4444: u32 = 0x32315852;
    pub const BGRX4444: u32 = 0x32315842;
    pub const ARGB4444: u32 = 0x32315241;
    pub const ABGR4444: u32 = 0x32314241;
    pub const RGBA4444: u32 = 0x32314152;
    pub const BGRA4444: u32 = 0x32314142;
    pub const XRGB1555: u32 = 0x35315258;
    pub const XBGR1555: u32 = 0x35314258;
    pub const RGBX5551: u32 = 0x35315852;
    pub const BGRX5551: u32 = 0x35315842;
    pub const ARGB1555: u32 = 0x35315241;
    pub const ABGR1555: u32 = 0x35314241;
    pub const RGBA5551: u32 = 0x35314152;
    pub const BGRA5551: u32 = 0x35314142;
    pub const RGB565: u32 = 0x36314752;
    pub const BGR565: u32 = 0x36314742;
    pub const RGB888: u32 = 0x34324752;
    pub const BGR888: u32 = 0x34324742;
    pub const XBGR8888: u32 = 0x34324258;
    pub const RGBX8888: u32 = 0x34325852;
    pub const BGRX8888: u32 = 0x34325842;
    pub const ABGR8888: u32 = 0x34324241;
    pub const RGBA8888: u32 = 0x34324152;
    pub const BGRA8888: u32 = 0x34324142;
}

pub mod shm_request {
    use super::*;

    /// create_pool — create a shared memory pool.
    pub const CREATE_POOL: u16 = 0;
    pub const CREATE_POOL_SIG: &[ArgType] = &[ArgType::NewId, ArgType::Fd, ArgType::Int];
    // id: new_id, fd: fd, size: int

    pub const RELEASE: u16 = 1;
    pub const RELEASE_SIG: &[ArgType] = &[];
}

pub mod shm_event {
    /// format — advertise a supported shm format.
    pub const FORMAT: u16 = 0;
    // sig: uint (format)
}

/// Create a `format` event.
pub fn shm_format_event(shm_id: u32, format: u32) -> Message {
    message_uint(shm_id, FORMAT, format)
}

// ─── wl_shm_pool ─────────────────────────────────────────────

pub const WL_SHM_POOL: &str = "wl_shm_pool";
pub const WL_SHM_POOL_VERSION: u32 = 1;

pub mod shm_pool_request {
    use super::*;

    /// create_buffer
    pub const CREATE_BUFFER: u16 = 0;
    pub const CREATE_BUFFER_SIG: &[ArgType] = &[ArgType::NewId, ArgType::Int, ArgType::Int, ArgType::Int, ArgType::Int, ArgType::Uint];
    // id: new_id, offset: int, width: int, height: int, stride: int, format: uint

    /// resize
    pub const RESIZE: u16 = 1;
    pub const RESIZE_SIG: &[ArgType] = &[ArgType::Int];

    /// destroy
    pub const DESTROY: u16 = 2;
    pub const DESTROY_SIG: &[ArgType] = &[];
}

// ─── wl_buffer ───────────────────────────────────────────────

pub const WL_BUFFER: &str = "wl_buffer";
pub const WL_BUFFER_VERSION: u32 = 1;

pub mod buffer_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];
}

pub mod buffer_event {
    /// release — compositor no longer using this buffer.
    pub const RELEASE: u16 = 0;
    // sig: none
}

/// Create a `release` event.
pub fn buffer_release_event(buffer_id: u32) -> Message {
    message_empty(buffer_id, RELEASE)
}

// ─── Buffer types for compositor use ─────────────────────────

/// A SHM buffer backed by a memory-mapped file.
#[derive(Debug)]
pub struct ShmBuffer {
    /// The pool this buffer belongs to.
    pub pool_id: u32,
    /// Offset within the pool.
    pub offset: i32,
    /// Buffer width in pixels.
    pub width: i32,
    /// Buffer height in pixels.
    pub height: i32,
    /// Row stride in bytes.
    pub stride: i32,
    /// Pixel format (from [`format`] constants).
    pub shm_format: u32,
    /// Bytes per pixel.
    pub bpp: usize,
}

impl ShmBuffer {
    /// Calculate bytes per pixel from SHM format.
    pub fn bytes_per_pixel(format: u32) -> usize {
        match format {
            format::ARGB8888 | format::XRGB8888
            | format::ABGR8888 | format::XBGR8888
            | format::RGBA8888 | format::RGBX8888
            | format::BGRA8888 | format::BGRX8888 => 4,

            format::XRGB4444 | format::XBGR4444 | format::RGBX4444 | format::BGRX4444
            | format::ARGB4444 | format::ABGR4444 | format::RGBA4444 | format::BGRA4444
            | format::XRGB1555 | format::XBGR1555 | format::RGBX5551 | format::BGRX5551
            | format::ARGB1555 | format::ABGR1555 | format::RGBA5551 | format::BGRA5551
            | format::RGB565 | format::BGR565 => 2,

            format::RGB888 | format::BGR888 => 3,

            format::C8 => 1,

            _ => 4, // default
        }
    }
}
