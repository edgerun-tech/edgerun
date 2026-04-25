//! Broadcast channel - multi-sender, multi-receiver.

#![no_std]

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::task::{Context, Poll, Waker};

const Acquire: Ordering = Ordering::Acquire;
const Release: Ordering = Ordering::Release;

// ===========================================================================
// Broadcast
// ===========================================================================

struct BroadcastInner<T> {
    value: UnsafeCell<T>,
    version: AtomicUsize,
    receivers: UnsafeCell<Vec<Waker>>,
}

impl<T> BroadcastInner<T> {
    fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
            version: AtomicUsize::new(0),
            receivers: UnsafeCell::new(Vec::new()),
        }
    }
}

/// Creates a new broadcast channel with the initial value.
pub fn channel<T: Clone + Send>(initial: T) -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(BroadcastInner::new(initial));
    (Sender { inner: inner.clone() }, Receiver { inner })
}

pub struct Sender<T> {
    inner: Arc<BroadcastInner<T>>,
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Sender<T> {
    pub fn send(&self, value: T) {
        unsafe { *self.inner.value.get() = value };
        self.inner.version.fetch_add(1, Release);
        unsafe {
            let receivers = (*self.inner.receivers.get()).drain(..).collect::<Vec<_>>();
            for waker in receivers {
                waker.wake();
            }
        }
    }
}

pub struct Receiver<T> {
    inner: Arc<BroadcastInner<T>>,
}

impl<T> Receiver<T> {
    pub fn recv(&self) -> BroadcastRecv<'_, T> {
        BroadcastRecv { receiver: self }
    }

    pub fn borrow(&self) -> T {
        unsafe { (*self.inner.value.get()).clone() }
    }

    pub fn try_recv(&self) -> Option<T> {
        let v = self.inner.version.load(Acquire);
        if v > 0 {
            Some(unsafe { (*self.inner.value.get()).clone() })
        } else {
            None
        }
    }
}

pub struct BroadcastRecv<'a, T> {
    receiver: &'a Receiver<T>,
}

impl<T> Future for BroadcastRecv<'_, T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let v = this.receiver.inner.version.load(Acquire);
        if v > 0 {
            return Poll::Ready(unsafe { (*this.receiver.inner.value.get()).clone() });
        }
        unsafe {
            (*this.receiver.inner.receivers.get()).push(cx.waker().clone());
        }
        Poll::Pending
    }
}