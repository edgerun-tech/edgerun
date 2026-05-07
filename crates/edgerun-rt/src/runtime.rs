//! Runtime core - bare-metal async

extern crate alloc;
extern crate edgerun_platform;

use crate::Error;
use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

type TaskFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

static TASK_QUEUE: crate::sync::Mutex<VecDeque<TaskFuture>> =
    crate::sync::Mutex::new(VecDeque::new());
static PENDING_TASKS: AtomicUsize = AtomicUsize::new(0);
static RUN_COUNT: AtomicU32 = AtomicU32::new(0);

pub fn spawn<F>(f: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let handle = JoinHandle::pending();
    let state = handle.state.clone();
    enqueue_task(Box::pin(async move {
        if state.aborted.load(Ordering::Acquire) {
            let mut should_wake = false;
            let waiters = {
                let mut result = state.result.lock();
                if result.is_none() {
                    *result = Some(Err(JoinError));
                    should_wake = true;
                }
                if should_wake {
                    let mut waiters = state.waiters.lock();
                    Some(core::mem::take(&mut *waiters))
                } else {
                    None
                }
            };

            if should_wake {
                if let Some(waiters) = waiters {
                    for waker in waiters {
                        waker.wake();
                    }
                }
            }
            return;
        }

        let value = f.await;
        let mut result = state.result.lock();
        let mut should_wake = false;
        if state.aborted.load(Ordering::Acquire) {
            if result.is_none() {
                *result = Some(Err(JoinError));
                should_wake = true;
            }
        } else {
            if result.is_none() {
                *result = Some(Ok(value));
                should_wake = true;
            }
        }

        if should_wake {
            let waiters = {
                let mut waiters = state.waiters.lock();
                core::mem::take(&mut *waiters)
            };
            for waker in waiters {
                waker.wake();
            }
        }
    }));
    handle
}

pub fn spawn_local<F>(f: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    spawn(f)
}

pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let handle = JoinHandle::pending();
    let state = handle.state.clone();

    enqueue_task(Box::pin(async move {
        let value = f();
        let mut result = state.result.lock();
        let mut should_wake = false;
        if state.aborted.load(Ordering::Acquire) {
            if result.is_none() {
                *result = Some(Err(JoinError));
                should_wake = true;
            }
        } else if result.is_none() {
            *result = Some(Ok(value));
            should_wake = true;
        }

        if should_wake {
            let waiters = {
                let mut waiters = state.waiters.lock();
                core::mem::take(&mut *waiters)
            };
            for waker in waiters {
                waker.wake();
            }
        }
    }));
    handle
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
            Poll::Pending => {
                run_queue();
                unsafe { edgerun_platform::yield_cpu() };
            }
        }
    }
}

fn enqueue_task(task: TaskFuture) {
    TASK_QUEUE.lock().push_back(task);
    PENDING_TASKS.fetch_add(1, Ordering::Release);
}

pub fn run_queue() {
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    let mut tasks = {
        let mut queue = TASK_QUEUE.lock();
        let mut tasks = Vec::with_capacity(queue.len());

        for _ in 0..queue.len() {
            if let Some(task) = queue.pop_front() {
                tasks.push(task);
            } else {
                break;
            }
        }

        tasks
    };
    let mut completed_count = 0usize;
    let mut pending = Vec::new();

    for mut task in tasks.drain(..) {
        match task.as_mut().poll(&mut cx) {
            Poll::Ready(()) => {
                completed_count += 1;
            }
            Poll::Pending => {
                pending.push(task);
            }
        }
    }

    if !pending.is_empty() {
        TASK_QUEUE.lock().extend(pending);
    }

    if completed_count > 0 {
        PENDING_TASKS.fetch_sub(completed_count, Ordering::AcqRel);
    }

    RUN_COUNT.fetch_add(1, Ordering::AcqRel);
}

pub fn pending() -> usize {
    PENDING_TASKS.load(Ordering::Acquire)
}

pub fn runs() -> u32 {
    RUN_COUNT.load(Ordering::Acquire)
}

pub fn noop_waker() -> Waker {
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

pub struct Builder {
    worker_threads: Option<usize>,
    max_blocking_threads: Option<usize>,
}

impl Builder {
    pub fn new_multi_thread() -> Self {
        Self {
            worker_threads: None,
            max_blocking_threads: None,
        }
    }
    pub fn worker_threads(&mut self, threads: usize) -> &mut Self {
        self.worker_threads = Some(threads.max(1));
        self
    }
    pub fn max_blocking_threads(&mut self, threads: usize) -> &mut Self {
        self.max_blocking_threads = Some(threads.max(1));
        self
    }
    pub fn enable_all(&mut self) -> &mut Self {
        self
    }
    pub fn build(&self) -> Result<Runtime, Error> {
        Ok(Runtime::new(
            self.worker_threads.unwrap_or_else(default_worker_threads),
        ))
    }
}

pub struct Runtime;

fn default_worker_threads() -> usize {
    1
}

impl Runtime {
    fn new(worker_threads: usize) -> Self {
        let _ = worker_threads;
        Self
    }

    pub fn new_multi_thread() -> Builder {
        Builder::new_multi_thread()
    }
    pub fn worker_count(&self) -> usize {
        1
    }
    pub fn shutdown(&self) {}
    pub fn spawn<F>(&self, f: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        spawn(f)
    }
    pub fn spawn_local<F>(&self, f: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
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

crate::error::impl_error!(JoinError, |_this, f| { f.write_str("join error") });

struct JoinState<T> {
    result: crate::sync::Mutex<Option<Result<T, JoinError>>>,
    aborted: AtomicBool,
    waiters: crate::sync::Mutex<Vec<Waker>>,
}

pub struct JoinHandle<T> {
    state: Arc<JoinState<T>>,
    waker: Option<Waker>,
}

impl<T> Clone for JoinHandle<T> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            waker: None,
        }
    }
}

