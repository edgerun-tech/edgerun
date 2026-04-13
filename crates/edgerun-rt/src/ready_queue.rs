//! Thread-safe ready queue for task scheduling.
//!
//! Workers call `pop()` to get the next task ID to poll.
//! Wakers call `push(id)` to schedule a task for polling.
//!
//! Uses our `sync::Condvar` for efficient waiting (no spin loops).

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crate::sync::{Condvar, Mutex};

struct ReadyQueueInner {
    q: Mutex<VecDeque<usize>>,
    cvar: Condvar,
    done: AtomicBool,
}

/// Thread-safe ready queue. Workers `pop()`, wakers `push()`.
pub(crate) struct ReadyQueue {
    inner: Arc<ReadyQueueInner>,
}

impl ReadyQueue {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(ReadyQueueInner {
                q: Mutex::new(VecDeque::new()),
                cvar: Condvar::new(),
                done: AtomicBool::new(false),
            }),
        }
    }

    pub(crate) fn push(&self, id: usize) {
        let mut q = self.inner.q.lock();
        q.push_back(id);
        self.inner.cvar.notify_one();
    }

    pub(crate) fn pop(&self) -> Option<usize> {
        let mut q = self.inner.q.lock();
        loop {
            if let Some(id) = q.pop_front() {
                return Some(id);
            }
            if self.inner.done.load(Ordering::Acquire) {
                return None;
            }
            // Use infinite wait — workers are only woken when there's work
            // or shutdown is requested. No periodic wake overhead.
            self.inner.cvar.wait(&mut q);
        }
    }

    pub(crate) fn shutdown(&self) {
        self.inner.done.store(true, Ordering::Release);
        self.inner.cvar.notify_all();
    }

    /// Number of tasks currently queued and ready to be polled.
    pub(crate) fn len(&self) -> usize {
        self.inner.q.lock().len()
    }
}
