//! Dumb buffer allocation — kernel-side simple buffer objects.

use std::io;
use std::os::fd::RawFd;
use std::ptr;

use super::ioctl::*;

/// A dumb buffer allocated in DRM-managed memory.
pub struct DumbBuffer {
    pub fd: RawFd,
    pub handle: u32,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub size: u64,
    pub mapped: Option<DumbMapping>,
}

/// Mapped region of a dumb buffer.
pub struct DumbMapping {
    pub ptr: *mut u8,
    pub len: usize,
}

unsafe impl Send for DumbMapping {}

impl DumbBuffer {
    /// Allocate a dumb buffer.
    pub fn create(fd: RawFd, width: u32, height: u32, bpp: u32) -> io::Result<Self> {
        let mut create = DrmModeCreateDumb {
            width,
            height,
            bpp,
            flags: 0,
            handle: 0,
            pitch: 0,
            size: 0,
        };

        let ret = unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_CREATE_DUMB as libc::c_ulong, &mut create) };
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(DumbBuffer {
            fd,
            handle: create.handle,
            width,
            height,
            pitch: create.pitch,
            size: create.size,
            mapped: None,
        })
    }

    /// Map the dumb buffer into userspace.
    pub fn map(&mut self) -> io::Result<&mut [u8]> {
        if self.mapped.is_some() {
            return Ok(unsafe {
                std::slice::from_raw_parts_mut(self.mapped.as_mut().unwrap().ptr, self.mapped.as_ref().unwrap().len)
            });
        }

        // Get mmap offset
        let mut map_req = DrmModeMapDumb {
            handle: self.handle,
            pad: 0,
            offset: 0,
        };

        let ret = unsafe { libc::ioctl(self.fd, DRM_IOCTL_MODE_MAP_DUMB as libc::c_ulong, &mut map_req) };
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        // mmap the buffer
        let ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                self.size as usize,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                self.fd,
                map_req.offset as libc::off_t,
            )
        };

        if ptr == libc::MAP_FAILED {
            return Err(io::Error::last_os_error());
        }

        let slice = unsafe { std::slice::from_raw_parts_mut(ptr as *mut u8, self.size as usize) };
        self.mapped = Some(DumbMapping {
            ptr: ptr as *mut u8,
            len: self.size as usize,
        });

        Ok(slice)
    }

    /// Unmap the buffer.
    pub fn unmap(&mut self) {
        if let Some(mapping) = self.mapped.take() {
            unsafe { libc::munmap(mapping.ptr as *mut libc::c_void, mapping.len) };
        }
    }

    /// Destroy the dumb buffer.
    pub fn destroy(mut self) -> io::Result<()> {
        self.unmap();
        let mut req = DrmModeDestroyDumb { handle: self.handle };
        let ret = unsafe { libc::ioctl(self.fd, DRM_IOCTL_MODE_DESTROY_DUMB as libc::c_ulong, &mut req) };
        if ret < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    /// Create a framebuffer from this dumb buffer.
    pub fn add_fb(&self) -> io::Result<u32> {
        const DRM_FORMAT_XRGB8888: u32 = 0x34325258;

        let mut fb = DrmModeFbCmd2 {
            fb_id: 0,
            width: self.width,
            height: self.height,
            pixel_format: DRM_FORMAT_XRGB8888,
            flags: 0,
            handles: [self.handle, 0, 0, 0],
            pitches: [self.pitch, 0, 0, 0],
            offsets: [0, 0, 0, 0],
            modifier: [0; 4],
            _pad: [0; 4],
        };

        let ret = unsafe { libc::ioctl(self.fd, DRM_IOCTL_MODE_ADDFB2 as libc::c_ulong, &mut fb) };
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(fb.fb_id)
    }
}

impl Drop for DumbBuffer {
    fn drop(&mut self) {
        self.unmap();
        let mut req = DrmModeDestroyDumb { handle: self.handle };
        unsafe { libc::ioctl(self.fd, DRM_IOCTL_MODE_DESTROY_DUMB as libc::c_ulong, &mut req) };
    }
}
