//! FIFO-based synchronization for OCI container start signaling.
//!
//! The OCI runtime spec requires `create` to return before the container
//! workload starts. We use a named pipe (FIFO) to synchronize:
//!
//! 1. **create** creates the FIFO, forks child, child opens FIFO for reading (blocks)
//! 2. **create** returns (container is in "created" state)
//! 3. **start** opens FIFO for writing, writes "go\n", child unblocks
//!
//! The FIFO must be opened for reading BEFORE pivot_root, since the path
//! becomes invalid after the root filesystem is replaced.

use crate::prelude::*;
use std::ffi::CString;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Default FIFO filename within the container state directory.
pub const FIFO_NAME: &str = "start.fifo";

/// Signal payload written to the FIFO to unblock the container child.
pub const START_SIGNAL: &[u8] = b"go\n";

/// Create a FIFO at the given path. Removes any existing file first.
///
/// Returns the path to the created FIFO.
pub fn create_fifo(path: &Path) -> io::Result<PathBuf> {
    // Remove existing file if present (from a previous failed container)
    let _ = fs::remove_file(path);

    let path_c = CString::new(path.to_string_lossy().as_bytes()).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid FIFO path: {}", e),
        )
    })?;

    let ret = unsafe { libc::mkfifo(path_c.as_ptr(), 0o600) };
    if ret != 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(path.to_path_buf())
}

/// Open a FIFO for reading. This blocks until a writer connects.
///
/// Returns a raw file descriptor. The caller is responsible for closing it.
pub fn open_fifo_read(path: &Path) -> io::Result<i32> {
    let path_c = CString::new(path.to_string_lossy().as_bytes()).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid FIFO path: {}", e),
        )
    })?;

    let fd = unsafe { libc::open(path_c.as_ptr(), libc::O_RDONLY) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(fd)
}

/// Open a FIFO for writing and send the start signal.
///
/// This unblocks the child process that is waiting on `open_fifo_read`.
pub fn signal_start(path: &Path) -> io::Result<()> {
    let mut fifo = open_fifo_write(path)?;
    write_start_signal(&mut fifo)
}

/// Open a FIFO for writing. This blocks until the child has opened the read end,
/// which means runtime-only child setup has completed.
pub fn open_fifo_write(path: &Path) -> io::Result<fs::File> {
    fs::OpenOptions::new()
        .write(true)
        .open(path)
        .map_err(|e| io::Error::other(format!("failed to open FIFO for writing: {}", e)))
}

/// Write the start signal to an already-open FIFO writer.
pub fn write_start_signal(fifo: &mut fs::File) -> io::Result<()> {
    fifo.write_all(START_SIGNAL)
        .map_err(|e| io::Error::other(format!("failed to write to FIFO: {}", e)))?;
    fifo.flush()
        .map_err(|e| io::Error::other(format!("failed to flush FIFO: {}", e)))?;
    Ok(())
}

/// Read the start signal from an already-open FIFO file descriptor.
///
/// This should be called after `open_fifo_read` returns.
pub fn read_start_signal(fd: i32) -> io::Result<()> {
    let mut buf = [0u8; 4];
    let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };

    if n <= 0 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "FIFO closed before start signal",
        ));
    }

    if &buf[..n as usize] != START_SIGNAL {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid start signal received",
        ));
    }

    Ok(())
}

/// Close a FIFO file descriptor.
pub fn close_fifo(fd: i32) {
    unsafe { libc::close(fd) };
}

/// Remove a FIFO file and its parent state directory if empty.
pub fn cleanup_fifo(path: &Path) {
    let _ = fs::remove_file(path);
    if let Some(parent) = path.parent() {
        let _ = fs::remove_dir(parent);
    }
}

// ===========================================================================
// Tests
// ===========================================================================
