//! Runtime core - Builder, Runtime, free spawn functions.

#![no_std]

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::{Context, Poll, Waker};

use crate::ready_queue::ReadyQueue;
use crate::waker::make_waker;

// ===========================================================================
// RuntimeInner
// ===========================================================================

pub struct RuntimeInner {
    pub queue: Arc<ReadyQueue>,
    pub shutdown: AtomicBool,
}

impl RuntimeInner {
    pub fn spawn_task<F>(&self, _f: F) -> crate::blocking_pool::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        crate::blocking_pool::JoinHandle::new_with_task(0, Arc::clone(&self.queue))
    }
}

// ===========================================================================
// Runtime
// ===========================================================================

pub struct Runtime {
    inner: Arc<RuntimeInner>,
}

impl Runtime {
    pub fn new_multi_thread() -> Builder {
        Builder::new_multi_thread()
    }

    pub fn spawn<F>(&self, f: F) -> crate::blocking_pool::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.inner.spawn_task(f)
    }

    pub fn block_on<F: Future + Unpin>(&self, f: F) -> F::Output {
        let w = Arc::new(ReadyQueue::new());
        let waker = make_waker(0, Arc::clone(&w));
        let mut cx = Context::from_waker(&waker);
        let mut f = f;
        loop {
            match Future::poll(Pin::new(&mut f), &mut cx) {
                Poll::Ready(v) => return v,
                Poll::Pending => {
                    if w.pop().is_some() {}
                }
            }
        }
    }

    pub fn shutdown(&self) {
        self.inner.shutdown.store(true, Ordering::Release);
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        self.shutdown();
    }
}

// ===========================================================================
// Builder
// ===========================================================================

pub struct Builder {
    workers: usize,
    blocking_workers: usize,
}

impl Builder {
    pub fn new_multi_thread() -> Self {
        Self {
            workers: 4,
            blocking_workers: 4,
        }
    }

    pub fn worker_threads(&mut self, n: usize) -> &mut Self {
        self.workers = n;
        self
    }

    pub fn max_blocking_threads(&mut self, n: usize) -> &mut Self {
        self.blocking_workers = n;
        self
    }

    pub fn enable_all(&mut self) -> &mut Self {
        self
    }

    pub fn build(&self) -> Result<Runtime, ()> {
        let queue = Arc::new(ReadyQueue::new());
        let rt = Arc::new(RuntimeInner {
            queue,
            shutdown: AtomicBool::new(false),
        });
        Ok(Runtime { inner: rt })
    }
}

// ===========================================================================
// Free spawn - stub (must have runtime passed)
// ===========================================================================

pub fn spawn<F>(_rt: &RuntimeHandle, _f: F) -> crate::blocking_pool::JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    todo!("spawn not implemented")
}

pub fn spawn_blocking<F, R, RT>(_rt: &RT, _f: F) -> crate::blocking_pool::JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    todo!("spawn_blocking not implemented")
}

pub struct RuntimeHandle {
    inner: Arc<RuntimeInner>,
}

impl RuntimeHandle {
    pub fn spawn<F>(&self, f: F) -> crate::blocking_pool::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.inner.spawn_task(f)
    }
}