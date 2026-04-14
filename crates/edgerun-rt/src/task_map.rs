//! Task storage — maps task IDs to poll functions.
//!
//! Workers `take_for_poll(id)` → poll → `reinsert(id)` if pending,
//! or drop if complete.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use crate::sync::Mutex;
use std::task::Context;

type PollFn = Box<dyn FnMut(&mut Context<'_>) -> bool + Send>;
/// Callback invoked when a task panics. Sets the JoinHandle result to Err(JoinError).
type PanicFn = Box<dyn FnOnce() + Send>;

struct TaskMapInner {
    map: HashMap<usize, PollFn>,
    /// Panic notifiers — called when a task panics so the JoinHandle can be resolved.
    panic_fns: HashMap<usize, PanicFn>,
}

pub(crate) struct TaskMap {
    inner: Mutex<TaskMapInner>,
    next: AtomicUsize,
}

impl TaskMap {
    pub(crate) fn new() -> Self {
        Self {
            inner: Mutex::new(TaskMapInner {
                map: HashMap::new(),
                panic_fns: HashMap::new(),
            }),
            next: AtomicUsize::new(1),
        }
    }

    /// Reserve the next available task ID.
    /// The caller must use this ID with `insert_with_id` to insert the task.
    pub(crate) fn next_id(&self) -> usize {
        self.next.fetch_add(1, Ordering::Relaxed)
    }

    /// Insert a task with a pre-reserved ID (from `next_id`).
    pub(crate) fn insert_with_id(&self, id: usize, f: PollFn) {
        self.inner.lock().map.insert(id, f);
    }

    /// Insert a task with a pre-reserved ID AND a panic notifier.
    /// The panic notifier is called by the worker when a task panics,
    /// so the JoinHandle can be resolved with Err(JoinError).
    pub(crate) fn insert_with_panic(&self, id: usize, f: PollFn, panic_fn: PanicFn) {
        let mut guard = self.inner.lock();
        guard.map.insert(id, f);
        guard.panic_fns.insert(id, panic_fn);
    }

    /// When a task panics, take and execute the panic notifier.
    pub(crate) fn take_panic_fn(&self, id: usize) -> Option<PanicFn> {
        self.inner.lock().panic_fns.remove(&id)
    }

    pub(crate) fn insert(&self, f: PollFn) -> usize {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        self.inner.lock().map.insert(id, f);
        id
    }

    /// Take the poll function out for polling (lock not held during poll).
    /// Caller must `reinsert` if the task is still pending.
    pub(crate) fn take_for_poll(&self, id: usize) -> Option<PollFn> {
        self.inner.lock().map.remove(&id)
    }

    pub(crate) fn reinsert(&self, id: usize, f: PollFn) {
        self.inner.lock().map.insert(id, f);
    }

    /// Number of tasks currently stored in the map.
    pub(crate) fn len(&self) -> usize {
        self.inner.lock().map.len()
    }

    /// Non-blocking length check. Returns None if the lock is contended.
    pub(crate) fn try_len(&self) -> Option<usize> {
        self.inner.try_lock().map(|g| g.map.len())
    }
}
