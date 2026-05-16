//! I/O scheduler for async I/O operations


use alloc::sync::Arc;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use core::sync::atomic::AtomicBool;

pub struct IoScheduler;

impl IoScheduler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for IoScheduler {
    fn default() -> Self {
        Self::new()
    }
}

pub struct IoFuture {
    ready: AtomicBool,
}

impl IoFuture {
    pub fn new() -> Self {
        Self { ready: AtomicBool::new(false) }
    }

    pub fn set_ready(&self) {
        self.ready.store(true, core::sync::atomic::Ordering::Release);
    }
}

impl Future for IoFuture {
    type Output = ();
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        if self.ready.load(core::sync::atomic::Ordering::Acquire) {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

impl Default for IoFuture {
    fn default() -> Self {
        Self::new()
    }
}