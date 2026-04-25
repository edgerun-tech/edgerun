//! Runtime core - Builder, Runtime, free spawn functions.


extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::boxed::Box;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use core::task::{Context, Poll, Waker};

use crate::ready_queue::ReadyQueue;
use crate::waker::make_waker;

static mut CURRENT_RT: Option<Arc<RuntimeInner>> = None;

fn current_rt() -> Option<&'static Arc<RuntimeInner>> {
    unsafe { CURRENT_RT.as_ref() }
}

fn set_current_rt(rt: Option<Arc<RuntimeInner>>) {
    unsafe { CURRENT_RT = rt; }
}

pub struct RuntimeInner {
    pub queue: Arc<ReadyQueue>,
    pub shutdown: AtomicBool,
    worker_count: AtomicUsize,
    next_id: AtomicUsize,
    waker: Waker,
}

impl RuntimeInner {
    pub fn new(queue: Arc<ReadyQueue>, workers: usize) -> Self {
        let waker = make_waker(0, queue.clone());
        Self {
            queue,
            shutdown: AtomicBool::new(false),
            worker_count: AtomicUsize::new(workers),
            next_id: AtomicUsize::new(0),
            waker,
        }
    }

    pub fn spawn_task<F>(&self, f: F) -> crate::blocking_pool::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let task_id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.queue.push(task_id);
        crate::blocking_pool::JoinHandle::new_with_task(task_id, Arc::clone(&self.queue))
    }
    
    pub fn run_once(&self) -> bool {
        if self.shutdown.load(Ordering::Acquire) {
            return false;
        }
        
        if let Some(id) = self.queue.try_pop() {
            // Poll the task - simplified for no_std
            // In a real implementation we'd get the task from a map and poll it
            true
        } else {
            false
        }
    }
}

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
        let mut cx = Context::from_waker(&self.inner.waker);
        let mut f = f;
        loop {
            match Pin::new(&mut f).poll(&mut cx) {
                Poll::Ready(v) => return v,
                Poll::Pending => {
                    // Run pending tasks from queue
                    while self.inner.run_once() {}
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
        set_current_rt(Some(rt.clone()));
        Ok(Runtime { inner: rt })
    }
}

pub fn spawn<F>(f: F) -> crate::blocking_pool::JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    if let Some(rt) = current_rt() {
        rt.spawn_task(f)
    } else {
        panic!("spawn() called without a Runtime")
    }
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