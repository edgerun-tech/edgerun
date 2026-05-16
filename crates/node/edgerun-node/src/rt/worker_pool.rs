//! Worker pool for parallel task execution

extern crate edgerun_platform;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use core::sync::atomic::{AtomicBool, Ordering};

use crate::rt::metrics::Metrics;
use crate::rt::sync::Mutex;

pub struct WorkerPool {
    workers: usize,
    queue: Mutex<Vec<usize>>,
    shutdown: AtomicBool,
    metrics: Metrics,
}

impl WorkerPool {
    pub fn new(workers: usize) -> Self {
        Self {
            workers: workers.max(1),
            queue: Mutex::new(Vec::new()),
            shutdown: AtomicBool::new(false),
            metrics: Metrics::new(),
        }
    }

    pub fn spawn<F>(&self, _f: F)
    where
        F: Future + Send + 'static,
    {
        if self.shutdown.load(Ordering::Acquire) {
            return;
        }
        self.metrics.spawn_task();
    }

    pub fn run(&self) {
        let waker = unsafe {
            edgerun_platform::waker::make_ipi_waker(edgerun_platform::this_cpu())
        };
        let mut cx = Context::from_waker(&waker);
        let start = edgerun_platform::timer::timer_ticks();
        loop {
            if self.shutdown.load(Ordering::Acquire) {
                break;
            }
            let mut queue = self.queue.lock();
            if let Some(ptr) = queue.pop() {
                drop(queue);
                self.metrics.complete_task();
            } else {
                drop(queue);
                unsafe { edgerun_platform::yield_cpu(); }
            }
        }
        let elapsed = edgerun_platform::timer::timer_ticks() - start;
        self.metrics.add_cpu_time(elapsed);
    }

    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
    }

    pub fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    pub fn worker_count(&self) -> usize {
        self.workers
    }
}

impl Default for WorkerPool {
    fn default() -> Self {
        Self::new(1)
    }
}