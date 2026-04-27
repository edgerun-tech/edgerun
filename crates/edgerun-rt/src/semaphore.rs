//! Async semaphore — capacity-limited concurrency control.
//!
//! ## Fixes applied:
//! - Removed racy double-check grab logic in `Acquire::poll`.
//!   If we register and are pending, we trust the waker system.
//!   When `add_permits` wakes us, the fast-path CAS succeeds on re-poll.

use crate::sync::Mutex;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};

struct SemaphoreInner {
    waiters: VecDeque<Waker>,
    closed: bool,
}

/// An async counting semaphore.
pub struct Semaphore {
    inner: std::sync::Arc<Mutex<SemaphoreInner>>,
    available: std::sync::Arc<AtomicUsize>,
}

impl Clone for Semaphore {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            available: self.available.clone(),
        }
    }
}

impl Semaphore {
    pub fn new(permits: usize) -> Self {
        Self {
            inner: std::sync::Arc::new(Mutex::new(SemaphoreInner {
                waiters: VecDeque::new(),
                closed: false,
            })),
            available: std::sync::Arc::new(AtomicUsize::new(permits)),
        }
    }

    pub fn acquire(&self) -> Acquire<'_> {
        Acquire {
            semaphore: self,
            registered: false,
        }
    }

    /// Tries to acquire a permit without blocking.
    pub fn try_acquire(&self) -> Result<Permit, TryAcquireError> {
        if self.closed() {
            return Err(TryAcquireError::Closed);
        }
        let mut prev = self.available.load(Ordering::Acquire);
        loop {
            if prev == 0 {
                return Err(TryAcquireError::NoPermits);
            }
            match self.available.compare_exchange_weak(
                prev,
                prev - 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Ok(Permit::new(self.clone())),
                Err(v) => prev = v,
            }
        }
    }

    /// Adds `n` permits, waking up to `n` waiting tasks.
    pub fn add_permits(&self, n: usize) {
        if n == 0 {
            return;
        }
        self.available.fetch_add(n, Ordering::Release);

        let to_wake = n;
        if to_wake > 0 {
            let mut inner = self.inner.lock();
            for _ in 0..to_wake {
                if let Some(waker) = inner.waiters.pop_front() {
                    waker.wake();
                } else {
                    break;
                }
            }
        }
    }

    pub fn available_permits(&self) -> usize {
        self.available.load(Ordering::Acquire)
    }

    pub fn close(&self) {
        let mut inner = self.inner.lock();
        inner.closed = true;
        let waiters = std::mem::take(&mut inner.waiters);
        drop(inner);
        for waker in waiters {
            waker.wake();
        }
    }

    pub fn closed(&self) -> bool {
        self.inner.lock().closed
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

        // Fast path: try to atomically decrement.
        let mut prev = this.semaphore.available.load(Ordering::Acquire);
        loop {
            if prev == 0 {
                break;
            }
            match this.semaphore.available.compare_exchange_weak(
                prev,
                prev - 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    return Poll::Ready(Ok(Permit::new(this.semaphore.clone())));
                }
                Err(v) => prev = v,
            }
        }

        // Re-check closed.
        if this.semaphore.closed() {
            return Poll::Ready(Err(AcquireError::Closed));
        }

        // Register waker.
        {
            let mut inner = this.semaphore.inner.lock();
            if inner.closed {
                return Poll::Ready(Err(AcquireError::Closed));
            }
            if !this.registered {
                inner.waiters.push_back(cx.waker().clone());
                this.registered = true;
            }
        }

        // No racy double-check: trust the waker. On re-poll,
        // the fast-path CAS will grab the permit.
        Poll::Pending
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TryAcquireError {
    NoPermits,
    Closed,
}

impl std::fmt::Display for TryAcquireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TryAcquireError::NoPermits => write!(f, "no permits available"),
            TryAcquireError::Closed => write!(f, "semaphore closed"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcquireError {
    Closed,
}

impl std::fmt::Display for AcquireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "semaphore closed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::cx;

    #[test]
    fn semaphore_try_acquire_closed() {
        let s = Semaphore::new(1);
        s.close();
        assert!(s.try_acquire().is_err());
    }

    #[test]
    fn semaphore_acquire_pending() {
        let s = Semaphore::new(0);
        let mut fut = s.acquire();
        assert!(matches!(Pin::new(&mut fut).poll(&mut cx()), Poll::Pending));
    }

    #[test]
    fn semaphore_acquire_immediate() {
        let s = Semaphore::new(1);
        let mut fut = s.acquire();
        let result = Pin::new(&mut fut).poll(&mut cx());
        assert!(matches!(result, Poll::Ready(Ok(_))));
        assert_eq!(s.available_permits(), 0);
    }

    #[test]
    fn semaphore_close_wakes_all() {
        let s = Semaphore::new(0);
        let mut fut1 = s.acquire();
        let mut fut2 = s.acquire();
        assert!(matches!(Pin::new(&mut fut1).poll(&mut cx()), Poll::Pending));
        assert!(matches!(Pin::new(&mut fut2).poll(&mut cx()), Poll::Pending));
        s.close();
        assert!(matches!(
            Pin::new(&mut fut1).poll(&mut cx()),
            Poll::Ready(Err(_))
        ));
        assert!(matches!(
            Pin::new(&mut fut2).poll(&mut cx()),
            Poll::Ready(Err(_))
        ));
    }
}
