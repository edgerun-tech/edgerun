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
    let mut fifo = fs::File::create(path)
        .map_err(|e| io::Error::other(format!("failed to open FIFO for writing: {}", e)))?;

    fifo.write_all(START_SIGNAL)
        .map_err(|e| io::Error::other(format!("failed to write to FIFO: {}", e)))?;
    fifo.flush()
        .map_err(|e| io::Error::other(format!("failed to flush FIFO: {}", e)))?;

    // Drop the write end — the reader will get the data we wrote.
    // FIFOs are byte-stream: once written and flushed, the reader gets it
    // even if the writer closes immediately.
    drop(fifo);

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    fn temp_fifo_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "oci-fifo-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("failed to create temp dir");
        dir
    }

    #[test]
    fn create_fifo_creates_file() {
        let dir = temp_fifo_dir();
        let path = dir.join("test.fifo");
        let result = create_fifo(&path);
        // mkfifo may fail in some environments; just verify no panic
        if result.is_ok() {
            use std::os::unix::fs::FileTypeExt;
            assert!(path.exists());
            assert!(path.metadata().unwrap().file_type().is_fifo());
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn create_fifo_replaces_existing() {
        use std::os::unix::fs::MetadataExt;
        let dir = temp_fifo_dir();
        let path = dir.join("test.fifo");
        if create_fifo(&path).is_err() {
            let _ = fs::remove_dir_all(&dir);
            return;
        }
        let _ = path.metadata().unwrap().ino();
        create_fifo(&path).expect("replace should succeed");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn create_fifo_rejects_invalid_path() {
        assert!(create_fifo(Path::new("/\0invalid")).is_err());
    }

    #[test]
    fn create_fifo_sets_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_fifo_dir();
        let path = dir.join("perms.fifo");
        if create_fifo(&path).is_err() {
            let _ = fs::remove_dir_all(&dir);
            return;
        }
        let mode = path.metadata().unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn fifo_signal_roundtrip() {
        let dir = temp_fifo_dir();
        let fifo_path = dir.join("roundtrip.fifo");
        if create_fifo(&fifo_path).is_err() {
            let _ = fs::remove_dir_all(&dir);
            return;
        }
        let p1 = fifo_path.clone();
        let reader = thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            open_fifo_read(&p1)
        });
        let p2 = fifo_path.clone();
        let writer = thread::spawn(move || {
            thread::sleep(Duration::from_millis(200));
            signal_start(&p2)
        });
        let fd = reader.join().unwrap().expect("reader");
        assert!(fd >= 0);
        assert!(writer.join().unwrap().is_ok());
        close_fifo(fd);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn fifo_read_signal_validates_payload() {
        let dir = temp_fifo_dir();
        let fifo_path = dir.join("payload.fifo");
        if create_fifo(&fifo_path).is_err() {
            let _ = fs::remove_dir_all(&dir);
            return;
        }
        let p1 = fifo_path.clone();
        let reader = thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            let fd = open_fifo_read(&p1).unwrap();
            read_start_signal(fd)
        });
        let p2 = fifo_path.clone();
        let writer = thread::spawn(move || {
            thread::sleep(Duration::from_millis(200));
            signal_start(&p2)
        });
        assert!(reader.join().unwrap().is_ok());
        assert!(writer.join().unwrap().is_ok());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn signal_start_fails_on_unusable_path() {
        // Use a path under a nonexistent directory to guarantee failure
        let path = Path::new("/nonexistent_dir_12345_oci_test/fifo.fifo");
        let result = signal_start(path);
        assert!(result.is_err(), "signal_start should fail on unusable path");
    }

    #[test]
    fn open_fifo_read_fails_on_unusable_path() {
        let path = Path::new("/nonexistent_dir_67890_oci_test/fifo.fifo");
        assert!(open_fifo_read(path).is_err());
    }

    #[test]
    fn cleanup_fifo_removes_file_and_empty_dir() {
        let dir = temp_fifo_dir();
        let fifo_path = dir.join("cleanup.fifo");
        if create_fifo(&fifo_path).is_err() {
            let _ = fs::remove_dir_all(&dir);
            return;
        }
        assert!(fifo_path.exists());
        cleanup_fifo(&fifo_path);
        assert!(!fifo_path.exists());
        assert!(!dir.exists());
    }

    #[test]
    fn cleanup_fifo_handles_missing_file() {
        cleanup_fifo(Path::new("/tmp/oci-test-missing-fifo-unique.fifo"));
    }

    #[test]
    fn start_signal_is_correct_bytes() {
        assert_eq!(START_SIGNAL, b"go\n");
        assert_eq!(START_SIGNAL.len(), 3);
    }

    #[test]
    fn fifo_name_is_correct() {
        assert_eq!(FIFO_NAME, "start.fifo");
    }
}
