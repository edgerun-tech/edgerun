//! Executor - task execution runtime

extern crate edgerun_platform;

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::metrics::Metrics;
use edgerun_platform::timer;

pub struct LocalExecutor {
    head: AtomicUsize,
    tail: AtomicUsize,
    shutdown: AtomicBool,
    metrics: Metrics,
}

impl LocalExecutor {
    pub fn new() -> Self {
        Self {
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            shutdown: AtomicBool::new(false),
            metrics: Metrics::new(),
        }
    }

    pub fn spawn<F>(&self, _f: F) -> LocalHandle<F::Output>
    where
        F: Future + 'static,
    {
        self.metrics.spawn_task();
        LocalHandle { _phantom: core::marker::PhantomData }
    }

    pub fn run(&self) {
        let waker = unsafe {
            edgerun_platform::waker::make_ipi_waker(edgerun_platform::this_cpu())
        };
        let mut cx = Context::from_waker(&waker);
        let start = timer::timer_ticks();
        loop {
            if self.shutdown.load(Ordering::Acquire) {
                break;
            }
            let tail = self.tail.load(Ordering::Acquire);
            let head = self.head.load(Ordering::Acquire);
            if tail == head {
                unsafe { edgerun_platform::yield_cpu(); }
                continue;
            }
            let next = head.wrapping_add(1);
            self.head.store(next, Ordering::Release);
            self.metrics.complete_task();
        }
        let elapsed = timer::timer_ticks() - start;
        self.metrics.add_cpu_time(elapsed);
    }

    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
    }

    pub fn metrics(&self) -> &Metrics {
        &self.metrics
    }
}

impl Default for LocalExecutor {
    fn default() -> Self {
        Self::new()
    }
}

pub struct LocalHandle<T> {
    _phantom: core::marker::PhantomData<T>,
}

impl<T> Clone for LocalHandle<T> {
    fn clone(&self) -> Self {
        Self { _phantom: core::marker::PhantomData }
    }
}

impl<T> LocalHandle<T> {
    pub fn abort(&self) {}
}

impl<T> Future for LocalHandle<T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}