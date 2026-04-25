//! Runtime core - Builder, Runtime, free spawn functions.

#![no_std]

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use core::task::{Context, Poll, Waker};

use crate::ready_queue::ReadyQueue;
use crate::waker::make_waker;

// ===========================================================================
// RuntimeInner
// ===========================================================================

pub struct RuntimeInner {
    pub queue: Arc<ReadyQueue>,
    pub shutdown: AtomicBool,
    worker_count: AtomicUsize,
}

impl RuntimeInner {
    pub fn new(queue: Arc<ReadyQueue>, workers: usize) -> Self {
        Self {
            queue,
            shutdown: AtomicBool::new(false),
            worker_count: AtomicUsize::new(workers),
        }
    }

    pub fn spawn_task<F>(&self, f: F) -> crate::blocking_pool::JoinHandle<F::Output>
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

    pub fn block_on<F>(&self, f: F) -> F::Output
    where
        F: Future + Unpin,
    {
        let w = Arc::new(ReadyQueue::new());
        let waker = make_waker(0, w);
        let mut cx = Context::from_waker(&waker);
        let mut f = f;
        loop {
            match Pin::new(&mut f).poll(&mut cx) {
                Poll::Ready(v) => return v,
                Poll::Pending => {
                    // In no_std, we can't block - spin
                    core::hint::spin_loop();
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
        let rt = Arc::new(RuntimeInner::new(queue, self.workers));
        Ok(Runtime { inner: rt })
    }
}

// ===========================================================================
// Free spawn functions
// ===========================================================================

pub fn spawn<F>(_f: F) -> crate::blocking_pool::JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    todo!("spawn requires runtime context")
}

pub fn spawn_blocking<F, R>(f: F) -> R
where
    F: FnOnce() -> R + Send,
    R: Send + 'static,
{
    f()
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