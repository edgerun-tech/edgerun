//! Async semaphore - capacity-limited concurrency control.

#![no_std]

extern crate alloc;

use alloc::sync::Arc;
use core::cell::UnsafeCell;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::task::{Context, Poll, Waker};

const Acquire: Ordering = Ordering::Acquire;
const AcqRel: Ordering = Ordering::AcqRel;
const Release: Ordering = Ordering::Release;

// ===========================================================================
// Semaphore
// ===========================================================================

struct SemaphoreInner {
    waiters: UnsafeCell<Option<Waker>>,
    closed: UnsafeCell<bool>,
}

/// An async counting semaphore.
pub struct Semaphore {
    inner: Arc<SemaphoreInner>,
    available: Arc<AtomicUsize>,
}

impl Semaphore {
    pub fn new(permits: usize) -> Self {
        Self {
            inner: Arc::new(SemaphoreInner {
                waiters: UnsafeCell::new(None),
                closed: UnsafeCell::new(false),
            }),
            available: Arc::new(AtomicUsize::new(permits)),
        }
    }

    pub fn acquire(&self) -> Acquire<'_> {
        Acquire {
            semaphore: self,
            registered: false,
        }
    }

    pub fn try_acquire(&self) -> Result<Permit, TryAcquireError> {
        if unsafe { *self.inner.closed.get() } {
            return Err(TryAcquireError::Closed);
        }
        let mut prev = self.available.load(Acquire);
        loop {
            if prev == 0 {
                return Err(TryAcquireError::NoPermits);
            }
            match self.available.compare_exchange(prev, prev - 1, AcqRel, Acquire) {
                Ok(_) => return Ok(Permit::new(self.clone())),
                Err(v) => prev = v,
            }
        }
    }

    pub fn add_permits(&self, n: usize) {
        if n == 0 {
            return;
        }
        self.available.fetch_add(n, Release);
        if n > 0 {
            unsafe {
                if let Some(w) = (*self.inner.waiters.get()).take() {
                    w.wake();
                }
            }
        }
    }

    pub fn available_permits(&self) -> usize {
        self.available.load(Acquire)
    }

    pub fn close(&self) {
        unsafe { *self.inner.closed.get() = true };
        unsafe {
            if let Some(w) = (*self.inner.waiters.get()).take() {
                w.wake();
            }
        }
    }

    pub fn closed(&self) -> bool {
        unsafe { *self.inner.closed.get() }
    }
}

impl Clone for Semaphore {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            available: self.available.clone(),
        }
    }
}

/// A permit acquired from a `Semaphore`.
pub struct Permit {
    semaphore: Semaphore,
}

impl Permit {
    fn new(semaphore: Semaphore) -> Self {
        Self { semaphore }
    }

    pub fn release(self) {
        self.semaphore.add_permits(1);
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        self.semaphore.add_permits(1);
    }
}

/// Future returned by `Semaphore::acquire()`.
pub struct Acquire<'a> {
    semaphore: &'a Semaphore,
    registered: bool,
}

impl Future for Acquire<'_> {
    type Output = Result<Permit, AcquireError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };

        if this.semaphore.closed() {
            return Poll::Ready(Err(AcquireError::Closed));
        }

        // Fast path: try to atomically decrement
        let mut prev = this.semaphore.available.load(Acquire);
        loop {
            if prev == 0 {
                break;
            }
            match this.semaphore.available.compare_exchange(
                prev,
                prev - 1,
                AcqRel,
                Acquire,
            ) {
                Ok(_) => return Poll::Ready(Ok(Permit::new(this.semaphore.clone()))),
                Err(v) => prev = v,
            }
        }

        // Re-check closed
        if this.semaphore.closed() {
            return Poll::Ready(Err(AcquireError::Closed));
        }

        // Register waker
        if !this.registered {
            unsafe { *this.semaphore.inner.waiters.get() = Some(cx.waker().clone()) };
            this.registered = true;
        }

        Poll::Pending
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TryAcquireError {
    NoPermits,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcquireError {
    Closed,
}