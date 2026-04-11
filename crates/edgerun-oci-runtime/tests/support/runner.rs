//! Container lifecycle runner for conformance tests.

use crate::bundle::{self, Bundle};

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const RUNTIME: &str = env!("CARGO_BIN_EXE_edgerun-oci");
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
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "invalid bundle path"))?;
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

    /// Run the full create→start→delete lifecycle.
    pub fn run(&self) -> io::Result<String> {
        let out = self.create()?;
        if !out.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "create failed: {}",
                    String::from_utf8_lossy(&out.stderr)
                ),
            ));
        }

        let out = self.start()?;
        if !out.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("start failed: {}", String::from_utf8_lossy(&out.stderr)),
            ));
        }

        let state_out = self.state()?;
        let state = String::from_utf8_lossy(&state_out.stdout).to_string();

        let _ = self.delete();
        Ok(state)
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
    if unsafe { libc::getuid() } == 0 {
        Command::new(RUNTIME).args(args).output()
    } else {
        Command::new("sudo").arg("-n").arg(RUNTIME).args(args).output()
    }
}
