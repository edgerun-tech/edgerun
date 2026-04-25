//! Blocking thread pool and JoinHandle.

#![no_std]

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::boxed::Box;
use crate::sync_prim::{Condvar, Mutex};
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub struct JoinHandle<T> {
    inner: Arc<JoinInner<T>>,
}

struct JoinInner<T> {
    result: Mutex<Option<Result<T, JoinError>>>,
    cvar: Condvar,
    completed: AtomicBool,
}

impl<T> JoinInner<T> {
    fn new() -> Self {
        Self {
            result: Mutex::new(None),
            cvar: Condvar::new(),
            completed: AtomicBool::new(false),
        }
    }
}

impl<T> JoinHandle<T> {
    pub fn new_with_task(_task_id: usize, _queue: Arc<crate::ready_queue::ReadyQueue>) -> Self {
        Self {
            inner: Arc::new(JoinInner::new()),
        }
    }

    pub fn blocking_recv(self) -> Result<T, JoinError> {
        let mut guard = self.inner.result.lock();
        while !self.inner.completed.load(Ordering::Acquire) {
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

pub struct BlockingPool {
    inner: Arc<PoolInner>,
    tasks: Mutex<Vec<Box<dyn FnOnce() + Send>>>,
}

struct PoolInner {
    max: usize,
    current: AtomicUsize,
    shutdown: AtomicBool,
}

impl BlockingPool {
    pub fn new(size: usize) -> Self {
        Self {
            inner: Arc::new(PoolInner {
                max: size,
                current: AtomicUsize::new(0),
                shutdown: AtomicBool::new(false),
            }),
            tasks: Mutex::new(Vec::new()),
        }
    }

    pub fn spawn<F, R>(&self, f: F) -> Result<JoinHandle<R>, PoolError>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        if self.inner.shutdown.load(Ordering::Acquire) {
            return Err(PoolError::Shutdown);
        }
        
        let current = self.inner.current.load(Ordering::Acquire);
        if current >= self.inner.max {
            return Err(PoolError::Full);
        }
        
        self.inner.current.fetch_add(1, Ordering::Relaxed);
        
        let handle = JoinHandle::new_with_task(0, Arc::new(crate::ready_queue::ReadyQueue::new()));
        
        Ok(handle)
    }

    pub fn shutdown(&self) {
        self.inner.shutdown.store(true, Ordering::Release);
    }
}