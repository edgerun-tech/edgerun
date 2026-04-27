//! Runtime core - bare-metal async

extern crate alloc;
extern crate edgerun_platform;

use crate::Error;
use alloc::sync::Arc;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

pub fn spawn<F>(_f: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
{
    JoinHandle::pending()
}

pub fn spawn_local<F>(_f: F) -> JoinHandle<F::Output>
where
    F: Future + 'static,
{
    JoinHandle::pending()
}

pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    JoinHandle::ready(Ok(f()))
}

pub fn block_on<F>(f: F) -> F::Output
where
    F: Future,
{
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    let mut f = alloc::boxed::Box::pin(f);
    loop {
        match f.as_mut().poll(&mut cx) {
            Poll::Ready(v) => return v,
            Poll::Pending => unsafe {
                edgerun_platform::yield_cpu();
            },
        }
    }
}

fn noop_waker() -> Waker {
    unsafe fn clone(_: *const ()) -> RawWaker {
        RawWaker::new(core::ptr::null(), &NOOP_WAKER_VTABLE)
    }
    unsafe fn wake(_: *const ()) {}
    unsafe fn wake_by_ref(_: *const ()) {}
    unsafe fn drop(_: *const ()) {}

    static NOOP_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);

    unsafe { Waker::from_raw(RawWaker::new(core::ptr::null(), &NOOP_WAKER_VTABLE)) }
}

pub fn shutdown() {}

pub struct Builder;

impl Builder {
    pub fn new_multi_thread() -> Self {
        Self
    }
    pub fn worker_threads(&mut self, _: usize) -> &mut Self {
        self
    }
    pub fn max_blocking_threads(&mut self, _: usize) -> &mut Self {
        self
    }
    pub fn enable_all(&mut self) -> &mut Self {
        self
    }
    pub fn build(&self) -> Result<Runtime, Error> {
        Ok(Runtime)
    }
}

pub struct Runtime;

impl Runtime {
    pub fn new_multi_thread() -> Builder {
        Builder::new_multi_thread()
    }
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
    pub fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        spawn_blocking(f)
    }
    pub fn block_on<F>(&self, f: F) -> F::Output
    where
        F: Future,
    {
        block_on(f)
    }
    pub fn handle(&self) -> RuntimeHandle {
        RuntimeHandle
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[derive(Debug, Clone, Copy)]
pub struct JoinError;

impl core::fmt::Display for JoinError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("join error")
    }
}

impl core::error::Error for JoinError {}

struct JoinState<T> {
    result: crate::sync::Mutex<Option<Result<T, JoinError>>>,
    aborted: AtomicBool,
}

pub struct JoinHandle<T> {
    state: Arc<JoinState<T>>,
}

impl<T> Clone for JoinHandle<T> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

impl<T> JoinHandle<T> {
    fn pending() -> Self {
        Self {
            state: Arc::new(JoinState {
                result: crate::sync::Mutex::new(None),
                aborted: AtomicBool::new(false),
            }),
        }
    }

    fn ready(result: Result<T, JoinError>) -> Self {
        Self {
            state: Arc::new(JoinState {
                result: crate::sync::Mutex::new(Some(result)),
                aborted: AtomicBool::new(false),
            }),
        }
    }

    pub fn abort(&self) {
        self.state.aborted.store(true, Ordering::Release);
        let mut result = self.state.result.lock();
        if result.is_none() {
            *result = Some(Err(JoinError));
        }
    }

    pub fn is_aborted(&self) -> bool {
        self.state.aborted.load(Ordering::Acquire)
    }

    pub fn is_finished(&self) -> bool {
        self.state.result.lock().is_some()
    }

    pub fn blocking_recv(self) -> Result<T, JoinError> {
        loop {
            if let Some(result) = self.state.result.lock().take() {
                return result;
            }
            core::hint::spin_loop();
        }
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = Result<T, JoinError>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(result) = self.state.result.lock().take() {
            Poll::Ready(result)
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

#[derive(Clone, Copy)]
pub struct RuntimeHandle;

impl RuntimeHandle {
    pub fn spawn<F>(&self, f: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
    {
        spawn(f)
    }

    pub fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        spawn_blocking(f)
    }
}

pub struct JoinSet<F> {
    _phantom: core::marker::PhantomData<F>,
    closed: AtomicBool,
}

impl<F> JoinSet<F> {
    pub fn new() -> Self {
        Self {
            _phantom: core::marker::PhantomData,
            closed: AtomicBool::new(false),
        }
    }

    pub fn spawn(&mut self, _f: F) -> Option<JoinHandle<F>>
    where
        F: Future + Send + 'static,
    {
        if self.closed.load(Ordering::Acquire) {
            return None;
        }
        Some(JoinHandle::pending())
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
