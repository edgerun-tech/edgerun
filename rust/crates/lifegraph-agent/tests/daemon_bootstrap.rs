use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn test_root(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("lifegraph-agent-bin-{name}-{nanos}"))
}

#[test]
fn daemon_bootstraps_key_and_blob_files() {
    let root = test_root("bootstrap");
    fs::create_dir_all(&root).unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_lifegraph-agent"))
        .env("LIFEGRAPH_AGENT_DATA_ROOT", &root)
        .env("LIFEGRAPH_AGENT_ID", "integration-agent")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    thread::sleep(Duration::from_secs(2));
    let _ = child.kill();
    let _ = child.wait();

    let key_path = root.join("bootstrap-test-key.hex");
    assert!(
        key_path.exists(),
        "missing bootstrap key: {}",
        key_path.display()
    );

    let blobs_root = root.join("blobs");
    let blob_files: Vec<_> = fs::read_dir(&blobs_root)
        .unwrap()
        .flat_map(|entry| fs::read_dir(entry.unwrap().path()).unwrap())
        .flat_map(|entry| fs::read_dir(entry.unwrap().path()).unwrap())
        .map(|entry| entry.unwrap().path())
        .collect();

    assert!(
        !blob_files.is_empty(),
        "expected at least one blob file in {}",
        blobs_root.display()
    );
    assert!(blob_files
        .iter()
        .any(|p| fs::metadata(p).unwrap().len() > 0));
    assert!(root.join("surreal").exists(), "missing surreal dir");
}