impl<T> JoinHandle<T> {
    fn pending() -> Self {
        Self {
            state: Arc::new(JoinState {
                result: crate::sync::Mutex::new(None),
                aborted: AtomicBool::new(false),
                waiters: crate::sync::Mutex::new(Vec::new()),
            }),
            waker: None,
        }
    }

    fn ready(result: Result<T, JoinError>) -> Self {
        Self {
            state: Arc::new(JoinState {
                result: crate::sync::Mutex::new(Some(result)),
                aborted: AtomicBool::new(false),
                waiters: crate::sync::Mutex::new(Vec::new()),
            }),
            waker: None,
        }
    }

    pub fn abort(&self) {
        self.state.aborted.store(true, Ordering::Release);
        let mut should_wake = false;
        let waiters = {
            let mut result = self.state.result.lock();
            if result.is_none() {
                *result = Some(Err(JoinError));
                should_wake = true;
            }
            if should_wake {
                let mut waiters = self.state.waiters.lock();
                Some(core::mem::take(&mut *waiters))
            } else {
                None
            }
        };

        if should_wake {
            if let Some(waiters) = waiters {
                for waker in waiters {
                    waker.wake();
                }
            }
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
            unsafe { edgerun_platform::yield_cpu() };
            core::hint::spin_loop();
        }
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = Result<T, JoinError>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if let Some(result) = this.state.result.lock().take() {
            if let Some(waker) = this.waker.take() {
                let mut waiters = this.state.waiters.lock();
                remove_waker(&mut waiters, &waker);
            }
            Poll::Ready(result)
        } else {
            let should_register = match this.waker.as_ref() {
                Some(registered) => !registered.will_wake(cx.waker()),
                None => true,
            };
            if should_register {
                let mut waiters = this.state.waiters.lock();
                if let Some(previous) = this.waker.replace(cx.waker().clone()) {
                    remove_waker(&mut waiters, &previous);
                }
                register_waker(
                    &mut waiters,
                    this.waker.as_ref().expect("registered wakeup"),
                );
            }
            Poll::Pending
        }
    }
}

impl<T> Drop for JoinHandle<T> {
    fn drop(&mut self) {
        if let Some(waker) = self.waker.take() {
            let mut waiters = self.state.waiters.lock();
            remove_waker(&mut waiters, &waker);
        }
    }
}

fn register_waker(waiters: &mut Vec<Waker>, waker: &Waker) {
    if !waiters.iter().any(|registered| registered.will_wake(waker)) {
        waiters.push(waker.clone());
    }
}

fn remove_waker(waiters: &mut Vec<Waker>, waker: &Waker) {
    if let Some(pos) = waiters
        .iter()
        .position(|registered| registered.will_wake(waker))
    {
        waiters.remove(pos);
    }
}

#[derive(Clone, Copy)]
pub struct RuntimeHandle;

impl RuntimeHandle {
    pub fn spawn<F>(&self, f: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
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
    tasks: crate::sync::Mutex<alloc::collections::VecDeque<JoinHandle<F>>>,
    closed: AtomicBool,
}

impl<F> JoinSet<F> {
    pub fn new() -> Self {
        Self {
            tasks: crate::sync::Mutex::new(alloc::collections::VecDeque::new()),
            closed: AtomicBool::new(false),
        }
    }

    pub fn spawn<F2>(&mut self, f: F2) -> Option<JoinHandle<F>>
    where
        F2: Future<Output = F> + Send + 'static,
        F: Send + 'static,
    {
        if self.closed.load(Ordering::Acquire) {
            return None;
        }
        let handle = spawn(f);
        self.tasks.lock().push_back(handle.clone());
        Some(handle)
    }

    pub fn abort(&self) {
        self.closed.store(true, Ordering::Release);
        let mut tasks = self.tasks.lock();
        for task in tasks.iter() {
            task.abort();
        }
    }

    pub fn len(&self) -> usize {
        let mut tasks = self.tasks.lock();
        tasks.retain(|task| !task.is_finished());
        tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<F> Default for JoinSet<F> {
    fn default() -> Self {
        Self::new()
    }
}
