//! Test harness — spawns the compositor, provides client connections.

use std::io;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use crate::client::WlClient;

/// Default path to the compositor binary.
pub const COMPOSITOR_PATH: &str = "target/release/edgerun-compositor";
const STARTUP_TIMEOUT: Duration = Duration::from_secs(10);

/// Counter for unique test socket names.
static SOCKET_COUNTER: AtomicU32 = AtomicU32::new(0);

/// A running compositor for testing.
pub struct CompositorHarness {
    process: Child,
    socket_path: PathBuf,
}

impl CompositorHarness {
    pub fn new() -> io::Result<Self> {
        // Unique socket per test instance
        let counter = SOCKET_COUNTER.fetch_add(1, Ordering::Relaxed);
        let socket_path = PathBuf::from(format!(
            "/tmp/edgerun-test-wayland-{}-{}",
            std::process::id(),
            counter
        ));
        let _ = std::fs::remove_file(&socket_path);
        let compositor_path = Self::find_compositor()?;

        eprintln!("[harness] Spawning compositor at {:?}", compositor_path);
        let mut process = Command::new(&compositor_path)
            .arg(&socket_path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| io::Error::new(io::ErrorKind::Other,
                format!("Failed to spawn compositor: {}", e)))?;

        let start = Instant::now();
        while start.elapsed() < STARTUP_TIMEOUT {
            if socket_path.exists() { break; }
            thread::sleep(Duration::from_millis(50));
            if let Some(status) = process.try_wait().ok().flatten() {
                // Read stderr to understand why it failed
                if let Some(ref mut stderr) = process.stderr {
                    let mut buf = Vec::new();
                    use std::io::Read;
                    let _ = stderr.read_to_end(&mut buf);
                    eprintln!("[harness] Compositor stderr: {}", String::from_utf8_lossy(&buf));
                }
                return Err(io::Error::new(io::ErrorKind::Other,
                    format!("Compositor exited early: {:?}", status)));
            }
        }

        if !socket_path.exists() {
            let _ = process.kill();
            return Err(io::Error::new(io::ErrorKind::TimedOut,
                format!("Socket not created within {:?}", STARTUP_TIMEOUT)));
        }

        // Verify socket is accepting connections
        match UnixStream::connect(&socket_path) {
            Ok(_) => {},
            Err(e) => {
                let _ = process.kill();
                return Err(io::Error::new(io::ErrorKind::Other,
                    format!("Cannot connect to socket: {}", e)));
            }
        }

        eprintln!("[harness] Compositor running on {:?}", socket_path);
        Ok(Self { process, socket_path })
    }

    fn find_compositor() -> io::Result<PathBuf> {
        for p in &[
            COMPOSITOR_PATH,
            "target/release/edgerun-compositor",
            "target/debug/edgerun-compositor",
            "../target/release/edgerun-compositor",
            "../target/debug/edgerun-compositor",
            "../../target/release/edgerun-compositor",
            "../../target/debug/edgerun-compositor",
        ] {
            let path = PathBuf::from(p);
            if path.exists() { return Ok(path.canonicalize()?); }
        }
        Err(io::Error::new(io::ErrorKind::NotFound,
            "Compositor binary not found. Run `cargo build -p edgerun-compositor --release`"))
    }

    pub fn socket_path(&self) -> &Path { &self.socket_path }

    /// Connect a raw client — returns a UnixStream to the Wayland socket.
    pub fn connect_raw(&self) -> io::Result<UnixStream> {
        UnixStream::connect(&self.socket_path)
    }

    /// Connect a high-level [`WlClient`].
    pub fn connect(&self) -> io::Result<WlClient> {
        let stream = self.connect_raw()?;
        Ok(WlClient::new(stream))
    }
}

impl Drop for CompositorHarness {
    fn drop(&mut self) {
        eprintln!("[harness] Shutting down compositor...");
        let _ = self.process.kill();
        let _ = self.process.wait();
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

#[derive(Debug, Clone)]
pub struct TestResult {
    pub protocol: String,
    pub interface: String,
    pub test_name: String,
    pub passed: bool,
    pub message: String,
}

impl TestResult {
    pub fn pass(protocol: &str, interface: &str, test_name: &str, message: &str) -> Self {
        Self { protocol: protocol.into(), interface: interface.into(), test_name: test_name.into(), passed: true, message: message.into() }
    }
    pub fn fail(protocol: &str, interface: &str, test_name: &str, message: &str) -> Self {
        Self { protocol: protocol.into(), interface: interface.into(), test_name: test_name.into(), passed: false, message: message.into() }
    }
}
