//! Thread-safe ready queue for task scheduling.
//!
//! Workers call `pop()` to get the next task ID to poll.
//! Wakers call `push(id)` to schedule a task for polling.


extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use crate::sync_prim::{Condvar, Mutex};
use core::sync::atomic::{AtomicBool, Ordering};

// ===========================================================================
// Ready Queue
// ===========================================================================

struct ReadyQueueInner {
    q: Mutex<Vec<usize>>,
    cvar: Condvar,
    done: AtomicBool,
}

/// Thread-safe ready queue. Workers `pop()`, wakers `push()`.
pub struct ReadyQueue {
    inner: Arc<ReadyQueueInner>,
}

impl ReadyQueue {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(ReadyQueueInner {
                q: Mutex::new(Vec::new()),
                cvar: Condvar::new(),
                done: AtomicBool::new(false),
            }),
        }
    }

    pub fn push(&self, id: usize) {
        let mut q = self.inner.q.lock();
        q.push(id);
        self.inner.cvar.notify_one();
    }

    pub fn pop(&self) -> Option<usize> {
        let mut q = self.inner.q.lock();
        loop {
            if let Some(id) = q.pop() {
                return Some(id);
            }
            if self.inner.done.load(Ordering::Acquire) {
                return None;
            }
            self.inner.cvar.wait(&mut q);
        }
    }

    pub fn try_pop(&self) -> Option<usize> {
        self.inner.q.lock().pop()
    }

    pub fn shutdown(&self) {
        self.inner.done.store(true, Ordering::Release);
        self.inner.cvar.notify_all();
    }

    pub fn len(&self) -> usize {
        self.inner.q.lock().len()
    }
}