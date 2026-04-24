/// Running workload registry — tracks active workloads and enables preemption.
///
/// The registry stores metadata needed to signal a running workload to stop.
/// The actual cleanup (resource release, directory removal, accounting) is
/// owned by the background thread that awaits the container — **never** by
/// `terminate()`. This avoids double-free races.
///
/// ## Concurrency model
/// - `RunningWorkloads` is a thread-safe registry accessed from the dispatch
///   thread (register/lookup/terminate) and the background thread (unregister).
/// - `terminate()` only kills the container process. The background thread
///   detects the exit, does cleanup once, and unregisters.
/// - A `kill_requested` AtomicBool provides a lock-free signal from
///   `terminate()` to the background thread.
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::RwLock;

/// Metadata about a running workload (stored in the registry).
/// The background thread owns the RunningContainer handle; the registry
/// only stores what's needed to kill the process if preempted.
#[derive(Clone, Debug)]
pub struct RunningWorkloadInfo {
    /// The unique work identifier.
    pub work_id: [u8; 32],
    /// Host PID of the container's init process.
    pub pid: u32,
    /// Allocated cores (for release on termination).
    pub allocated_cores: u32,
    /// Allocated memory bytes (for release on termination).
    pub allocated_memory_bytes: u64,
    /// Path to the temporary bundle directory (for cleanup).
    pub bundle_path: std::path::PathBuf,
    /// Cgroup path (for cgroup-level kill).
    pub cgroup_path: String,
}

/// Internal entry: info + kill signal.
struct WorkloadEntry {
    info: RunningWorkloadInfo,
    /// Set to true by terminate() to signal the background thread.
    kill_requested: AtomicBool,
}

/// Thread-safe registry of running workloads.
///
/// ## Invariants
/// - A workload is in the registry iff it is running or being killed.
/// - `terminate()` sets `kill_requested` and kills the process, but does
///   NOT release resources or remove the entry — the background thread
///   does all that after detecting the exit.
/// - Natural exit: background thread detects it, cleans up, unregisters.
pub struct RunningWorkloads {
    workloads: RwLock<HashMap<[u8; 32], WorkloadEntry>>,
    /// Monotonically increasing counter for total workloads ever started.
    total_started: AtomicU64,
    /// Monotonically increasing counter for total workloads completed.
    total_completed: AtomicU64,
}

