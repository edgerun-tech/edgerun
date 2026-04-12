//! DRM syncobj — explicit synchronization via Linux kernel DRM sync objects.
//!
//! This replaces implicit "fence" barriers with explicit timeline-based
//! synchronization. Each surface has acquire/release syncobj handles that
//! tell the compositor when a buffer is ready to scan out and when the
//! compositor is done with it.

use std::ffi::c_int;
use std::io;
use std::os::fd::RawFd;

use super::ioctl::*;

/// Create a DRM syncobj. Returns the DRM handle.
pub fn syncobj_create(fd: RawFd, flags: u32) -> io::Result<u32> {
    let mut create = DrmSyncobjCreate { flags, handle: 0 };
    let ret = unsafe {
        libc::ioctl(fd, DRM_IOCTL_SYNCOBJ_CREATE, &mut create)
    };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(create.handle)
    }
}

/// Destroy a DRM syncobj.
pub fn syncobj_destroy(fd: RawFd, handle: u32) -> io::Result<()> {
    let mut destroy = DrmSyncobjDestroy { handle, pad: 0 };
    let ret = unsafe {
        libc::ioctl(fd, DRM_IOCTL_SYNCOBJ_DESTROY, &mut destroy)
    };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Import a sync_file fd into a DRM syncobj timeline.
pub fn import_sync_file(fd: RawFd, syncobj_handle: u32, sync_file_fd: c_int) -> io::Result<()> {
    let mut import = DrmSyncobjImportSyncFile {
        handle: syncobj_handle,
        fd: sync_file_fd,
    };
    let ret = unsafe {
        libc::ioctl(fd, DRM_IOCTL_SYNCOBJ_IMPORT_SYNC_FILE, &mut import)
    };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Export a DRM syncobj to a sync_file fd.
pub fn export_sync_file(fd: RawFd, syncobj_handle: u32) -> io::Result<RawFd> {
    let mut export = DrmSyncobjExportSyncFile {
        handle: syncobj_handle,
        fd: -1,
    };
    let ret = unsafe {
        libc::ioctl(fd, DRM_IOCTL_SYNCOBJ_EXPORT_SYNC_FILE, &mut export)
    };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(export.fd)
    }
}

/// Wait on a syncobj timeline to reach a given point.
/// Non-blocking if timeout_nsec is 0.
pub fn timeline_wait(
    fd: RawFd,
    syncobj_handle: u32,
    point: u64,
    timeout_nsec: u64,
    flags: u32,
) -> io::Result<()> {
    let mut wait = DrmSyncobjTimelineWait {
        handles_ptr: &syncobj_handle as *const u32 as *mut u32,
        timelines_ptr: &point as *const u64 as *mut u64,
        timeout_nsec,
        flags,
        count_handles: 1,
        pad: [0; 8],
    };
    let ret = unsafe {
        libc::ioctl(fd, DRM_IOCTL_SYNCOBJ_TIMELINE_WAIT, &mut wait)
    };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Signal a syncobj timeline to a given point.
pub fn timeline_signal(fd: RawFd, syncobj_handle: u32, point: u64) -> io::Result<()> {
    let mut signal = DrmSyncobjTimelineSignal {
        handles_ptr: &syncobj_handle as *const u32 as *mut u32,
        timelines_ptr: &point as *const u64 as *mut u64,
        count_handles: 1,
        flags: 0,
    };
    let ret = unsafe {
        libc::ioctl(fd, DRM_IOCTL_SYNCOBJ_TIMELINE_SIGNAL, &mut signal)
    };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Import a DRM syncobj from a file descriptor (O_RDWR).
/// Returns the DRM syncobj handle.
pub fn syncobj_import(fd: RawFd, syncobj_fd: RawFd) -> io::Result<u32> {
    let mut import = DrmSyncobjFdToHandle {
        fd: syncobj_fd,
        handle: 0,
    };
    let ret = unsafe {
        libc::ioctl(fd, DRM_IOCTL_SYNCOBJ_FD_TO_HANDLE, &mut import)
    };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(import.handle)
    }
}
