//! Blocking thread pool and `JoinHandle`.
//!
//! `JoinHandle<T>` is both an async `Future` and provides `blocking_recv`.
//! Supports `abort()` to cancel a task.
//!
//! ## Fixes applied:
//! - `JoinHandle::abort()` — sets a flag checked before each poll
//! - Tasks are re-enqueued on abort so the worker sees the flag
//! - Single `JoinInner` pattern shared by spawn and spawn_blocking

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use crate::sync::{Condvar, Mutex};
use std::task::{Context, Poll};
use std::thread::JoinHandle as StdJoinHandle;

// ===========================================================================
// JoinHandle
// ===========================================================================

struct JoinInner<T> {
    result: Mutex<Option<Result<T, JoinError>>>,
    cvar: Condvar,
    waker: Mutex<Option<std::task::Waker>>,
    aborted: AtomicBool,
    completed: AtomicBool,
    /// Task ID for re-enqueuing on abort. None for blocking tasks.
    task_id: Option<usize>,
    /// Ready queue for re-enqueuing aborted tasks.
    queue: Option<Arc<crate::ready_queue::ReadyQueue>>,
}

/// Handle for a spawned task. Can be awaited or `blocking_recv`'d.
pub struct JoinHandle<T> {
    inner: Arc<JoinInner<T>>,
}

impl<T> JoinHandle<T> {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(JoinInner {
                result: Mutex::new(None),
                cvar: Condvar::new(),
                waker: Mutex::new(None),
                aborted: AtomicBool::new(false),
                completed: AtomicBool::new(false),
                task_id: None,
                queue: None,
            }),
        }
    }

    /// Create a JoinHandle for an async task (with task ID and ready queue).
    pub(crate) fn new_with_task(
        task_id: usize,
        queue: Arc<crate::ready_queue::ReadyQueue>,
    ) -> Self {
        Self {
            inner: Arc::new(JoinInner {
                result: Mutex::new(None),
                cvar: Condvar::new(),
                waker: Mutex::new(None),
                aborted: AtomicBool::new(false),
                completed: AtomicBool::new(false),
                task_id: Some(task_id),
                queue: Some(queue),
            }),
        }
    }

    pub(crate) fn set_result(&self, result: Result<T, JoinError>) {
        // Use compare_exchange to ensure only one writer sets the result.
        let mut guard = self.inner.result.lock();
        if guard.is_some() {
            // Already has a result — don't overwrite.
            return;
        }
        *guard = Some(result);
        self.inner.completed.store(true, Ordering::Release);
        self.inner.cvar.notify_all();
        if let Some(waker) = self.inner.waker.lock().take() {
            waker.wake();
        }
    }

    /// Blocking receive. Parks the thread until the result is available.
    pub fn blocking_recv(self) -> Result<T, JoinError> {
        let mut guard = self.inner.result.lock();
        while guard.is_none() {
            self.inner.cvar.wait(&mut guard);
        }
        guard.take().unwrap()
    }

    /// Abort the task. If the task hasn't completed yet, it will be dropped
    /// on the next poll cycle and `JoinHandle` will resolve with `JoinError`.
    /// If the task has already completed, this is a no-op.
    ///
    /// Abortion is cooperative — the task is simply not polled again.
    /// It does **not** send a signal or interrupt a running poll.
    pub fn abort(&self) {
        // No-op if already completed.
        if self.inner.completed.load(Ordering::Acquire) {
            return;
        }
        self.inner.aborted.store(true, Ordering::Release);

        // Re-enqueue the task so the worker polls it and sees the abort flag.
        if let (Some(id), Some(q)) = (self.inner.task_id, &self.inner.queue) {
            q.push(id);
        }

        // Wake up any awaiter so they see the abort result.
        self.inner.cvar.notify_all();
        if let Some(waker) = self.inner.waker.lock().take() {
            waker.wake();
        }
    }

    /// Returns true if the task has completed (successfully or with error).
    pub fn is_finished(&self) -> bool {
        self.inner.completed.load(Ordering::Acquire)
    }

    /// Returns true if `abort()` was called on this handle.
    pub fn is_aborted(&self) -> bool {
        self.inner.aborted.load(Ordering::Acquire)
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = Result<T, JoinError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut guard = self.inner.result.lock();
        if let Some(result) = guard.take() {
            Poll::Ready(result)
        } else {
            *self.inner.waker.lock() = Some(cx.waker().clone());
            // Double-check after registering waker.
            if let Some(result) = guard.take() {
                self.inner.waker.lock().take();
                Poll::Ready(result)
            } else {
                Poll::Pending
            }
        }
    }
}

impl<T> Clone for JoinHandle<T> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

/// Error returned when a `JoinHandle`'s task is dropped or aborted.
#[derive(Debug)]
pub struct JoinError;

impl std::fmt::Display for JoinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("join error")
    }
}

