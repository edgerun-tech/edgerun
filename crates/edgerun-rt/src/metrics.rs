//! Runtime metrics — snapshot of runtime state for observability.
//!
//! Call `Runtime::metrics()` or `RuntimeHandle::metrics()` to get a snapshot.
//! All fields are sampled at the time of the call (not atomic-live).

use std::sync::atomic::Ordering;

/// A point-in-time snapshot of runtime metrics.
///
/// # Example
/// ```ignore
/// let rt = Builder::new_multi_thread().build()?;
/// let m = rt.metrics();
/// println!("tasks: {} active, {} queued", m.active_tasks(), m.queued_tasks());
/// ```
#[derive(Debug, Clone)]
pub struct RuntimeMetrics {
    active_tasks: usize,
    queued_tasks: usize,
    total_spawned: u64,
    total_completed: u64,
    total_aborted: u64,
    blocking_threads: usize,
    blocking_active: usize,
}

impl RuntimeMetrics {
    /// Number of tasks currently in-flight (stored in the task map,
    /// either being polled or waiting for I/O/timer).
    #[inline]
    pub fn active_tasks(&self) -> usize {
        self.active_tasks
    }

    /// Number of tasks currently waiting in the ready queue to be polled.
    #[inline]
    pub fn queued_tasks(&self) -> usize {
        self.queued_tasks
    }

    /// Total number of async tasks spawned since runtime creation.
    #[inline]
    pub fn total_spawned(&self) -> u64 {
        self.total_spawned
    }

    /// Total number of async tasks that have completed (successfully or panicked).
    #[inline]
    pub fn total_completed(&self) -> u64 {
        self.total_completed
    }

    /// Total number of async tasks that were aborted via `JoinHandle::abort()`.
    #[inline]
    pub fn total_aborted(&self) -> u64 {
        self.total_aborted
    }

    /// Number of worker threads in the blocking pool.
    #[inline]
    pub fn blocking_threads(&self) -> usize {
        self.blocking_threads
    }

    /// Estimated number of blocking tasks currently running.
    #[inline]
    pub fn blocking_active(&self) -> usize {
        self.blocking_active
    }
}

// ===========================================================================
// Internal metrics storage — shared atomics for counters.
// ===========================================================================

/// Internal metrics, stored in `RuntimeInner` and updated by the runtime.
pub(crate) struct Metrics {
    pub(crate) total_spawned: std::sync::atomic::AtomicU64,
    pub(crate) total_completed: std::sync::atomic::AtomicU64,
    pub(crate) total_aborted: std::sync::atomic::AtomicU64,
}

impl Metrics {
    pub(crate) fn new() -> Self {
        Self {
            total_spawned: std::sync::atomic::AtomicU64::new(0),
            total_completed: std::sync::atomic::AtomicU64::new(0),
            total_aborted: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Take a snapshot, combining atomic counters with live state from
    /// the task map, queue, and blocking pool.
    pub(crate) fn snapshot(
        &self,
        task_map_len: usize,
        queue_len: usize,
        blocking_threads: usize,
        blocking_active: usize,
    ) -> RuntimeMetrics {
        RuntimeMetrics {
            active_tasks: task_map_len,
            queued_tasks: queue_len,
            total_spawned: self.total_spawned.load(Ordering::Relaxed),
            total_completed: self.total_completed.load(Ordering::Relaxed),
            total_aborted: self.total_aborted.load(Ordering::Relaxed),
            blocking_threads,
            blocking_active,
        }
    }
}
