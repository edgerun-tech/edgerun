//! Per-CPU task queue

extern crate alloc;

use alloc::sync::Arc;
use alloc::collections::VecDeque;
use core::sync::atomic::{AtomicUsize, Ordering};

const MAX_TASKS: usize = 256;

pub struct Task {
    pub future: core::cell::UnsafeCell<usize>,
    pub state: AtomicUsize,
    pub next: AtomicUsize,
}

impl Task {
    pub fn new() -> Self {
        Self {
            future: core::cell::UnsafeCell::new(0),
            state: AtomicUsize::new(0),
            next: AtomicUsize::new(0),
        }
    }

    pub unsafe fn set_future(&self, f: *const ()) {
        *self.future.get() = f as usize;
    }

    pub fn wake_task(&self) {
        self.state.fetch_or(1, Ordering::Release);
    }
}

pub struct TaskQueue {
    queue: VecDeque<Arc<Task>>,
    lock: AtomicUsize,
}

impl TaskQueue {
    pub const fn new() -> Self {
        Self { queue: VecDeque::new(), lock: AtomicUsize::new(0) }
    }

    pub fn enqueue(&mut self, task: Arc<Task>) {
        if self.queue.len() >= MAX_TASKS {
            return;
        }
        while self.lock.compare_exchange(0, 1, Ordering::Acquire, Ordering::Acquire).is_err() {
            core::hint::spin_loop();
        }
        self.queue.push_back(task);
        self.lock.store(0, Ordering::Release);
    }

    pub fn dequeue(&mut self) -> Option<Arc<Task>> {
        while self.lock.compare_exchange(0, 1, Ordering::Acquire, Ordering::Acquire).is_err() {
            core::hint::spin_loop();
        }
        let task = self.queue.pop_front();
        self.lock.store(0, Ordering::Release);
        task
    }

    pub fn try_dequeue(&mut self) -> Option<Arc<Task>> {
        if self.lock.load(Ordering::Acquire) != 0 {
            return None;
        }
        self.dequeue()
    }
}

impl Default for TaskQueue {
    fn default() -> Self {
        Self::new()
    }
}