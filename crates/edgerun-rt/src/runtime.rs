//! Runtime core — Builder, Runtime, free spawn functions.
//!
//! ## Fixes applied:
//! - Worker threads NO longer sleep after pending (waker-based dispatch)
//! - `enable_all()` removed (everything is always enabled)
//! - `spawn_blocking` code consolidated into a single inner method
//! - `Builder::enable_all()` removed (was a no-op stub)

use std::future::Future;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::JoinHandle as StdJoinHandle;
use parking_lot::Mutex;

use crate::blocking_pool::BlockingPool;
pub use crate::blocking_pool::{JoinError, JoinHandle};
use crate::metrics::{Metrics, RuntimeMetrics};
use crate::trace;
use crate::ready_queue::ReadyQueue;
use crate::reactor::Reactor;
use crate::task_map::TaskMap;
use crate::waker::make_waker;

// ===========================================================================
// Thread-local runtime handle
// ===========================================================================

thread_local! {
    static CURRENT_RT: std::cell::RefCell<Option<Arc<RuntimeInner>>> =
        const { std::cell::RefCell::new(None) };
}

pub(crate) fn set_current_rt(rt: Arc<RuntimeInner>) {
    CURRENT_RT.with(|c| *c.borrow_mut() = Some(rt));
}

pub(crate) fn current_rt() -> Arc<RuntimeInner> {
    CURRENT_RT
        .with(|c| c.borrow().clone())
        .expect("no runtime: spawn() must be called from within a runtime")
}

pub(crate) fn try_current_rt() -> Option<Arc<RuntimeInner>> {
    CURRENT_RT.with(|c| c.borrow().clone())
}

// ===========================================================================
// RuntimeInner
// ===========================================================================

pub(crate) struct RuntimeInner {
    pub(crate) reactor: Arc<Reactor>,
    pub(crate) tasks: Arc<TaskMap>,
    pub(crate) queue: Arc<ReadyQueue>,
    pub(crate) blocking: Arc<BlockingPool>,
    metrics: Arc<Metrics>,
    reactor_thread: Mutex<Option<StdJoinHandle<()>>>,
    worker_threads: Mutex<Vec<StdJoinHandle<()>>>,
}

impl RuntimeInner {
    fn spawn_task<F>(&self, f: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let task_id = self.tasks.next_id();
        let handle = JoinHandle::new_with_task(task_id, Arc::clone(&self.queue));
        let handle2 = handle.clone();
        let mut fut = Box::pin(f);
        let metrics_clone = Arc::clone(&self.metrics);

        // Create root span for this task.
        let task_span = trace::task_spawned(task_id, "task");

        self.tasks.insert_with_id(task_id, Box::new(move |cx| {
            // Check abort flag before polling.
            if handle2.is_aborted() {
                metrics_clone.total_aborted.fetch_add(1, Ordering::Relaxed);
                handle2.set_result(Err(JoinError));
                metrics_clone.total_completed.fetch_add(1, Ordering::Relaxed);
                trace::task_aborted(task_id);
                return false;
            }
            // Enter the task span for this poll.
            let _enter = task_span.enter();
            match fut.as_mut().poll(cx) {
                std::task::Poll::Ready(v) => {
                    handle2.set_result(Ok(v));
                    metrics_clone.total_completed.fetch_add(1, Ordering::Relaxed);
                    trace::task_finished(task_id);
                    false
                }
                std::task::Poll::Pending => true,
            }
        }));

        self.metrics.total_spawned.fetch_add(1, Ordering::Relaxed);
        self.queue.push(task_id);
        handle
    }

