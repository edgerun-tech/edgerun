use super::*;
use std::sync::Mutex;

static STATE_LOCK: Mutex<()> = Mutex::new(());

fn state_lock() -> std::sync::MutexGuard<'static, ()> {
    STATE_LOCK.lock().unwrap_or_else(|err| err.into_inner())
}

fn tmp_state_dir() -> std::path::PathBuf {
    static C: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = C.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("oci_state_{}_{}", std::process::id(), n));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn with_tmp_state_dir<F: FnOnce()>(f: F) {
    let _lock = state_lock();
    let dir = tmp_state_dir();
    let dir_str = dir.to_string_lossy().to_string();
    set_state_dir(&dir_str);
    f();
    let _ = std::fs::remove_dir_all(&dir);
    clear_state_dir_override();
}

#[test]
fn state_dir_default() {
    let _lock = state_lock();
    clear_state_dir_override();
    assert_eq!(state_dir_base().as_ref(), default_state_dir());
}

#[test]
fn state_dir_custom() {
    let _lock = state_lock();
    let dir = tmp_state_dir();
    let dir_str = dir.to_string_lossy().to_string();
    set_state_dir(&dir_str);
    assert_eq!(state_dir_base().as_ref(), dir_str);
    clear_state_dir_override();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn container_state_dir_path() {
    let _lock = state_lock();
    clear_state_dir_override();
    let expected = if is_root() {
        std::path::Path::new(STATE_DIR).join("my-container")
    } else {
        // Rootless: uses XDG_RUNTIME_DIR or HOME
        std::path::Path::new(default_state_dir().as_str()).join("my-container")
    };
    assert_eq!(container_state_dir("my-container"), expected);
}

#[test]
fn state_file_path_format() {
    let p = state_file_path("test-id");
    assert!(p.to_string_lossy().ends_with("test-id/state.json"));
}

#[test]
fn fifo_path_format() {
    let p = fifo_path("test-id");
    assert!(p.to_string_lossy().ends_with("test-id/start.fifo"));
}

#[test]
fn state_save_and_load() {
    with_tmp_state_dir(|| {
        let state = ContainerState {
            oci_version: "1.0.2".into(),
            id: "test-1".into(),
            status: "created".into(),
            pid: Some(12345),
            bundle: "/tmp/bundle".into(),
            annotations: Some(alloc::collections::BTreeMap::from([(
                "key".into(),
                "value".into(),
            )])),
        };

        save_state(&state, "test-1").unwrap();

        let loaded = load_state("test-1").unwrap();
        assert_eq!(loaded.oci_version, "1.0.2");
        assert_eq!(loaded.id, "test-1");
        assert_eq!(loaded.status, "created");
        assert_eq!(loaded.pid, Some(12345));
        assert_eq!(loaded.bundle, "/tmp/bundle");
        assert_eq!(loaded.annotations.as_ref().unwrap()["key"], "value");
    });
}

#[test]
fn state_exists_checks() {
    with_tmp_state_dir(|| {
        assert!(!state_exists("nonexistent"));

        let state = ContainerState {
            oci_version: "1.0.2".into(),
            id: "exist-test".into(),
            status: "created".into(),
            pid: None,
            bundle: "/tmp/b".into(),
            annotations: None,
        };
        save_state(&state, "exist-test").unwrap();

        assert!(state_exists("exist-test"));
    });
}

#[test]
fn state_delete() {
    with_tmp_state_dir(|| {
        let state = ContainerState {
            oci_version: "1.0.2".into(),
            id: "del-test".into(),
            status: "created".into(),
            pid: None,
            bundle: "/tmp/b".into(),
            annotations: None,
        };
        save_state(&state, "del-test").unwrap();
        assert!(state_exists("del-test"));

        delete_state("del-test");
        assert!(!state_exists("del-test"));
    });
}

#[test]
fn state_delete_nonexistent_is_noop() {
    with_tmp_state_dir(|| {
        delete_state("does-not-exist");
        // Should not panic
    });
}

#[test]
fn state_load_nonexistent_fails() {
    with_tmp_state_dir(|| {
        let result = load_state("nonexistent");
        assert!(result.is_err());
    });
}

#[test]
fn state_status_transitions() {
    with_tmp_state_dir(|| {
        // Simulate lifecycle: creating → created → running → stopped
        for status in &["creating", "created", "running", "stopped"] {
            let state = ContainerState {
                oci_version: "1.0.2".into(),
                id: "lifecycle".into(),
                status: status.to_string(),
                pid: if *status == "creating" {
                    None
                } else {
                    Some(9999)
                },
                bundle: "/tmp/b".into(),
                annotations: None,
            };
            save_state(&state, "lifecycle").unwrap();
            let loaded = load_state("lifecycle").unwrap();
            assert_eq!(loaded.status, *status);
        }
    });
}

#[test]
fn is_root_when_root() {
    // In test environment we're usually root (sudo)
    let uid = unsafe { libc::getuid() };
    assert_eq!(is_root(), uid == 0);
}

#[test]
fn default_state_dir_uses_xdg_when_not_root() {
    // Only meaningful when not root — test is usually root, so just verify structure
    if !is_root() {
        let dir = default_state_dir();
        // Should not be the root STATE_DIR
        assert!(!dir.starts_with(STATE_DIR));
    }
}
