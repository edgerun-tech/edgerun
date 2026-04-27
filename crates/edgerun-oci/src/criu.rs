//! CRIU (Checkpoint/Restore In Userspace) integration.
//!
//! This module provides direct syscall-based checkpoint and restore functionality
//! without shelling out to an external CRIU binary.

use crate::prelude::*;
use std::ffi::CString;
use std::io;
use std::os::raw::{c_char, c_int, c_long, c_uint};
use std::path::Path;

#[derive(Clone, Copy)]
#[repr(u32)]
pub enum DumpFlags {
    Empty = 0,
    ShellFds = (1 << 1),
    TcpEstablished = (1 << 2),
    LazyPages = (1 << 3),
    PreDump = (1 << 4),
    WorkNetMigrate = (1 << 5),
    UsernInPipe = (1 << 7),
    ForkLast = (1 << 8),
    LogFiles = (1 << 9),
    RestartLink = (1 << 10),
    AutoDedup = (1 << 11),
    Laconf = (1 << 12),
    SoftLazy = (1 << 13),
    LazyPagesExperimental = (1 << 14),
}

#[derive(Clone, Copy)]
#[repr(u32)]
pub enum RestoreFlags {
    Empty = 0,
    ShellFds = (1 << 1),
    TcpEstablished = (1 << 2),
    LinkRestored = (1 << 3),
    TcpCorked = (1 << 4),
    LazyPages = (1 << 5),
    Unprivileged = (1 << 6),
    ForkRestore = (1 << 7),
    UsernInPipe = (1 << 9),
    WorkNetMigrate = (1 << 10),
    SeizeInherit = (1 << 11),
    RbSoft = (1 << 12),
    RbResetSiginfo = (1 << 13),
}

extern "C" {
    fn syscall(number: c_long, ...) -> c_long;
}

#[cfg(target_arch = "x86_64")]
const SYS_CRIU_CHECKPOINT: i64 = 437;
#[cfg(target_arch = "x86_64")]
const SYS_CRIU_RESTORE: i64 = 438;

#[cfg(target_arch = "aarch64")]
const SYS_CRIU_CHECKPOINT: i64 = 436;
#[cfg(target_arch = "aarch64")]
const SYS_CRIU_RESTORE: i64 = 437;

pub struct CriuDumpOpts<'a> {
    pub pid: i32,
    pub img: &'a Path,
    pub work: Option<&'a Path>,
    pub flags: u32,
    pub status_fd: Option<i32>,
}

pub fn criu_dump(opts: &CriuDumpOpts) -> io::Result<()> {
    let img_c = CString::new(opts.img.to_string_lossy().as_bytes())?;

    let mut work_c = None;
    let work_ptr = if let Some(w) = opts.work {
        work_c = Some(CString::new(w.to_string_lossy().as_bytes())?);
        work_c
            .as_ref()
            .map(|c| c.as_ptr())
            .unwrap_or(std::ptr::null())
    } else {
        std::ptr::null()
    };

    let mut status: i32 = 0;

    let ret = unsafe {
        syscall(
            SYS_CRIU_CHECKPOINT as c_long,
            opts.pid as c_int,
            img_c.as_ptr(),
            opts.flags as c_uint,
            work_ptr,
            &mut status as *mut i32,
        )
    };

    if ret < 0 {
        let err = io::Error::last_os_error();
        return Err(err);
    }

    Ok(())
}

pub struct CriuRestoreOpts<'a> {
    pub img: &'a Path,
    pub flags: u32,
    pub pid: Option<i32>,
    pub status_fd: Option<i32>,
}

pub fn criu_restore(opts: &CriuRestoreOpts) -> io::Result<i32> {
    let img_c = CString::new(opts.img.to_string_lossy().as_bytes())?;

    let mut status: i32 = 0;

    let ret = unsafe {
        syscall(
            SYS_CRIU_RESTORE as c_long,
            img_c.as_ptr(),
            opts.flags as c_uint,
            opts.pid.unwrap_or(0) as c_int,
            &mut status as *mut i32,
        )
    };

    if ret < 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(ret as i32)
}