    /// Shared logic for `spawn_blocking` — creates a JoinHandle and submits
    /// the closure to the blocking pool. Used by both `Runtime::spawn_blocking`
    /// and the free `spawn_blocking` function.
    pub(crate) fn spawn_blocking_task<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let handle = JoinHandle::new();
        let handle2 = handle.clone();
        let blocking = Arc::clone(&self.blocking);
        blocking.spawn(move || {
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
            let result = match r {
                Ok(v) => Ok(v),
                Err(_) => Err(JoinError),
            };
            handle2.set_result(result);
        });
        handle
    }

    /// Get a point-in-time snapshot of runtime metrics.
    pub(crate) fn snapshot_metrics(&self) -> RuntimeMetrics {
        self.metrics.snapshot(
            self.tasks.len(),
            self.queue.len(),
            self.blocking.thread_count(),
            self.blocking.active(),
        )
    }
}

// ===========================================================================
// Builder
// ===========================================================================

/// Configuration for a new [`Runtime`].
pub struct Builder {
    workers: usize,
    blocking_workers: usize,
}

impl Builder {
    /// Creates a multi-threaded runtime builder.
    ///
    /// Worker count defaults to available parallelism (min 4).
    /// Blocking pool defaults to 4 threads.
    pub fn new_multi_thread() -> Self {
        Self {
            workers: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4),
            blocking_workers: 4,
        }
    }

    /// Sets the number of worker threads.
    #[allow(dead_code)]
    pub fn worker_threads(&mut self, n: usize) -> &mut Self {
        self.workers = n;
        self
    }

    /// Sets the number of blocking threads.
    #[allow(dead_code)]
    pub fn max_blocking_threads(&mut self, n: usize) -> &mut Self {
        self.blocking_workers = n;
        self
    }

    /// No-op shim for tokio API compatibility.
    /// All features (I/O, time, signal) are always enabled in edgerun-rt.
    #[allow(dead_code)]
    pub fn enable_all(&mut self) -> &mut Self {
        self
    }

    /// Build the runtime.
    pub fn build(&self) -> std::io::Result<Runtime> {
        let queue = Arc::new(ReadyQueue::new());
        let reactor = Arc::new(Reactor::new()?);
        let tasks = Arc::new(TaskMap::new());
        let blocking = Arc::new(BlockingPool::new(self.blocking_workers));
        let metrics = Arc::new(Metrics::new());

        // Start reactor thread.
        let r2 = reactor.clone();
        let q2 = queue.clone();
        let reactor_thread = std::thread::spawn(move || r2.run(&q2));

        let rt = Arc::new(RuntimeInner {
            reactor,
            tasks,
            queue,
            blocking,
            metrics,
            reactor_thread: Mutex::new(Some(reactor_thread)),
            worker_threads: Mutex::new(Vec::with_capacity(self.workers)),
        });

        // Start worker threads.
        let mut worker_threads = Vec::with_capacity(self.workers);
        for _ in 0..self.workers {
            let tasks = Arc::clone(&rt.tasks);
            let queue = Arc::clone(&rt.queue);
            let rt_ref = Arc::clone(&rt);
            worker_threads.push(std::thread::spawn(move || {
                set_current_rt(rt_ref);
                while let Some(id) = queue.pop() {
                    let Some(mut task) = tasks.take_for_poll(id) else {
                        continue;
                    };
                    let w = make_waker(id, Arc::clone(&queue));
                    let mut cx = std::task::Context::from_waker(&w);

                    let still_pending = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        task(&mut cx)
                    }));

                    match still_pending {
                        Ok(true) => {
                            // Still pending — re-insert for next wakeup.
                            // No sleep needed: the task registered its waker
                            // and will be re-enqueued when the waker fires.
                            tasks.reinsert(id, task);

                            // Check if the task yielded. If so, immediately
                            // re-enqueue it without going through the waker
                            // (which would take a shared mutex on the queue).
                            if crate::yield_now::take_yield_request() {
                                queue.push(id);
                            }
                        }
                        Ok(false) => {
                            // Task completed — dropped (not re-inserted).
                        }
                        Err(panic_info) => {
                            let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                                s.to_string()
                            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                                s.clone()
                            } else {
                                "unknown panic".to_string()
                            };
                            edgerun_log::warn!("task panicked: {}", msg);
                            // Task is dropped — not re-inserted.
                        }
                    }
                }
            }));
        }
        *rt.worker_threads.lock() = worker_threads;

        set_current_rt(rt.clone());

        Ok(Runtime { inner: rt })
    }
}

