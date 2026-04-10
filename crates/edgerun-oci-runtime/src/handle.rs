//! `RunningContainer` handle — await, kill, cgroup kill.

use std::fs;
use std::io;
use std::os::raw::c_int;
use std::path::Path;

use crate::json::OciHook;
use crate::syscalls::{kill, SIGKILL, SIGTERM};

/// A handle to a running container that can be awaited or killed.
pub struct RunningContainer {
    pub(crate) child: std::process::Child,
    pub(crate) cgroup_path: String,
    pub(crate) bundle_path: String,
    pub(crate) pid: u32,
    pub(crate) poststop_hooks: Vec<OciHook>,
}

impl RunningContainer {
    /// Block until the container exits and return its exit status.
    pub fn wait(mut self) -> io::Result<std::process::ExitStatus> {
        self.child.wait()
    }

    /// Kill the container with SIGTERM, then SIGKILL if it doesn't exit.
    /// Returns the exit status after killing.
    pub fn kill(mut self) -> io::Result<std::process::ExitStatus> {
        let pid = self.child.id();
        let _ = unsafe { kill(pid as c_int, SIGTERM) };

        for _ in 0..50 {
            match self.child.try_wait()? {
                Some(status) => return Ok(status),
                None => std::thread::sleep(std::time::Duration::from_millis(100)),
            }
        }

        let _ = unsafe { kill(pid as c_int, SIGKILL) };
        self.child.wait()
    }

    /// Kill the container cgroup (kills all processes in the cgroup).
    /// More reliable than killing a single PID for container preemption.
    pub fn kill_cgroup(&self) {
        let cgroup_root = Path::new("/sys/fs/cgroup").join(self.cgroup_path.trim_start_matches('/'));
        let _ = fs::write(cgroup_root.join("cgroup.kill"), "1");
    }

    /// The host PID of the container's init process.
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// The bundle path for this container.
    pub fn bundle_path(&self) -> &str {
        &self.bundle_path
    }

    /// The cgroup path for this container.
    pub fn cgroup_path(&self) -> &str {
        &self.cgroup_path
    }
}
