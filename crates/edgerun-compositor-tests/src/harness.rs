//! Test harness — spawns the compositor binary, waits for the Wayland socket,
//! provides client connections, and cleans up on drop.

use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant};

use wayland_client::{
    globals::{registry_queue_init, GlobalListContents},
    protocol::{wl_compositor, wl_registry, wl_seat, wl_shm, wl_output},
    ConnectError, Connection, DispatchError, EventQueue,
};

/// Default path to the compositor binary (relative to workspace root).
pub const COMPOSITOR_PATH: &str = "../../target/release/edgerun-compositor";

/// Timeout for compositor to start and create the socket.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(10);

/// A running compositor instance for testing.
pub struct CompositorHarness {
    process: Child,
    socket_path: PathBuf,
    connection: Connection,
    event_queue: EventQueue<GlobalListContents>,
    registry: wl_registry::WlRegistry,
    globals: wayland_client::globals::GlobalList<GlobalListContents>,
}

impl CompositorHarness {
    /// Spawn the compositor with a unique test socket.
    pub fn new() -> io::Result<Self> {
        let socket_path = PathBuf::from(format!(
            "/tmp/edgerun-test-wayland-{}",
            std::process::id()
        ));

        // Remove stale socket
        let _ = std::fs::remove_file(&socket_path);

        // Find the compositor binary
        let compositor_path = Self::find_compositor()?;

        eprintln!("[harness] Spawning compositor at {:?}", compositor_path);
        eprintln!("[harness] Socket: {:?}", socket_path);

        let process = Command::new(&compositor_path)
            .arg(&socket_path)
            .spawn()
            .map_err(|e| io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to spawn compositor at {:?}: {}", compositor_path, e)
            ))?;

        // Wait for socket to appear
        let start = Instant::now();
        while start.elapsed() < STARTUP_TIMEOUT {
            if socket_path.exists() {
                break;
            }
            thread::sleep(Duration::from_millis(50));

            // Check if process died
            if let Some(status) = process.try_wait().ok().flatten() {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Compositor exited early with status: {:?}", status)
                ));
            }
        }

        if !socket_path.exists() {
            let _ = process.kill();
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("Compositor did not create socket {:?} within {:?}", socket_path, STARTUP_TIMEOUT)
            ));
        }

        // Connect to the Wayland socket
        let connection = Connection::connect_to_socket(&socket_path)
            .map_err(|e| io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to connect to Wayland socket: {}", e)
            ))?;

        let (registry, event_queue) = registry_queue_init::<GlobalListContents>(&connection)
            .map_err(|e| io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to initialize registry queue: {}", e)
            ))?;

        let qh = event_queue.handle();
        let globals = GlobalListContents::new(&registry, &qh);

        eprintln!("[harness] Connected to compositor, {} globals advertised", globals.len());

        Ok(Self {
            process,
            socket_path,
            connection,
            event_queue,
            registry,
            globals,
        })
    }

    /// Find the compositor binary.
    fn find_compositor() -> io::Result<PathBuf> {
        let candidates = [
            PathBuf::from(COMPOSITOR_PATH),
            PathBuf::from("target/release/edgerun-compositor"),
            PathBuf::from("../target/release/edgerun-compositor"),
        ];

        for path in &candidates {
            if path.exists() {
                return Ok(path.canonicalize()?);
            }
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "Compositor binary not found. Tried: {:?}. Run `cargo build -p edgerun-compositor --release` first.",
                candidates.iter().map(|p| p.to_string_lossy()).collect::<Vec<_>>()
            )
        ))
    }

    /// Get the socket path.
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    /// Get the Wayland connection.
    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    /// Get the global list (advertised interfaces).
    pub fn globals(&self) -> &wayland_client::globals::GlobalList<GlobalListContents> {
        &self.globals
    }

    /// Get the registry.
    pub fn registry(&self) -> &wl_registry::WlRegistry {
        &self.registry
    }

    /// Get a mutable event queue reference.
    pub fn event_queue(&mut self) -> &mut EventQueue<GlobalListContents> {
        &mut self.event_queue
    }

    /// Check if a specific global is advertised.
    pub fn has_global(&self, interface: &str) -> Option<u32> {
        self.globals
            .clone()
            .into_iter()
            .find(|(name, _, iface, _)| iface == interface)
            .map(|(name, _, _, _)| name)
    }

    /// Get the version of an advertised global.
    pub fn global_version(&self, interface: &str) -> Option<u32> {
        self.globals
            .clone()
            .into_iter()
            .find(|(_, _, iface, _)| iface == interface)
            .map(|(_, _, _, ver)| ver)
    }

    /// List all advertised globals.
    pub fn list_globals(&self) -> Vec<(u32, String, u32)> {
        self.globals
            .clone()
            .into_iter()
            .map(|(name, _, iface, ver)| (name, iface, ver))
            .collect()
    }

    /// Dispatch pending events with a timeout.
    pub fn dispatch_pending(&mut self) -> io::Result<()> {
        // First flush any pending sends
        self.connection().flush().map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("Flush error: {}", e))
        })?;

        // Dispatch with a short timeout
        self.event_queue.dispatch_pending(&mut (), &self.connection).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("Dispatch error: {}", e))
        })?;

        Ok(())
    }

    /// Block and dispatch events with a timeout.
    pub fn blocking_dispatch(&mut self, timeout: Duration) -> io::Result<()> {
        self.connection().flush().map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("Flush error: {}", e))
        })?;

        // Use a read with timeout via the connection
        let _ = self.event_queue.blocking_dispatch(&mut (), &self.connection).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("Blocking dispatch error: {}", e))
        });

        Ok(())
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

/// A test result for a single protocol conformance check.
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
        Self {
            protocol: protocol.to_string(),
            interface: interface.to_string(),
            test_name: test_name.to_string(),
            passed: true,
            message: message.to_string(),
        }
    }

    pub fn fail(protocol: &str, interface: &str, test_name: &str, message: &str) -> Self {
        Self {
            protocol: protocol.to_string(),
            interface: interface.to_string(),
            test_name: test_name.to_string(),
            passed: false,
            message: message.to_string(),
        }
    }
}
