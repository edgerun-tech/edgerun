//! Async read-write lock.
//!
//! Uses our `sync::RwLock` internally for correct concurrent reader access.
//! The async layer provides wait queues with write-preference scheduling.
//!
//! ## Fix applied:
//! Guards hold `crate::sync::RwLockReadGuard`/`RwLockWriteGuard` (not `MutexGuard`),
//! so multiple readers coexist correctly. The async wait queue prevents busy-spinning.

use crate::sync::{Mutex, RwLock as SyncRwLock};
use std::collections::VecDeque;
use std::future::Future;
use std::mem::ManuallyDrop;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

struct RwState {
    async_readers: usize,
    async_writing: bool,
    waiting_writers: usize,
    read_waiters: VecDeque<Waker>,
    write_waiters: VecDeque<Waker>,
}

struct Inner<T> {
    data: SyncRwLock<T>,
    state: Arc<Mutex<RwState>>,
}

/// An async read-write lock. Write-preferring for waiting writers.
pub struct RwLock<T> {
    inner: Arc<Inner<T>>,
}

impl<T> RwLock<T> {
    pub fn new(value: T) -> Self {
        let state = Arc::new(Mutex::new(RwState {
            async_readers: 0,
            async_writing: false,
            waiting_writers: 0,
            read_waiters: VecDeque::new(),
            write_waiters: VecDeque::new(),
        }));
        Self {
            inner: Arc::new(Inner {
                data: SyncRwLock::new(value),
                state,
            }),
        }
    }

    pub fn read(&self) -> RwLockReadFuture<'_, T> {
        RwLockReadFuture {
            inner: &self.inner,
            arc: Arc::clone(&self.inner),
            registered: false,
        }
    }

    pub fn write(&self) -> RwLockWriteFuture<'_, T> {
        RwLockWriteFuture {
            inner: &self.inner,
            arc: Arc::clone(&self.inner),
            registered: false,
        }
    }

    pub fn into_inner(self) -> T {
        match Arc::try_unwrap(self.inner) {
            Ok(inner) => inner.data.into_inner(),
            Err(_) => panic!("RwLock::into_inner called with outstanding guards"),
        }
    }
}

impl<T: Default> Default for RwLock<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

/// A read guard. Multiple readers can coexist.
pub struct RwLockReadGuard<'a, T> {
    guard: crate::sync::RwLockReadGuard<'a, T>,
    state: Arc<Mutex<RwState>>,
}

impl<T> std::ops::Deref for RwLockReadGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.guard
    }
}

impl<T> Drop for RwLockReadGuard<'_, T> {
    fn drop(&mut self) {
        let mut state = self.state.lock();
        state.async_readers -= 1;
        // If no more async readers and writers are waiting, wake one.
        if state.async_readers == 0 && state.waiting_writers > 0 {
            if let Some(waker) = state.write_waiters.pop_front() {
                waker.wake();
            }
        }
    }
}

/// A write guard. Exclusive access.
pub struct RwLockWriteGuard<'a, T> {
    guard: crate::sync::RwLockWriteGuard<'a, T>,
    state: Arc<Mutex<RwState>>,
}

impl<T> std::ops::Deref for RwLockWriteGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.guard
    }
}

impl<T> std::ops::DerefMut for RwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.guard
    }
}

impl<T> Drop for RwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        let mut state = self.state.lock();
        state.async_writing = false;
        if state.waiting_writers > 0 {
            state.waiting_writers -= 1;
            if let Some(waker) = state.write_waiters.pop_front() {
                waker.wake();
            }
        } else {
            let waiters = std::mem::take(&mut state.read_waiters);
            for waker in waiters {
                state.async_readers += 1;
                waker.wake();
            }
        }
    }
}

pub struct RwLockReadFuture<'a, T> {
    inner: &'a Inner<T>,
    arc: Arc<Inner<T>>,
    registered: bool,
}

pub struct RwLockWriteFuture<'a, T> {
    inner: &'a Inner<T>,
    arc: Arc<Inner<T>>,
    registered: bool,
}

// Safety: The RwLockReadFuture borrows `inner` which is behind an Arc in the
// RwLock. The future's output guard also references the same inner. This is
// safe because the RwLock outlives the future (the RwLock owns the Arc).
// We use a lifetime trick: the future's lifetime 'a is bounded by the RwLock's
// borrow, and the guard uses the same 'a from the Inner's data.

