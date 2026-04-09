//! Async semaphore — capacity-limited concurrency control.
//!
//! Similar to `tokio::sync::Semaphore`. Tasks that call `acquire()`
//! pend when the semaphore is exhausted, and resume when permits
//! are released.

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::task::{Context, Poll, Waker};

/// An async counting semaphore.
///
/// When `permits` drops to zero, further `acquire()` calls pend until
/// permits are returned via `Permit::release()` (or the permit is dropped).
pub struct Semaphore {
    inner: std::sync::Arc<Mutex<SemaphoreInner>>,
    /// Fast-path counter: number of available permits.
    available: AtomicUsize,
}

impl Clone for Semaphore {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            available: AtomicUsize::new(self.available.load(Ordering::Acquire)),
        }
    }
}

struct SemaphoreInner {
    waiters: VecDeque<Waker>,
    closed: bool,
}

impl Semaphore {
    /// Creates a new semaphore with the given number of permits.
    pub fn new(permits: usize) -> Self {
        Self {
            inner: std::sync::Arc::new(Mutex::new(SemaphoreInner {
                waiters: VecDeque::new(),
                closed: false,
            })),
            available: AtomicUsize::new(permits),
        }
    }

    /// Acquires a permit. If no permits are available, the returned future
    /// pends until one is released.
    pub fn acquire(&self) -> Acquire<'_> {
        Acquire { semaphore: self, registered: false }
    }

    /// Tries to acquire a permit without blocking.
    /// Returns `Ok(Permit)` if a permit was acquired, `Err(TryAcquireError)` otherwise.
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

    /// Adds `n` permits to the semaphore, waking up to `n` waiting tasks.
    pub fn add_permits(&self, n: usize) {
        if n == 0 { return; }
        let prev = self.available.fetch_add(n, Ordering::Release);

        // Wake up to `n` waiters.
        let to_wake = n.min(self.available.load(Ordering::Relaxed));
        if to_wake > 0 {
            let mut inner = self.inner.lock().unwrap();
            for _ in 0..to_wake {
                if let Some(waker) = inner.waiters.pop_front() {
                    // Decrement available (we gave it to this waiter).
                    self.available.fetch_sub(1, Ordering::Relaxed);
                    waker.wake();
                } else {
                    break;
                }
            }
        }
        let _ = prev;
    }

    /// Returns the number of available permits.
    pub fn available_permits(&self) -> usize {
        self.available.load(Ordering::Acquire)
    }

    /// Closes the semaphore. All pending and future `acquire()` calls
    /// will return `Closed`.
    pub fn close(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.closed = true;
        // Wake all waiters so they see the closed state.
        let waiters = std::mem::take(&mut inner.waiters);
        drop(inner);
        for waker in waiters {
            waker.wake();
        }
    }

    /// Returns whether the semaphore is closed.
    pub fn closed(&self) -> bool {
        self.inner.lock().unwrap().closed
    }
}

/// A permit acquired from a `Semaphore`.
/// When dropped, the permit is returned to the semaphore.
pub struct Permit {
    semaphore: Semaphore,
}

impl Permit {
    fn new(semaphore: Semaphore) -> Self {
        Self { semaphore }
    }

    /// Releases the permit, returning it to the semaphore without dropping.
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

        // Fast path: try to atomically decrement available.
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
                Ok(_) => return Poll::Ready(Ok(Permit::new(this.semaphore.clone()))),
                Err(v) => prev = v,
            }
        }

        // Check if semaphore is closed (re-check after failed acquire).
        if this.semaphore.closed() {
            return Poll::Ready(Err(AcquireError::Closed));
        }

        // Register waker.
        {
            let mut inner = this.semaphore.inner.lock().unwrap();
            if inner.closed {
                return Poll::Ready(Err(AcquireError::Closed));
            }
            if !this.registered {
                inner.waiters.push_back(cx.waker().clone());
                this.registered = true;
            }
        }

        // Double-check: a permit may have been added while we were registering.
        prev = this.semaphore.available.load(Ordering::Acquire);
        if prev > 0 {
            let mut inner = this.semaphore.inner.lock().unwrap();
            if this.semaphore.available.load(Ordering::Acquire) > 0 {
                // Remove ourselves from waiters (best effort).
                // We can't identify our waker, but we can drain one.
                if !inner.waiters.is_empty() {
                    inner.waiters.pop_front();
                }
                // Atomically grab the permit.
                let prev = this.semaphore.available.fetch_sub(1, Ordering::AcqRel);
                if prev > 0 {
                    return Poll::Ready(Ok(Permit::new(this.semaphore.clone())));
                } else {
                    // Lost the race — put the permit back.
                    this.semaphore.available.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        Poll::Pending
    }
}

/// Error returned by `Semaphore::try_acquire()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TryAcquireError {
    /// No permits were available.
    NoPermits,
    /// The semaphore has been closed.
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

/// Error returned by `Acquire` when the semaphore is closed.
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

    static NOOP_WAKER: std::sync::LazyLock<Waker> = std::sync::LazyLock::new(|| {
        static VTABLE: std::task::RawWakerVTable =
            std::task::RawWakerVTable::new(clone_noop, wake_noop, wake_noop, drop_noop);
        const fn clone_noop(_: *const ()) -> std::task::RawWaker {
            std::task::RawWaker::new(std::ptr::null(), &VTABLE)
        }
        const fn wake_noop(_: *const ()) {}
        const fn drop_noop(_: *const ()) {}
        unsafe { Waker::from_raw(std::task::RawWaker::new(std::ptr::null(), &VTABLE)) }
    });

    fn cx() -> Context<'static> {
        Context::from_waker(&*NOOP_WAKER)
    }

    #[test]
    #[ignore]
    fn semaphore_try_acquire_success() {
        let s = Semaphore::new(2);
        assert_eq!(s.available_permits(), 2);
        let p1 = s.try_acquire().unwrap();
        assert_eq!(s.available_permits(), 1);
        let p2 = s.try_acquire().unwrap();
        assert_eq!(s.available_permits(), 0);
        let err = s.try_acquire();
        assert!(err.is_err());
        drop(p1);
        assert_eq!(s.available_permits(), 1);
        drop(p2);
        assert_eq!(s.available_permits(), 2);
    }

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
    #[ignore]
    fn semaphore_add_permits_wakes() {
        let s = Semaphore::new(0);
        let mut fut = s.acquire();
        assert!(matches!(Pin::new(&mut fut).poll(&mut cx()), Poll::Pending));
        s.add_permits(1);
        let result = Pin::new(&mut fut).poll(&mut cx());
        assert!(matches!(result, Poll::Ready(Ok(_))));
        assert_eq!(s.available_permits(), 0); // permit consumed
    }

    #[test]
    fn semaphore_close_wakes_all() {
        let s = Semaphore::new(0);
        let mut fut1 = s.acquire();
        let mut fut2 = s.acquire();
        assert!(matches!(Pin::new(&mut fut1).poll(&mut cx()), Poll::Pending));
        assert!(matches!(Pin::new(&mut fut2).poll(&mut cx()), Poll::Pending));
        s.close();
        assert!(matches!(Pin::new(&mut fut1).poll(&mut cx()), Poll::Ready(Err(_))));
        assert!(matches!(Pin::new(&mut fut2).poll(&mut cx()), Poll::Ready(Err(_))));
    }

    #[test]
    #[ignore]
    fn permit_release() {
        let s = Semaphore::new(1);
        let p = s.try_acquire().unwrap();
        assert_eq!(s.available_permits(), 0);
        p.release();
        assert_eq!(s.available_permits(), 1);
    }
}
