//! Async channels for bare-metal

extern crate alloc;

use alloc::sync::Arc;
use core::cell::UnsafeCell;
use core::pin::Pin;
use core::future::Future;
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::{Context, Poll};

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(Inner {
        data: UnsafeCell::new(None),
        sent: AtomicBool::new(false),
    });
    (Sender { inner: inner.clone() }, Receiver { inner })
}

struct Inner<T> {
    data: UnsafeCell<Option<T>>,
    sent: AtomicBool,
}

pub struct Sender<T> {
    inner: Arc<Inner<T>>,
}

impl<T> Sender<T> {
    pub fn send(&self, value: T) -> Result<(), SendError<T>> {
        if self.inner.sent.load(Ordering::Acquire) {
            return Err(SendError(value));
        }
        unsafe { *self.inner.data.get() = Some(value) };
        self.inner.sent.store(true, Ordering::Release);
        Ok(())
    }

    pub fn is_closed(&self) -> bool {
        self.inner.sent.load(Ordering::Acquire)
    }
}

pub struct SendError<T>(pub T);

impl<T> core::fmt::Debug for SendError<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "SendError(..)")
    }
}

impl<T> core::fmt::Display for SendError<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "channel closed")
    }
}

pub struct Receiver<T> {
    inner: Arc<Inner<T>>,
}

impl<T> Future for Receiver<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.inner.sent.load(Ordering::Acquire) {
            let ptr = self.inner.data.get();
            unsafe {
                if (*ptr).is_some() {
                    return Poll::Ready((*ptr).take().unwrap());
                }
            }
        }
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

impl<T> Receiver<T> {
    pub fn try_recv(&self) -> Option<T> {
        if self.inner.sent.load(Ordering::Acquire) {
            unsafe { self.inner.data.get().as_mut()?.take() }
        } else {
            None
        }
    }

    pub fn close(&self) {
        self.inner.sent.store(true, Ordering::Release);
    }
}