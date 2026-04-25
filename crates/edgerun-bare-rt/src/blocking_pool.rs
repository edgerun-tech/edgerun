//! Blocking thread pool and `JoinHandle`.

#![no_std]

extern crate alloc;

use crate::sync_prim::{Condvar, Mutex};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use core::task::Waker;

// ===========================================================================
// JoinHandle
// ===========================================================================

struct JoinInner<T> {
    result: Mutex<Option<Result<T, JoinError>>>,
    cvar: Condvar,
    waker: Mutex<Option<Waker>>,
    completed: AtomicBool,
}

impl<T> JoinInner<T> {
    fn new() -> Self {
        Self {
            result: Mutex::new(None),
            cvar: Condvar::new(),
            waker: Mutex::new(None),
            completed: AtomicBool::new(false),
        }
    }
}

    fn set_result(&self, result: Result<T, JoinError>) {
        let mut guard = self.result.lock();
        if guard.is_some() {
            return;
        }
        *guard = Some(result);
        self.completed.store(true, Ordering::Release);
        self.cvar.notify_all();
        if let Some(w) = self.waker.lock().take() {
            w.wake();
        }
    }
}

/// Handle for a spawned task.
pub struct JoinHandle<T> {
    inner: Arc<JoinInner<T>>,
}

impl<T> JoinHandle<T> {
    pub fn blocking_recv(self) -> Result<T, JoinError> {
        let mut guard = self.inner.result.lock();
        while guard.is_none() {
            self.inner.cvar.wait(&mut guard);
        }
        guard.take().unwrap()
    }

    pub fn is_finished(&self) -> bool {
        self.inner.completed.load(Ordering::Acquire)
    }
}

impl<T> Clone for JoinHandle<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

#[derive(Debug)]
pub struct JoinError;

// ===========================================================================
// Blocking pool (stub for now)
// ===========================================================================

pub struct BlockingPool {
    _priv: (),
}

impl BlockingPool {
    pub fn new(_size: usize) -> Self {
        Self { _priv: () }
    }

    pub fn spawn<F, R>(&self, _f: F) -> Result<(), PoolError>
    where
        F: FnOnce() -> R + Send,
        R: Send,
    {
        Err(PoolError::Full)
    }

    pub fn shutdown(&self) {}
}

#[derive(Debug)]
pub enum PoolError {
    Full,
    Shutdown,
}