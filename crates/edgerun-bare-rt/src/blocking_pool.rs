//! Blocking thread pool and JoinHandle.

#![no_std]

extern crate alloc;

use alloc::sync::Arc;
use crate::sync_prim::{Condvar, Mutex};
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::Waker;

pub struct JoinHandle<T> {
    inner: Arc<JoinInner<T>>,
}

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

impl<T> JoinHandle<T> {
    pub fn new_with_task(task_id: usize, queue: Arc<crate::ready_queue::ReadyQueue>) -> Self {
        Self {
            inner: Arc::new(JoinInner::new()),
        }
    }

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

    pub fn abort(&self) {}
}

impl<T> Clone for JoinHandle<T> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

#[derive(Debug)]
pub struct JoinError;

#[derive(Debug)]
pub enum PoolError {
    Full,
    Shutdown,
}

pub struct BlockingPool { _priv: () }

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