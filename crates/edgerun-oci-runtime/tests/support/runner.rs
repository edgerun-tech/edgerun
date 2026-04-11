//! Container lifecycle runner for conformance tests.

use crate::bundle::{self, Bundle};

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const STATE_DIR: &str = "/run/edgerun-oci";

/// Runs a container through its lifecycle.
pub struct Runner {
    id: String,
    bundle: Bundle,
    bundle_path: PathBuf,
}

impl Runner {
    /// Create a new runner with a unique bundle directory.
    pub fn new(name: &str) -> Self {
        let ts = unique_id();
        let id = format!("oci-{name}-{ts}");
        let bundle_path = std::env::temp_dir().join(format!("oci-{name}-{ts}"));
        Self {
            id,
            bundle: bundle::minimal(),
            bundle_path,
        }
    }

    /// Set a custom bundle.
    pub fn bundle(&mut self, bundle: Bundle) -> &mut Self {
        self.bundle = bundle;
        self
    }

    /// The container ID.
    pub fn id(&self) -> &str { &self.id }

    /// The bundle path on disk.
    pub fn bundle_path(&self) -> &Path {
        &self.bundle_path
    }

    /// Build the bundle on disk.
    pub fn build(&self) -> io::Result<()> {
        fs::create_dir_all(&self.bundle_path)?;
        self.bundle.write_to(&self.bundle_path)
    }

    /// Create the container.
    pub fn create(&self) -> io::Result<Output> {
        self.build()?;
        let bundle_str = self
            .bundle_path
            .to_str()
            .ok_or_else(|| io::Error::other("invalid bundle path"))?;
        cli(&["create", "--bundle", bundle_str, &self.id])
    }

    /// Start the container.
    pub fn start(&self) -> io::Result<Output> {
        cli(&["start", &self.id])
    }

    /// Delete the container (requires stopped state).
    pub fn delete(&self) -> io::Result<Output> {
        let out = cli(&["delete", &self.id])?;
        let _ = fs::remove_dir_all(&self.bundle_path);
        Ok(out)
    }

    /// Force delete the container.
    pub fn delete_force(&self) -> io::Result<Output> {
        let out = cli(&["delete", "--force", &self.id])?;
        let _ = fs::remove_dir_all(&self.bundle_path);
        Ok(out)
    }

    /// Get container state JSON.
    pub fn state(&self) -> io::Result<Output> {
        cli(&["state", &self.id])
    }

/// Run the full create→start→wait→delete lifecycle.
    /// Waits for the container process to exit before deleting.
    pub fn run(&self) -> io::Result<String> {
        let out = self.create()?;
        if !out.status.success() {
            return Err(io::Error::other(
                format!(
                    "create failed: {}",
                    String::from_utf8_lossy(&out.stderr)
                ),
            ));
        }

        let out = self.start()?;
        if !out.status.success() {
            return Err(io::Error::other(
                format!("start failed: {}", String::from_utf8_lossy(&out.stderr)),
            ));
        }

        // Wait for the container to exit (poll state until it says "stopped")
        for _ in 0..100 {
            let state_out = self.state()?;
            let json = String::from_utf8_lossy(&state_out.stdout);
            if json.contains("\"stopped\"") {
                let _ = self.delete();
                return Ok(json.to_string());
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        // Force delete if container hasn't stopped
        let _ = self.delete_force();
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "container did not stop within 10 seconds",
        ))
    }

    /// Assert the container state matches the expected status.
    pub fn assert_state(&self, expected: &str) {
        let out = self.state().expect("state command failed");
        assert!(out.status.success(), "state command failed");
        let json = String::from_utf8_lossy(&out.stdout);
        assert!(
            json.contains(&format!("\"{expected}\"")),
            "expected state '{expected}', got: {json}"
        );
    }

    /// Assert the container state file has been deleted.
    pub fn assert_state_deleted(&self) {
        let out = self.state();
        assert!(
            out.is_err() || !out.as_ref().map(|o| o.status.success()).unwrap_or(false),
            "state should fail after delete"
        );
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        cleanup(&self.id);
        let _ = fs::remove_dir_all(&self.bundle_path);
    }
}

// ===========================================================================
// Helpers
// ===========================================================================

fn unique_id() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

fn cleanup(id: &str) {
    let _ = Command::new("rm")
        .args(["-rf", &format!("{STATE_DIR}/{id}")])
        .output();
}

/// Run the edgerun-oci CLI binary.
fn cli(args: &[&str]) -> io::Result<Output> {
    let runtime = env!("CARGO_BIN_EXE_edgerun-oci");
    if is_root() {
        Command::new(runtime).args(args).output()
    } else {
        Command::new("sudo").arg("-n").arg(runtime).args(args).output()
    }
}

/// Helper for tests that need to call `create` directly with a specific ID and bundle.
/// Used by the duplicate ID test.
pub fn cli_create(id: &str, bundle_path: &Path) -> Output {
    let runtime = env!("CARGO_BIN_EXE_edgerun-oci");
    let bundle_str = bundle_path.to_str().expect("invalid bundle path");
    if is_root() {
        Command::new(runtime)
            .arg("--bundle").arg(bundle_str)
            .arg("create").arg(id)
            .output().expect("create command failed")
    } else {
        Command::new("sudo").arg("-n")
            .arg(runtime)
            .arg("--bundle").arg(bundle_str)
            .arg("create").arg(id)
            .output().expect("create command failed")
    }
}

/// Check if running as root (UID 0).
fn is_root() -> bool {
    // Read UID from /proc — avoids libc dependency
    match std::fs::read_to_string("/proc/self/status") {
        Ok(content) => content.lines().any(|line| {
            line.starts_with("Uid:") && line.split_whitespace().nth(1) == Some("0")
        }),
        Err(_) => {
            // Fallback: try `id -u` command
            Command::new("id").arg("-u").output()
                .ok()
                .and_then(|out| String::from_utf8(out.stdout).ok())
                .map(|s| s.trim() == "0")
                .unwrap_or(false)
        }
    }
}
