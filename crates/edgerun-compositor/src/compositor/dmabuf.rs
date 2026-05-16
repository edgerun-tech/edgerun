//! DMA-BUF buffer management.
//!
//! Handles buffers received via zwp_linux_dmabuf_v1 protocol.

use std::os::fd::RawFd;

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
        Self::default()
    }

    /// Add a plane with its fd.
    pub fn add_plane(
        &mut self,
        fd: RawFd,
        plane_idx: u32,
        offset: u32,
        stride: u32,
        modifier_hi: u32,
        modifier_lo: u32,
    ) {
        let _ = plane_idx; // used for validation in future implementation
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
