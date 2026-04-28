use crate::prelude::*;
use std::ffi::CString;
use std::fs;
use std::io;
use std::path::Path;

use crate::spec::OciLinuxDevice;
use crate::syscalls::{chown, makedev, mknod, S_IFCHR};

fn c_string(value: &str) -> io::Result<CString> {
    CString::new(value).map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))
}

fn libc_unit(ret: i32) -> io::Result<()> {
    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

pub(crate) fn create_rootfs_devices(spec_devices: Option<&[OciLinuxDevice]>) {
    create_essential_devices();

    if let Some(devices) = spec_devices {
        for device in devices {
            let _ = create_spec_device(device);
        }
    }
}

fn create_device(path: &str, major: u64, minor: u64, mode: u32) {
    let dev = makedev(major, minor);
    let path_c = match c_string(path) {
        Ok(path) => path,
        Err(_) => return,
    };
    let _ = unsafe { mknod(path_c.as_ptr(), S_IFCHR | mode, dev) };
}

fn create_spec_device(device: &OciLinuxDevice) -> io::Result<()> {
    let dev_type = match device.ns_type.as_str() {
        "c" | "char" => S_IFCHR,
        "b" | "block" => 0o060000,
        "p" | "fifo" => 0o010000,
        _ => return Ok(()),
    };

    let mode = device.file_mode.unwrap_or(0o660);
    let major = device.major.unwrap_or(0) as u64;
    let minor = device.minor.unwrap_or(0) as u64;

    if let Some(parent) = Path::new(&device.path).parent() {
        let _ = fs::create_dir_all(parent);
    }

    let path_c = match c_string(device.path.as_str()) {
        Ok(path) => path,
        Err(_) => return Ok(()),
    };

    let dev = makedev(major, minor);
    libc_unit(unsafe { mknod(path_c.as_ptr(), dev_type | mode, dev) })?;

    if device.uid.is_some() || device.gid.is_some() {
        let uid = device.uid.unwrap_or(u32::MAX);
        let gid = device.gid.unwrap_or(u32::MAX);
        libc_unit(unsafe { chown(path_c.as_ptr(), uid, gid) })?;
    }

    Ok(())
}

fn create_essential_devices() {
    create_device("/dev/null", 1, 3, 0o666);
    create_device("/dev/zero", 1, 5, 0o666);
    create_device("/dev/full", 1, 7, 0o666);
    create_device("/dev/random", 1, 8, 0o444);
    create_device("/dev/urandom", 1, 9, 0o444);
    create_device("/dev/tty", 5, 0, 0o666);
    let _ = fs::create_dir_all("/dev/pts");
    let _ = fs::create_dir_all("/dev/shm");
}
