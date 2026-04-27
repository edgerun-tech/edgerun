//! Blocking thread pool for CPU-intensive tasks

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::boxed::Box;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::sync::Mutex;

pub struct BlockingPool {
    workers: usize,
    queue: Arc<Mutex<Vec<TaskEntry>>>,
    active: AtomicUsize,
    shutdown: AtomicUsize,
}

struct TaskEntry {
    task: Box<dyn FnOnce() + Send>,
}

impl BlockingPool {
    pub fn new(workers: usize) -> Self {
        Self {
            workers,
            queue: Arc::new(Mutex::new(Vec::new())),
            active: AtomicUsize::new(0),
            shutdown: AtomicUsize::new(0),
        }
    }

    pub fn spawn<T: FnOnce() + Send + 'static>(&self, task: T) {
        if self.shutdown.load(Ordering::Acquire) != 0 {
            return;
        }
        self.queue.lock().push(TaskEntry { task: Box::new(task) });
    }

    pub fn worker_count(&self) -> usize {
        self.workers
    }

    pub fn shutdown(&mut self) {
        self.shutdown.store(1, Ordering::Release);
    }
}

impl Drop for BlockingPool {
    fn drop(&mut self) {
        self.shutdown();
    }
}