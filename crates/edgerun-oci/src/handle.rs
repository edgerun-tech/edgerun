//! `RunningContainer` handle — await, kill, cgroup kill.

use crate::prelude::*;
use std::fs;
use std::io;
use std::os::raw::c_int;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;

use crate::json::OciHook;
use crate::syscalls::{kill, SIGKILL, SIGTERM};

/// A handle to a running container that can be awaited or killed.
pub struct RunningContainer {
    pub(crate) cgroup_path: String,
    pub(crate) bundle_path: String,
    pub(crate) pid: u32,
    pub(crate) poststop_hooks: Vec<OciHook>,
}

impl RunningContainer {
    /// Block until the container exits and return its exit status.
    pub fn wait(self) -> io::Result<std::process::ExitStatus> {
        // Use waitpid directly since we have the PID from raw fork()
        let mut status: c_int = 0;
        let result = unsafe { libc::waitpid(self.pid as i32, &mut status, 0) };

        if result < 0 {
            return Err(io::Error::last_os_error());
        }

        // Convert wait status to ExitStatus using ExitStatusExt::from_raw
        Ok(std::process::ExitStatus::from_raw(status as i32))
    }

    /// Kill the container with SIGTERM, then SIGKILL if it doesn't exit.
    /// Returns the exit status after killing.
    pub fn kill(self) -> io::Result<std::process::ExitStatus> {
        let pid = self.pid;
        let _ = unsafe { kill(pid as c_int, SIGTERM) };

        for _ in 0..50 {
            let mut status: c_int = 0;
            let result = unsafe { libc::waitpid(pid as i32, &mut status, libc::WNOHANG) };
            if result > 0 {
                return Ok(std::process::ExitStatus::from_raw(status as i32));
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        let _ = unsafe { kill(pid as c_int, SIGKILL) };

        // Wait for final exit
        let mut status: c_int = 0;
        unsafe { libc::waitpid(pid as i32, &mut status, 0) };
        Ok(std::process::ExitStatus::from_raw(status as i32))
    }

    /// Kill the container cgroup (kills all processes in the cgroup).
    /// More reliable than killing a single PID for container preemption.
    pub fn kill_cgroup(&self) {
        let cgroup_root =
            Path::new("/sys/fs/cgroup").join(self.cgroup_path.trim_start_matches('/'));
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
