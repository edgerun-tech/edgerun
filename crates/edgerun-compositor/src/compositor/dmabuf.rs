//! DMA-BUF buffer management.
//!
//! Handles buffers received via zwp_linux_dmabuf_v1 protocol.

use std::collections::HashMap;
use std::os::fd::RawFd;

/// A DMA-BUF buffer.
#[derive(Debug)]
pub struct DmabufBuffer {
    /// The wl_buffer object id.
    pub buffer_id: u32,
    /// Width in pixels.
    pub width: i32,
    /// Height in pixels.
    pub height: i32,
    /// DRM format (e.g., XRGB8888).
    pub format: u32,
    /// Modifier (hi, lo parts combined).
    pub modifier: u64,
    /// Number of planes.
    pub num_planes: u32,
    /// Plane information.
    pub planes: Vec<DmabufPlane>,
    /// The raw fd(s) — kept alive until buffer is destroyed.
    pub fds: Vec<RawFd>,
}

/// A single plane of a DMA-BUF buffer.
#[derive(Debug, Clone)]
pub struct DmabufPlane {
    /// File descriptor for this plane.
    pub fd: RawFd,
    /// Offset into the fd.
    pub offset: u32,
    /// Stride in bytes.
    pub stride: u32,
    /// DRM modifier for this plane.
    pub modifier: u64,
}

/// DMA-BUF buffer registry.
#[derive(Debug, Default)]
pub struct DmabufRegistry {
    buffers: HashMap<u32, DmabufBuffer>,
}

impl DmabufRegistry {
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
        }
    }

    /// Register a new DMA-BUF buffer.
    pub fn insert(&mut self, buffer_id: u32, buffer: DmabufBuffer) {
        self.buffers.insert(buffer_id, buffer);
    }

    /// Get a DMA-BUF buffer.
    pub fn get(&self, buffer_id: u32) -> Option<&DmabufBuffer> {
        self.buffers.get(&buffer_id)
    }

    /// Remove and close all fds for a buffer.
    pub fn remove(&mut self, buffer_id: u32) {
        if let Some(buf) = self.buffers.remove(&buffer_id) {
            for fd in buf.fds {
                if fd >= 0 {
                    unsafe { libc::close(fd) };
                }
            }
        }
    }
}

/// Pending DMA-BUF parameters (accumulated from CREATE_PARAMS requests).
#[derive(Debug, Default)]
pub struct DmabufParams {
    pub width: i32,
    pub height: i32,
    pub format: u32,
    pub flags: u32,
    pub planes: Vec<DmabufPlane>,
    /// Number of fds accumulated.
    pub fd_count: u32,
}

impl DmabufParams {
    pub fn new() -> Self {
        Self {
            width: 0,
            height: 0,
            format: 0,
            flags: 0,
            planes: Vec::new(),
            fd_count: 0,
        }
    }

    /// Add a plane with its fd.
    pub fn add_plane(&mut self, fd: RawFd, plane_idx: u32, offset: u32, stride: u32, modifier_hi: u32, modifier_lo: u32) {
        let modifier = ((modifier_hi as u64) << 32) | (modifier_lo as u64);
        self.planes.push(DmabufPlane {
            fd,
            offset,
            stride,
            modifier,
        });
        self.fd_count += 1;
    }
}
