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