impl<'a, T> Future for RwLockReadFuture<'a, T> {
    type Output = RwLockReadGuard<'a, T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let mut state = this.inner.state.lock();

        // Write-preference: if a writer is active or waiting, block readers.
        if state.async_writing || state.waiting_writers > 0 {
            if !this.registered {
                state.read_waiters.push_back(cx.waker().clone());
                this.registered = true;
            }
            return Poll::Pending;
        }

        // No async contention — try the underlying RwLock.
        if let Some(guard) = this.inner.data.try_read() {
            state.async_readers += 1;
            return Poll::Ready(RwLockReadGuard {
                guard,
                state: this.arc.state.clone(),
            });
        }

        // Underlying RwLock is busy (shouldn't happen in pure-async usage).
        if !this.registered {
            state.read_waiters.push_back(cx.waker().clone());
            this.registered = true;
        }
        Poll::Pending
    }
}

impl<'a, T> Future for RwLockWriteFuture<'a, T> {
    type Output = RwLockWriteGuard<'a, T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let mut state = this.inner.state.lock();

        if state.async_writing || state.async_readers > 0 {
            if !this.registered {
                state.waiting_writers += 1;
                state.write_waiters.push_back(cx.waker().clone());
                this.registered = true;
            }
            return Poll::Pending;
        }

        if let Some(guard) = this.inner.data.try_write() {
            state.async_writing = true;
            return Poll::Ready(RwLockWriteGuard {
                guard,
                state: this.arc.state.clone(),
            });
        }

        if !this.registered {
            state.waiting_writers += 1;
            state.write_waiters.push_back(cx.waker().clone());
            this.registered = true;
        }
        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::cx;
    use std::pin::Pin;

    #[test]
    fn rwlock_read_immediate() {
        let lock = RwLock::new(42);
        let mut fut = lock.read();
        let Poll::Ready(guard) = Pin::new(&mut fut).poll(&mut cx()) else {
            panic!("expected ready")
        };
        assert_eq!(*guard, 42);
    }

    #[test]
    fn rwlock_write_immediate() {
        let lock = RwLock::new(42);
        let mut fut = lock.write();
        let Poll::Ready(mut guard) = Pin::new(&mut fut).poll(&mut cx()) else {
            panic!("expected ready")
        };
        *guard += 1;
        drop(guard);
        let Poll::Ready(guard2) = Pin::new(&mut lock.read()).poll(&mut cx()) else {
            panic!("expected ready")
        };
        assert_eq!(*guard2, 43);
    }

    #[test]
    fn rwlock_multiple_readers() {
        let lock = RwLock::new(42);
        let Poll::Ready(_g1) = Pin::new(&mut lock.read()).poll(&mut cx()) else {
            panic!("expected ready")
        };
        let Poll::Ready(_g2) = Pin::new(&mut lock.read()).poll(&mut cx()) else {
            panic!("expected ready")
        };
    }

    #[test]
    fn rwlock_write_blocks_reader() {
        let lock = RwLock::new(42);
        let mut write_fut = lock.write();
        let Poll::Ready(_guard) = Pin::new(&mut write_fut).poll(&mut cx()) else {
            panic!("expected ready")
        };
        let mut read_fut = lock.read();
        assert!(matches!(
            Pin::new(&mut read_fut).poll(&mut cx()),
            Poll::Pending
        ));
        drop(_guard);
        let Poll::Ready(_guard) = Pin::new(&mut read_fut).poll(&mut cx()) else {
            panic!("expected ready")
        };
    }

    #[test]
    fn rwlock_reader_blocks_writer() {
        let lock = RwLock::new(42);
        let Poll::Ready(_g) = Pin::new(&mut lock.read()).poll(&mut cx()) else {
            panic!("expected ready")
        };
        let mut write_fut = lock.write();
        assert!(matches!(
            Pin::new(&mut write_fut).poll(&mut cx()),
            Poll::Pending
        ));
    }

    #[test]
    fn rwlock_into_inner() {
        let lock = RwLock::new(42);
        let val = lock.into_inner();
        assert_eq!(val, 42);
    }
}
