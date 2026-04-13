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
use parking_lot::{Condvar, Mutex};
use std::task::{Context, Poll};
use std::thread::JoinHandle as StdJoinHandle;
use std::time::Duration;

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
        *self.inner.result.lock() = Some(result);
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

struct PoolInner {
    tx: Mutex<std::sync::mpsc::Sender<Box<dyn FnOnce() + Send>>>,
    shutdown_flag: Arc<AtomicBool>,
    threads: Mutex<Vec<StdJoinHandle<()>>>,
    joined: AtomicBool,
    thread_count: usize,
    active: AtomicUsize,
}

/// Bounded blocking thread pool for `spawn_blocking`.
pub(crate) struct BlockingPool {
    inner: Arc<PoolInner>,
}

impl BlockingPool {
    pub(crate) fn new(size: usize) -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<Box<dyn FnOnce() + Send>>();
        let rx = Arc::new(Mutex::new(rx));
        let shutdown_flag = Arc::new(AtomicBool::new(false));

        let threads: Vec<_> = (0..size)
            .map(|_| {
                let rx = Arc::clone(&rx);
                let flag = Arc::clone(&shutdown_flag);
                std::thread::spawn(move || {
                    loop {
                        let job = {
                            let guard = rx.lock();
                            guard.recv_timeout(Duration::from_millis(100))
                        };
                        match job {
                            Ok(job) => {
                                job();
                            }
                            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                                if flag.load(Ordering::Relaxed) {
                                    break;
                                }
                                continue;
                            }
                            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                        }
                    }
                    // Drain remaining jobs.
                    let guard = rx.lock();
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
                tx: Mutex::new(tx),
                shutdown_flag,
                threads: Mutex::new(threads),
                joined: AtomicBool::new(false),
                thread_count: size,
                active: AtomicUsize::new(0),
            }),
        }
    }

    pub(crate) fn spawn<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let inner = Arc::clone(&self.inner);
        let _ = self.inner.tx.lock().send(Box::new(move || {
            inner.active.fetch_add(1, Ordering::Relaxed);
            f();
            inner.active.fetch_sub(1, Ordering::Relaxed);
        }));
    }

    pub(crate) fn shutdown(&self) {
        self.inner.shutdown_flag.store(true, Ordering::Release);
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
