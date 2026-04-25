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
use crate::sync_prim::{Condvar, Mutex};
use crate::waker::make_waker;

// ===========================================================================
// Thread-local
// ===========================================================================

thread_local! {
    static CURRENT_RT: UnsafeCell<Option<Arc<RuntimeInner>>> = UnsafeCell::new(None);
}

pub(crate) fn set_current_rt(rt: Arc<RuntimeInner>) {
    CURRENT_RT.with(|c| unsafe { *c.get() = Some(rt) });
}

pub(crate) fn current_rt() -> Arc<RuntimeInner> {
    CURRENT_RT.with(|c| unsafe { (*c.get()).clone() })
        .expect("no runtime: spawn() must be called from within a runtime")
}

// ===========================================================================
// RuntimeInner
// ===========================================================================

pub(crate) struct RuntimeInner {
    queue: Arc<ReadyQueue>,
    shutdown: AtomicBool,
}

impl RuntimeInner {
    fn spawn_task<F>(&self, f: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let task_id = 0; // TODO: proper task ID
        let handle = JoinHandle::new_with_task(task_id, Arc::clone(&self.queue));
        // TODO: actual task spawning
        handle
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

    pub fn spawn<F>(&self, f: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.inner.spawn_task(f)
    }

    pub fn block_on<F>(&self, f: F) -> F::Output
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        set_current_rt(self.inner.clone());
        // Simple block_on - run until complete
        let mut f = f;
        let w = Arc::new(ReadyQueue::new());
        let cx = Context::from_waker(&make_waker(0, w));
        loop {
            match Pin::new(&mut f).poll(&mut cx) {
                Poll::Ready(v) => return v,
                Poll::Pending => {
                    // Process one task
                    if let Some(_id) = w.pop() {
                        // Poll the task
                    } else {
                        core::hint::spin_loop();
                    }
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

    pub fn build(&self) -> Result<Runtime, std::io::Error> {
        let queue = Arc::new(ReadyQueue::new());
        let rt = Arc::new(RuntimeInner {
            queue,
            shutdown: AtomicBool::new(false),
        });
        set_current_rt(rt.clone());
        Ok(Runtime { inner: rt })
    }
}

// ===========================================================================
// Free spawn functions
// ===========================================================================

pub fn spawn<F>(f: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let rt = current_rt();
    rt.spawn_task(f)
}

pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    todo!("spawn_blocking not yet implemented")
}

pub struct RuntimeHandle {
    inner: Arc<RuntimeInner>,
}

impl RuntimeHandle {
    pub fn spawn<F>(&self, f: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.inner.spawn_task(f)
    }
}