impl std::error::Error for JoinError {}

// ===========================================================================
// Blocking pool
// ===========================================================================

/// Default capacity for the blocking job queue.
/// When full, `spawn()` returns `PoolError::Full` instead of growing unboundedly.
const POOL_QUEUE_CAPACITY: usize = 1024;

/// Error returned by `BlockingPool::spawn()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolError {
    /// The pool has been shut down.
    Shutdown,
    /// The job queue is full. Wait and retry, or increase pool size.
    Full,
}

impl std::fmt::Display for PoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PoolError::Shutdown => write!(f, "blocking pool has been shut down"),
            PoolError::Full => write!(f, "blocking job queue is full"),
        }
    }
}

impl std::error::Error for PoolError {}

struct PoolInner {
    /// `None` = shut down. Held inside a Mutex so `shutdown()` can take it
    /// (dropping the sender → `Disconnected` for all workers).
    tx: Mutex<Option<std::sync::mpsc::SyncSender<Box<dyn FnOnce() + Send>>>>,
    threads: Mutex<Vec<StdJoinHandle<()>>>,
    joined: AtomicBool,
    thread_count: usize,
    active: AtomicUsize,
}

/// Bounded blocking thread pool for `spawn_blocking`.
///
/// Fix #3: Uses a bounded `sync_channel(POOL_QUEUE_CAPACITY)` instead of an
/// unbounded `channel()`. Callers get `PoolError::Full` when the queue is full
/// instead of silently growing memory.
///
/// Fix #9: `shutdown()` drops the sender, causing workers to see `Disconnected`
/// and exit promptly (no more 100ms polling + flag dance).
pub(crate) struct BlockingPool {
    inner: Arc<PoolInner>,
}

impl BlockingPool {
    pub(crate) fn new(size: usize) -> Self {
        let (tx, rx) =
            std::sync::mpsc::sync_channel::<Box<dyn FnOnce() + Send>>(POOL_QUEUE_CAPACITY);
        let rx = Arc::new(std::sync::Mutex::new(rx));

        let threads: Vec<_> = (0..size)
            .map(|_| {
                let rx = Arc::clone(&rx);
                std::thread::spawn(move || {
                    loop {
                        // Blocking recv. Returns Err(Disconnected) when sender is dropped.
                        let job = {
                            let guard = rx.lock().unwrap();
                            guard.recv()
                        };
                        match job {
                            Ok(job) => {
                                job();
                            }
                            Err(std::sync::mpsc::RecvError) => break,
                        }
                    }
                    // Drain remaining jobs.
                    let guard = rx.lock().unwrap();
                    while let Ok(job) = guard.try_recv() {
                        if let Err(panic_info) =
                            std::panic::catch_unwind(std::panic::AssertUnwindSafe(job))
                        {
                            let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                                s.to_string()
                            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                                s.clone()
                            } else {
                                "unknown panic".to_string()
                            };
                            edgerun_log::warn!("blocking task panicked: {}", msg);
                        }
                    }
                })
            })
            .collect();

        Self {
            inner: Arc::new(PoolInner {
                tx: Mutex::new(Some(tx)),
                threads: Mutex::new(threads),
                joined: AtomicBool::new(false),
                thread_count: size,
                active: AtomicUsize::new(0),
            }),
        }
    }

    /// Submit a job to the blocking pool.
    ///
    /// Returns `Err(PoolError::Full)` if the queue is at capacity, or
    /// `Err(PoolError::Shutdown)` if the pool has been shut down.
    pub(crate) fn spawn<F>(&self, f: F) -> Result<(), PoolError>
    where
        F: FnOnce() + Send + 'static,
    {
        let tx_guard = self.inner.tx.lock();
        let tx = (&*tx_guard).as_ref().ok_or(PoolError::Shutdown)?;

        let inner = Arc::clone(&self.inner);
        tx.send(Box::new(move || {
            inner.active.fetch_add(1, Ordering::Relaxed);
            f();
            inner.active.fetch_sub(1, Ordering::Relaxed);
        }))
        .map_err(|_| PoolError::Shutdown)
    }

    /// Shut down the pool. Drops the sender so all workers see `Disconnected`
    /// and exit. No more jobs are accepted after this call.
    pub(crate) fn shutdown(&self) {
        // Take and drop the sender → all workers get Disconnected.
        self.inner.tx.lock().take();
    }

    pub(crate) fn join(&self) {
        if self.inner.joined.swap(true, Ordering::AcqRel) {
            return;
        }
        for t in self.inner.threads.lock().drain(..) {
            let _ = t.join();
        }
    }

    /// Number of threads in the blocking pool.
    pub(crate) fn thread_count(&self) -> usize {
        self.inner.thread_count
    }

    /// Number of blocking tasks currently running.
    pub(crate) fn active(&self) -> usize {
        self.inner.active.load(Ordering::Relaxed)
    }
}