impl RunningWorkloads {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            workloads: RwLock::new(HashMap::new()),
            total_started: AtomicU64::new(0),
            total_completed: AtomicU64::new(0),
        }
    }

    /// Register a new running workload.
    /// Returns `Ok(())` if registered, `Err(existing)` if a workload with
    /// the same work_id is already in the registry.
    pub fn register(&self, info: RunningWorkloadInfo) -> Result<(), RunningWorkloadInfo> {
        let mut map = self.workloads.write().expect("workloads map poisoned");
        if map.contains_key(&info.work_id) {
            return Err(info);
        }
        map.insert(info.work_id, WorkloadEntry {
            info: info.clone(),
            kill_requested: AtomicBool::new(false),
        });
        self.total_started.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Look up a running workload by work_id.
    pub fn get(&self, work_id: &[u8; 32]) -> Option<RunningWorkloadInfo> {
        let map = self.workloads.read().expect("workloads map poisoned");
        map.get(work_id).map(|e| e.info.clone())
    }

    /// Signal that a workload should be terminated.
    ///
    /// This:
    /// 1. Sets the `kill_requested` flag (signals the background thread)
    /// 2. Kills the container via cgroup.kill or SIGTERM/SIGKILL
    ///
    /// It does NOT release resources or remove the entry — the background
    /// thread does all that after detecting the process exit.
    ///
    /// Returns `Some(info)` if the workload was found, `None` if not.
    pub fn terminate(&self, work_id: &[u8; 32]) -> Option<RunningWorkloadInfo> {
        let entry = {
            let map = self.workloads.read().expect("workloads map poisoned");
            map.get(work_id).map(|e| {
                e.kill_requested.store(true, Ordering::Release);
                e.info.clone()
            })
        };

        if let Some(ref info) = entry {
            Self::kill_container(info);
        }

        entry
    }

    /// Kill the container associated with a workload info record.
    fn kill_container(info: &RunningWorkloadInfo) {
        // Method 1: cgroup v2 kill (kernel 5.15+, kills all processes in cgroup)
        let cgroup_root = std::path::Path::new("/sys/fs/cgroup")
            .join(info.cgroup_path.trim_start_matches('/'));
        if std::fs::write(cgroup_root.join("cgroup.kill"), "1").is_ok() {
            edgerun_log::info!("killed workload {} via cgroup.kill (pid {})",
                edgerun_core::util::bytes_to_hex(&info.work_id[..8]),
                info.pid);
            return;
        }

        // Method 2: SIGTERM → wait 5s → SIGKILL on init PID
        let term_result = unsafe { libc::kill(info.pid as libc::pid_t, libc::SIGTERM) };
        if term_result != 0 {
            edgerun_log::warn!("SIGTERM failed for workload {} (pid {}): {}",
                edgerun_core::util::bytes_to_hex(&info.work_id[..8]),
                info.pid, std::io::Error::last_os_error());
        }

        for _ in 0..50 {
            if !Self::process_exists(info.pid) {
                edgerun_log::info!("workload {} exited gracefully (pid {})",
                    edgerun_core::util::bytes_to_hex(&info.work_id[..8]),
                    info.pid);
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        // SIGKILL if still alive
        let kill_result = unsafe { libc::kill(info.pid as libc::pid_t, libc::SIGKILL) };
        if kill_result != 0 {
            edgerun_log::error!("SIGKILL failed for workload {} (pid {}): {}",
                edgerun_core::util::bytes_to_hex(&info.work_id[..8]),
                info.pid, std::io::Error::last_os_error());
        } else {
            edgerun_log::info!("force-killed workload {} via SIGKILL (pid {})",
                edgerun_core::util::bytes_to_hex(&info.work_id[..8]),
                info.pid);
        }
    }

    /// Check if a process is still running.
    fn process_exists(pid: u32) -> bool {
        unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
    }

    /// Check if kill was requested for a workload (for the background thread).
    pub fn is_kill_requested(&self, work_id: &[u8; 32]) -> bool {
        let map = self.workloads.read().expect("workloads map poisoned");
        map.get(work_id).map(|e| e.kill_requested.load(Ordering::Acquire)).unwrap_or(false)
    }

    /// Unregister a workload after the background thread has completed cleanup.
    /// Called by the background thread after it has released resources and
    /// cleaned up the bundle directory.
    pub fn unregister(&self, work_id: &[u8; 32]) -> Option<RunningWorkloadInfo> {
        let mut map = self.workloads.write().expect("workloads map poisoned");
        let entry = map.remove(work_id)?;
        self.total_completed.fetch_add(1, Ordering::Relaxed);
        Some(entry.info)
    }

    /// Terminate all running workloads (used during shutdown).
    /// Returns a list of signaled workloads. Cleanup is async — the caller
    /// should wait for background threads to finish.
    pub fn terminate_all(&self) -> Vec<RunningWorkloadInfo> {
        let all_ids: Vec<[u8; 32]> = {
            let map = self.workloads.read().expect("workloads map poisoned");
            map.keys().copied().collect()
        };

        let mut results = Vec::new();
        for id in &all_ids {
            if let Some(info) = self.terminate(id) {
                results.push(info);
            }
        }
        results
    }

    /// Return the number of currently running workloads.
    pub fn count(&self) -> usize {
        self.workloads.write().expect("workloads map poisoned").len()
    }

    /// Total workloads ever started since this registry was created.
    pub fn total_started(&self) -> u64 {
        self.total_started.load(Ordering::Relaxed)
    }

    /// Total workloads completed since this registry was created.
    pub fn total_completed(&self) -> u64 {
        self.total_completed.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_info(work_id: [u8; 32], pid: u32) -> RunningWorkloadInfo {
        RunningWorkloadInfo {
            work_id,
            pid,
            allocated_cores: 4,
            allocated_memory_bytes: 8 * 1024 * 1024 * 1024,
            bundle_path: std::path::PathBuf::from("/tmp/test"),
            cgroup_path: "/edgerun/test".to_string(),
        }
    }

    #[test]
    fn registry_new_is_empty() {
        let registry = RunningWorkloads::new();
        assert_eq!(registry.count(), 0);
        assert_eq!(registry.total_started(), 0);
        assert_eq!(registry.total_completed(), 0);
    }

    #[test]
    fn register_and_get() {
        let registry = RunningWorkloads::new();
        let info = make_info([0xAB; 32], 12345);
        assert!(registry.register(info).is_ok());
        assert_eq!(registry.count(), 1);
        assert_eq!(registry.total_started(), 1);

        let fetched = registry.get(&[0xAB; 32]).unwrap();
        assert_eq!(fetched.pid, 12345);
        assert_eq!(fetched.allocated_cores, 4);
    }

    #[test]
    fn duplicate_register_returns_err() {
        let registry = RunningWorkloads::new();
        let info = make_info([0xCD; 32], 11111);
        assert!(registry.register(info.clone()).is_ok());
        assert!(registry.register(info).is_err());
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn terminate_nonexistent_returns_none() {
        let registry = RunningWorkloads::new();
        assert!(registry.terminate(&[0u8; 32]).is_none());
    }

    #[test]
    fn terminate_signals_but_does_not_remove() {
        let registry = RunningWorkloads::new();
        let info = make_info([0xEF; 32], 99999);
        registry.register(info);
        assert_eq!(registry.count(), 1);

        // Signal termination (will fail to kill pid 99999 but still signal)
        let result = registry.terminate(&[0xEF; 32]);
        assert!(result.is_some());

        // Entry should still be in registry — background thread removes it
        assert!(registry.is_kill_requested(&[0xEF; 32]));
        assert_eq!(registry.count(), 1);

        // Now unregister (as background thread would)
        let unreg = registry.unregister(&[0xEF; 32]);
        assert!(unreg.is_some());
        assert_eq!(registry.count(), 0);
        assert_eq!(registry.total_completed(), 1);
    }

    #[test]
    fn unregister_nonexistent_returns_none() {
        let registry = RunningWorkloads::new();
        assert!(registry.unregister(&[0u8; 32]).is_none());
    }

    #[test]
    fn concurrent_register_unregister() {
        let registry = RunningWorkloads::new();
        let info = make_info([0x12; 32], 77777);
        assert!(registry.register(info).is_ok());

        // Signal kill
        assert!(registry.terminate(&[0x12; 32]).is_some());

        // Unregister (background thread's job)
        assert!(registry.unregister(&[0x12; 32]).is_some());

        // Should be gone
        assert!(registry.get(&[0x12; 32]).is_none());
        assert_eq!(registry.count(), 0);
    }
}
