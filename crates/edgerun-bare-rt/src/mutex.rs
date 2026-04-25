//! Async mutex - exclusive access with async wait support.
//!
//! A sync Mutex guards the data. Data is behind an UnsafeCell, protected by the async protocol.

#![no_std]

extern crate alloc;

use alloc::sync::Arc;
use core::cell::UnsafeCell;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::AtomicBool;
use core::task::{Context, Poll, Waker};

use crate::sync_prim::Mutex as SyncMutex;

// ===========================================================================
// Async Mutex
// ===========================================================================

struct MutexInner {
    acquired: AtomicBool,
    waker: UnsafeCell<Option<Waker>>,
}

/// An async mutex for exclusive access to `T`.
pub struct Mutex<T> {
    state: Arc<MutexInner>,
    data: UnsafeCell<T>,
}

impl<T> Mutex<T> {
    pub fn new(data: T) -> Self {
        Self {
            state: Arc::new(MutexInner {
                acquired: AtomicBool::new(false),
                waker: UnsafeCell::new(None),
            }),
            data: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> MutexLockFuture<'_> {
        MutexLockFuture {
            state: Arc::clone(&self.state),
            data: &self.data,
            registered: false,
        }
    }

    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

impl<T: Default> Default for Mutex<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}

/// A guard that holds exclusive access to the mutex.
pub struct MutexGuard<'a, T> {
    data: &'a UnsafeCell<T>,
    state: Arc<MutexInner>,
}

unsafe impl<T: Send> Send for MutexGuard<'_, T> {}

impl<T> core::ops::Deref for MutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.data.get() }
    }
}

impl<T> core::ops::DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.data.get() }
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.state.acquired.store(false, core::sync::atomic::Ordering::Release);
        unsafe {
            if let Some(w) = (*self.state.waker.get()).take() {
                w.wake();
            }
        }
    }
}

/// A future that resolves to an async mutex lock guard.
pub struct MutexLockFuture<'a, T> {
    state: Arc<MutexInner>,
    data: &'a UnsafeCell<T>,
    registered: bool,
}

unsafe impl<T: Send> Send for MutexLockFuture<'_, T> {}

impl<'a, T> Future for MutexLockFuture<'a, T> {
    type Output = MutexGuard<'a, T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };

        if !this.state.acquired.load(core::sync::atomic::Ordering::Acquire) {
            this.state.acquired.store(true, core::sync::atomic::Ordering::Release);
            return Poll::Ready(MutexGuard {
                data: this.data,
                state: this.state.clone(),
            });
        }

        if !this.registered {
            unsafe { *this.state.waker.get() = Some(cx.waker().clone()) };
            this.registered = true;
        }

        Poll::Pending
    }
}