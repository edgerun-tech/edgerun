//! Async notification primitive.
//!
//! A `Notify` allows one or more tasks to be woken when notified.
//! Unlike `Condvar`, this is fully async - no thread blocking.

#![no_std]

extern crate alloc;

use alloc::sync::Arc;
use core::cell::UnsafeCell;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::task::{Context, Poll, Waker};

const Acquire: Ordering = Ordering::Acquire;
const Release: Ordering = Ordering::Release;

// ===========================================================================
// Notify
// ===========================================================================

struct NotifyInner {
    notified: AtomicUsize,
    waiters: UnsafeCell<Option<Waker>>,
}

impl NotifyInner {
    fn new() -> Self {
        Self {
            notified: AtomicUsize::new(0),
            waiters: UnsafeCell::new(None),
        }
    }
}

/// An async notification primitive. Thread-safe, can be cloned.
pub struct Notify {
    inner: Arc<NotifyInner>,
}

impl Notify {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(NotifyInner::new()),
        }
    }

    pub fn notify_one(&self) {
        self.inner.notified.fetch_add(1, Release);
        unsafe {
            if let Some(w) = (*self.inner.waiters.get()).take() {
                w.wake();
            }
        }
    }

    pub fn notify_waiters(&self) {
        self.inner.notified.fetch_add(1, Release);
        unsafe {
            if let Some(w) = (*self.inner.waiters.get()).take() {
                w.wake();
            }
        }
    }

    pub fn notified(&self) -> Notified<'_> {
        Notified { notify: self }
    }
}

impl Clone for Notify {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl Default for Notify {
    fn default() -> Self {
        Self::new()
    }
}

/// A future that resolves when `Notify::notify_one()` is called.
pub struct Notified<'a> {
    notify: &'a Notify,
}

impl Future for Notified<'_> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if this.notify.inner.notified.load(Acquire) > 0 {
            this.notify.inner.notified.fetch_sub(1, Release);
            return Poll::Ready(());
        }
        unsafe {
            *this.notify.inner.waiters.get() = Some(cx.waker().clone());
        }
        Poll::Pending
    }
}