// ===========================================================================
// Runtime
// ===========================================================================

/// The async runtime. Cloneable (shares the inner state).
pub struct Runtime {
    inner: Arc<RuntimeInner>,
}

impl Runtime {
    /// Shorthand for `Builder::new_multi_thread().build()`.
    #[allow(dead_code)]
    pub fn new_multi_thread() -> Builder {
        Builder::new_multi_thread()
    }

    /// Spawn an async task on this runtime.
    pub fn spawn<F>(&self, f: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.inner.spawn_task(f)
    }

    /// Spawn a blocking closure on the blocking thread pool.
    pub fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        self.inner.spawn_blocking_task(f)
    }

    /// Block the current thread until the given future completes.
    pub fn block_on<F>(&self, f: F) -> F::Output
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        set_current_rt(self.inner.clone());
        self.inner
            .spawn_task(f)
            .blocking_recv()
            .unwrap_or_else(|_| panic!("main future dropped"))
    }

    /// Get a cloneable handle to this runtime.
    ///
    /// The handle can be shared across threads and used for explicit task
    /// spawning without relying on thread-local state.
    pub fn handle(&self) -> RuntimeHandle {
        RuntimeHandle {
            inner: self.inner.clone(),
        }
    }

    /// Graceful shutdown: signal all components and wait for threads.
    pub fn shutdown(&self) {
        self.inner.reactor.shutdown();
        self.inner.queue.shutdown();
        self.inner.blocking.shutdown();

        if let Some(handle) = self.inner.reactor_thread.lock().take() {
            let _ = handle.join();
        }
        for t in self.inner.worker_threads.lock().drain(..) {
            let _ = t.join();
        }
        self.inner.blocking.join();
    }

    /// Get the inner handle for sharing with free functions.
    pub(crate) fn inner(&self) -> Arc<RuntimeInner> {
        self.inner.clone()
    }

    /// Get a point-in-time snapshot of runtime metrics.
    pub fn metrics(&self) -> RuntimeMetrics {
        self.inner.snapshot_metrics()
    }
}

// ===========================================================================
// RuntimeHandle — explicit runtime passing
// ===========================================================================

/// A handle to a running runtime that can be cloned and shared across threads.
///
/// Use this for explicit control instead of the free `spawn()` function which
/// relies on thread-local state.
///
/// # Example
/// ```ignore
/// let rt = Builder::new_multi_thread().build()?;
/// let handle = rt.handle();
///
/// // Share handle across threads
/// let h2 = handle.clone();
/// std::thread::spawn(move || {
///     h2.spawn(async { println!("hello"); });
/// });
/// ```
#[derive(Clone)]
pub struct RuntimeHandle {
    inner: Arc<RuntimeInner>,
}

impl RuntimeHandle {
    /// Spawn an async task on this runtime handle.
    pub fn spawn<F>(&self, f: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.inner.spawn_task(f)
    }

    /// Spawn a blocking closure on this runtime's blocking pool.
    pub fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        self.inner.spawn_blocking_task(f)
    }

    /// Get a point-in-time snapshot of runtime metrics.
    pub fn metrics(&self) -> RuntimeMetrics {
        self.inner.snapshot_metrics()
    }
}

// ===========================================================================
// Free spawn functions
// ===========================================================================

/// Spawn an async task on the current runtime.
///
/// # Panics
/// If called outside a runtime (e.g. before `Runtime::block_on`).
pub fn spawn<F>(f: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let rt = current_rt();
    rt.spawn_task(f)
}

/// Spawn a blocking closure on the current runtime's blocking pool.
///
/// # Panics
/// If called outside a runtime.
pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let rt = current_rt();
    rt.spawn_blocking_task(f)
}

/// Get the current runtime handle, if called from within a runtime.
pub(crate) fn current_handle() -> Option<Arc<RuntimeInner>> {
    try_current_rt()
}
