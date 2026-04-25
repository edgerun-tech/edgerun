//! Runtime core - Builder, Runtime, free spawn functions.


extern crate alloc;

use alloc::sync::Arc;
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
    pub worker_count: AtomicUsize,
    pub waker: Waker,
}

impl RuntimeInner {
    pub fn new(queue: Arc<ReadyQueue>, workers: usize) -> Self {
        let waker = make_waker(0, queue.clone());
        Self {
            queue,
            shutdown: AtomicBool::new(false),
            worker_count: AtomicUsize::new(workers),
            waker,
        }
    }

    #[allow(dead_code)]
    pub fn spawn_task<F>(&self, _f: F) -> crate::blocking_pool::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
    {
        crate::blocking_pool::JoinHandle::new_with_task(0, Arc::clone(&self.queue))
    }
    
    #[allow(dead_code)]
    pub fn run_once(&self) -> bool {
        if self.shutdown.load(Ordering::Acquire) {
            return false;
        }
        self.queue.try_pop().is_some()
    }
}

pub struct Runtime {
    inner: Arc<RuntimeInner>,
}

impl Runtime {
    pub fn new_multi_thread() -> Builder {
        Builder::new_multi_thread()
    }

    #[allow(dead_code)]
    pub fn spawn<F>(&self, _f: F) -> crate::blocking_pool::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
    {
        self.inner.spawn_task(_f)
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
                    core::hint::spin_loop();
                }
            }
        }
    }

    pub fn shutdown(&self) {
        self.inner.shutdown.store(true, Ordering::Release);
    }
    
    pub fn inner(&self) -> &Arc<RuntimeInner> {
        &self.inner
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

#[allow(dead_code)]
pub fn spawn<F>(f: F) -> crate::blocking_pool::JoinHandle<F::Output>
where
    F: Future + Send + 'static,
{
    if let Some(rt) = current_rt() {
        rt.spawn_task(f)
    } else {
        panic!("spawn() called without a Runtime")
    }
}

#[allow(dead_code)]
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
    #[allow(dead_code)]
    pub fn spawn<F>(&self, f: F) -> crate::blocking_pool::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
    {
        self.inner.spawn_task(f)
    }
}