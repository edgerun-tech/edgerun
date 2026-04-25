//! Async oneshot channel - single value sender/receiver.


extern crate alloc;

use alloc::sync::Arc;
use core::cell::UnsafeCell;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::{Context, Poll, Waker};

pub fn channel<T: Send>() -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(Inner {
        val: UnsafeCell::new(None),
        waker: UnsafeCell::new(None),
        closed: AtomicBool::new(false),
    });
    (Sender { inner: Some(inner.clone()) }, Receiver { inner })
}

struct Inner<T> {
    val: UnsafeCell<Option<T>>,
    waker: UnsafeCell<Option<Waker>>,
    closed: AtomicBool,
}

pub struct Sender<T> {
    inner: Option<Arc<Inner<T>>>,
}

impl<T: Send> Sender<T> {
    pub fn send(&mut self, val: T) -> Result<(), T> {
        match self.inner.take() {
            Some(inner) => {
                unsafe {
                    let ptr = inner.val.get();
                    if (*ptr).is_some() {
                        return Err(val);
                    }
                    *ptr = Some(val);
                }
                inner.closed.store(true, Ordering::Release);
                unsafe {
                    if let Some(w) = (*inner.waker.get()).take() {
                        w.wake();
                    }
                }
                Ok(())
            }
            None => Err(val),
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.as_ref() {
            inner.closed.store(true, Ordering::Release);
            unsafe {
                if let Some(w) = (*inner.waker.get()).take() {
                    w.wake();
                }
            }
        }
    }
}

pub struct Receiver<T> {
    inner: Arc<Inner<T>>,
}

impl<T: Send> Receiver<T> {
    pub fn blocking_recv(self) -> Result<T, RecvError> {
        loop {
            unsafe {
                if let Some(v) = (*self.inner.val.get()).take() {
                    return Ok(v);
                }
            }
            if self.inner.closed.load(Ordering::Acquire) {
                return Err(RecvError);
            }
            core::hint::spin_loop();
        }
    }
}

impl<T: Send> Future for Receiver<T> {
    type Output = Result<T, RecvError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe {
            if let Some(v) = (*self.inner.val.get()).take() {
                return Poll::Ready(Ok(v));
            }
        }
        if self.inner.closed.load(Ordering::Acquire) {
            return Poll::Ready(Err(RecvError));
        }
        unsafe {
            *self.inner.waker.get() = Some(cx.waker().clone());
        }
        Poll::Pending
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecvError;

impl core::fmt::Display for RecvError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "receiver closed")
    }
}

impl core::error::Error for RecvError {}