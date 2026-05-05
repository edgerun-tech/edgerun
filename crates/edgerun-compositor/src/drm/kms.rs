//! KMS — Kernel Mode Setting: CRTC control, page flipping.

use std::io;
use std::os::fd::RawFd;
use std::sync::atomic::{AtomicU32, Ordering};

use super::ioctl::*;

/// KMS output state.
#[derive(Debug, Clone)]
pub struct OutputInfo {
    pub connector_id: u32,
    pub crtc_id: u32,
    pub fb_id: u32,
    pub mode: DrmModeModeInfo,
    pub width: u32,
    pub height: u32,
    pub refresh_mhz: u32,
    pub connector_name: String,
}

/// Set a CRTC to a specific mode (legacy mode setting).
pub fn set_crtc(
    fd: RawFd,
    crtc_id: u32,
    fb_id: u32,
    connector_id: u32,
    mode: &DrmModeModeInfo,
) -> io::Result<()> {
    // The kernel uses the same drm_mode_crtc struct for both GET and SET.
    let mut set = DrmModeCrtc {
        set_connectors_ptr: &connector_id as *const u32 as *mut u32,
        count_connectors: 1,
        crtc_id,
        fb_id,
        x: 0,
        y: 0,
        gamma_size: 0,
        mode_valid: 1,
        mode: *mode,
    };

    let ret = unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_SETCRTC, &mut set) };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Queue a page flip. The `user_data` value will be returned in the vblank event.
/// `flags` can include page_flip::PAGE_FLIP_EVENT | page_flip::PAGE_FLIP_ASYNC.
pub fn page_flip(
    fd: RawFd,
    crtc_id: u32,
    fb_id: u32,
    flags: u32,
    user_data: u64,
) -> io::Result<()> {
    let mut flip = DrmModePageFlip {
        crtc_id,
        fb_id,
        flags,
        reserved: 0,
        user_data,
    };

    let ret = unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_PAGE_FLIP, &mut flip) };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Remove a framebuffer.
pub fn rmfb(fd: RawFd, fb_id: u32) -> io::Result<()> {
    let mut id = fb_id;
    let ret = unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_RMFB, &mut id) };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// DRM event types (from drm.h).
pub mod event_type {
    pub const VBLANK: u32 = 0x01;
}

/// Read pending DRM events from the device fd.
///
/// Returns a list of (type, user_data) tuples.
/// Returns empty Vec when no more data available (EAGAIN/WOULDBLOCK).
pub fn read_events(fd: RawFd) -> io::Result<Vec<(u32, u64)>> {
    // DRM events are read as drm_event structs from the fd
    // struct drm_event {
    //     u32 type;
    //     u32 length;
    // };
    // struct drm_event_vblank {
    //     struct drm_event base;
    //     u64 user_data;
    //     u32 tv_sec, tv_usec;
    //     u32 sequence;
    //     u64 crtc_id (if DRM_CAP_CRTC_IN_VBLANK_EVENT)
    // };

    let mut events = Vec::new();

    loop {
        // Read event header (type + length)
        let mut hdr = [0u8; 8];
        if !read_exact_nonblocking(fd, &mut hdr)? {
            break;
        }

        let type_ = u32::from_le_bytes([hdr[0], hdr[1], hdr[2], hdr[3]]);
        let length = u32::from_le_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]) as usize;

        if length < 8 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "DRM event too short",
            ));
        }

        // Read rest of event
        let mut rest = vec![0u8; length - 8];
        if !read_exact_nonblocking(fd, &mut rest)? {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "short DRM event body",
            ));
        }

        match type_ {
            event_type::VBLANK => {
                // drm_event_vblank: user_data is at offset 0 of the rest
                if rest.len() >= 8 {
                    let user_data = u64::from_le_bytes([
                        rest[0], rest[1], rest[2], rest[3], rest[4], rest[5], rest[6], rest[7],
                    ]);
                    events.push((type_, user_data));
                }
            }
            _ => {
                // Unknown event type, skip
            }
        }
    }

    Ok(events)
}

fn read_exact_nonblocking(fd: RawFd, buf: &mut [u8]) -> io::Result<bool> {
    let mut offset = 0;

    while offset < buf.len() {
        let ret = unsafe {
            libc::read(
                fd,
                buf[offset..].as_mut_ptr() as *mut libc::c_void,
                buf.len() - offset,
            )
        };

        if ret < 0 {
            let err = io::Error::last_os_error();
            match err.raw_os_error() {
                Some(code) if is_would_block(code) && offset == 0 => return Ok(false),
                Some(code) if is_would_block(code) => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "partial DRM event read",
                    ));
                }
                Some(libc::EINTR) => continue,
                _ => return Err(err),
            }
        }

        if ret == 0 {
            return if offset == 0 {
                Ok(false)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "partial DRM event read",
                ))
            };
        }

        offset += ret as usize;
    }

    Ok(true)
}

fn is_would_block(code: i32) -> bool {
    code == libc::EAGAIN || code == libc::EWOULDBLOCK
}

/// Global serial counter for page flip user_data.
static FLIP_SERIAL: AtomicU32 = AtomicU32::new(1);

/// Get the next page flip serial.
pub fn next_flip_serial() -> u64 {
    FLIP_SERIAL.fetch_add(1, Ordering::Relaxed) as u64
}
