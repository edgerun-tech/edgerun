//! DRM/KMS backend — direct Linux kernel via ioctls.

use std::io;
use std::os::fd::RawFd;

pub mod device;
pub mod dumb;
pub mod ioctl;
pub mod kms;
pub mod syncobj;

/// Get the size of a file descriptor via fstat.
pub fn fd_size(fd: RawFd) -> io::Result<usize> {
    let mut st = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
    let ret = unsafe { libc::fstat(fd, &mut st) };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(st.st_size as usize)
    }
}

/// Find available DRM devices (/dev/dri/card*).
pub fn find_card_devices() -> Vec<String> {
    use std::fs;

    let mut devices = Vec::new();
    if let Ok(entries) = fs::read_dir("/dev/dri") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("card") && !name.contains("-") {
                devices.push(format!("/dev/dri/{}", name));
            }
        }
    }
    devices.sort();
    devices
}

/// Find render nodes (/dev/dri/renderD*).
pub fn find_render_devices() -> Vec<String> {
    use std::fs;

    let mut devices = Vec::new();
    if let Ok(entries) = fs::read_dir("/dev/dri") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("renderD") {
                devices.push(format!("/dev/dri/{}", name));
            }
        }
    }
    devices.sort();
    devices
}
