//! Watch channel - single-value broadcast with version tracking.


extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use core::task::{Context, Poll, Waker};

const ACQUIRE: Ordering = Ordering::Acquire;
const RELEASE: Ordering = Ordering::Release;

// ===========================================================================
// Watch channel
// ===========================================================================

struct WatchInner<T> {
    value: UnsafeCell<T>,
    version: AtomicU64,
    waiters: UnsafeCell<Vec<Waker>>,
    closed: AtomicBool,
}

/// Creates a new watch channel with the initial value.
pub fn channel<T: Clone + Send>(initial: T) -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(WatchInner {
        value: UnsafeCell::new(initial),
        version: AtomicU64::new(1),
        waiters: UnsafeCell::new(Vec::new()),
        closed: AtomicBool::new(false),
    });
    (Sender { inner: inner.clone() }, Receiver { inner })
}

#[derive(Debug)]
pub struct ClosedError;

pub struct Sender<T> {
    inner: Arc<WatchInner<T>>,
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: Clone> Sender<T> {
    pub fn send_replace(&self, value: T) {
        unsafe {
            *self.inner.value.get() = value;
        }
        self.inner.version.fetch_add(1, RELEASE);
        unsafe {
            let waiters = (*self.inner.waiters.get()).drain(..).collect::<Vec<_>>();
            for waker in waiters {
                waker.wake();
            }
        }
    }

    pub fn close(&self) {
        self.inner.closed.store(true, RELEASE);
        unsafe {
            let waiters = (*self.inner.waiters.get()).drain(..).collect::<Vec<_>>();
            for waker in waiters {
                waker.wake();
            }
        }
    }
}

pub struct Receiver<T> {
    inner: Arc<WatchInner<T>>,
}

impl<T: Clone> Receiver<T> {
    pub fn borrow(&self) -> Result<T, ClosedError> {
        if self.inner.closed.load(ACQUIRE) {
            return Err(ClosedError);
        }
        Ok(unsafe { (*self.inner.value.get()).clone() })
    }

    pub fn has_changed(&self) -> bool {
        self.inner.version.load(ACQUIRE) > 0
    }

    pub fn changed(&mut self) -> Changed<'_, T> {
        Changed {
            receiver: self,
            registered: false,
        }
    }
}

pub struct Changed<'a, T> {
    receiver: &'a Receiver<T>,
    registered: bool,
}

impl<T: Clone> Future for Changed<'_, T> {
    type Output = Result<(), ClosedError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        if this.receiver.inner.closed.load(ACQUIRE) {
            return Poll::Ready(Err(ClosedError));
        }
        if this.receiver.has_changed() {
            return Poll::Ready(Ok(()));
        }
        if !this.registered {
            unsafe {
                (*this.receiver.inner.waiters.get()).push(cx.waker().clone());
            }
            this.registered = true;
        }
        Poll::Pending
    }
}