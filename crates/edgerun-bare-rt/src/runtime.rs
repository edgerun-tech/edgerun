//! Runtime core - bare-metal async

extern crate edgerun_platform;
extern crate alloc;

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use core::sync::atomic::{AtomicBool, Ordering};

pub fn spawn<F>(_f: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
{
    JoinHandle { _phantom: core::marker::PhantomData }
}

pub fn spawn_local<F>(f: F) -> JoinHandle<F::Output>
where
    F: Future + 'static,
{
    JoinHandle { _phantom: core::marker::PhantomData }
}

pub fn block_on<F>(f: F) -> F::Output
where
    F: Unpin + Future,
{
    let waker = unsafe {
        edgerun_platform::waker::make_ipi_waker(edgerun_platform::this_cpu())
    };
    let mut cx = Context::from_waker(&waker);
    let mut f = f;
    loop {
        match Pin::new(&mut f).poll(&mut cx) {
            Poll::Ready(v) => return v,
            Poll::Pending => unsafe { edgerun_platform::yield_cpu(); },
        }
    }
}

pub fn shutdown() {}

pub struct Builder;

impl Builder {
    pub fn new_multi_thread() -> Self { Self }
    pub fn worker_threads(&mut self, _: usize) -> &mut Self { self }
    pub fn max_blocking_threads(&mut self, _: usize) -> &mut Self { self }
    pub fn build(&self) -> Runtime { Runtime }
}

pub struct Runtime;

impl Runtime {
    pub fn new_multi_thread() -> Self { Self }
    pub fn shutdown(&self) {}
    pub fn spawn<F>(&self, f: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
    {
        spawn(f)
    }
    pub fn spawn_local<F>(&self, f: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
    {
        spawn_local(f)
    }
    pub fn block_on<F>(&self, f: F) -> F::Output
    where
        F: Unpin + Future,
    {
        block_on(f)
    }
}

impl Drop for Runtime {
    fn drop(&mut self) { self.shutdown(); }
}

pub struct JoinHandle<T> {
    _phantom: core::marker::PhantomData<T>,
}

impl<T> Clone for JoinHandle<T> {
    fn clone(&self) -> Self {
        Self { _phantom: core::marker::PhantomData }
    }
}

impl<T> JoinHandle<T> {
    pub fn abort(&self) {}
}

impl<T> Future for JoinHandle<T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct JoinSet<F> {
    _phantom: core::marker::PhantomData<F>,
    closed: AtomicBool,
}

impl<F> JoinSet<F> {
    pub fn new() -> Self {
        Self { _phantom: core::marker::PhantomData, closed: AtomicBool::new(false) }
    }

    pub fn spawn(&mut self, _f: F) -> Option<JoinHandle<F>>
    where
        F: Future + Send + 'static,
    {
        if self.closed.load(Ordering::Acquire) {
            return None;
        }
        Some(JoinHandle { _phantom: core::marker::PhantomData })
    }

    pub fn abort(&self) {
        self.closed.store(true, Ordering::Release);
    }

    pub fn len(&self) -> usize {
        0
    }
}

impl<F> Default for JoinSet<F> {
    fn default() -> Self {
        Self::new()
    }
}