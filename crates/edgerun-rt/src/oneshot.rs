//! Async oneshot channel — single-value, single-use communication.
//!
//! A `Sender` sends exactly one value to a `Receiver`.
//! The receiver gets the value once; further sends fail.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use crate::sync::{Condvar, Mutex};
use std::task::{Context, Poll, Waker};

/// Creates a new oneshot channel.
pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(OneInner {
        val: Mutex::new(None),
        waker: Mutex::new(None),
        sender_dropped: Mutex::new(false),
        cvar: Condvar::new(),
    });
    (
        Sender { inner: Some(inner.clone()) },
        Receiver { inner },
    )
}

struct OneInner<T> {
    val: Mutex<Option<T>>,
    waker: Mutex<Option<Waker>>,
    /// Tracks whether the sender was dropped without sending.
    sender_dropped: Mutex<bool>,
    cvar: Condvar,
}

/// Oneshot sender. Can send at most one value.
pub struct Sender<T> {
    inner: Option<Arc<OneInner<T>>>,
}

impl<T> Sender<T> {
    /// Sends a value to the receiver. Can only be called once.
    pub fn send(mut self, val: T) -> Result<(), T> {
        let inner = std::mem::take(&mut self.inner);
        let inner = match inner {
            Some(i) => i,
            None => return Err(val),
        };
        let mut guard = inner.val.lock();
        if guard.is_some() {
            return Err(val);
        }
        *guard = Some(val);
        inner.cvar.notify_all();
        if let Some(w) = inner.waker.lock().take() {
            w.wake();
        }
        Ok(())
    }

    /// Returns true if the sender has not yet been used.
    pub fn is_unused(&self) -> bool {
        self.inner.is_some()
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        if let Some(inner) = &self.inner {
            *inner.sender_dropped.lock() = true;
            inner.cvar.notify_all();
            if let Some(w) = inner.waker.lock().take() {
                w.wake();
            }
        }
    }
}

/// Oneshot receiver. Receives exactly one value.
pub struct Receiver<T> {
    inner: Arc<OneInner<T>>,
}

impl<T> Receiver<T> {
    /// Receives the value. Returns `Err(RecvError)` if the sender
    /// was dropped without sending.
    pub fn blocking_recv(self) -> Result<T, RecvError> {
        let mut guard = self.inner.val.lock();
        loop {
            if let Some(v) = guard.take() {
                return Ok(v);
            }
            if *self.inner.sender_dropped.lock() {
                if let Some(v) = guard.take() {
                    return Ok(v);
                }
                return Err(RecvError);
            }
            self.inner.cvar.wait(&mut guard);
        }
    }
}

impl<T> Future for Receiver<T> {
    type Output = Result<T, RecvError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(v) = self.inner.val.lock().take() {
            return Poll::Ready(Ok(v));
        }
        if *self.inner.sender_dropped.lock() {
            if let Some(v) = self.inner.val.lock().take() {
                return Poll::Ready(Ok(v));
            }
            return Poll::Ready(Err(RecvError));
        }
        *self.inner.waker.lock() = Some(cx.waker().clone());

        // Double-check after registering waker.
        if let Some(v) = self.inner.val.lock().take() {
            return Poll::Ready(Ok(v));
        }
        if *self.inner.sender_dropped.lock() {
            if let Some(v) = self.inner.val.lock().take() {
                return Poll::Ready(Ok(v));
            }
            return Poll::Ready(Err(RecvError));
        }

        Poll::Pending
    }
}

/// Error returned when the sender was dropped without sending.
#[derive(Debug)]
pub struct RecvError;

impl std::fmt::Display for RecvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sender dropped")
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
        Context::from_waker(&NOOP_WAKER)
    }

    #[test]
    fn oneshot_send_recv() {
        let (tx, rx) = channel::<i32>();
        tx.send(42).unwrap();
        let mut rx = rx;
        assert!(matches!(Pin::new(&mut rx).poll(&mut cx()), Poll::Ready(Ok(42))));
    }

    #[test]
    fn oneshot_recv_pending() {
        let (_tx, rx) = channel::<i32>();
        let mut rx = rx;
        assert!(matches!(Pin::new(&mut rx).poll(&mut cx()), Poll::Pending));
    }

    #[test]
    fn oneshot_sender_dropped() {
        let (tx, rx) = channel::<i32>();
        drop(tx);
        let mut rx = rx;
        assert!(matches!(Pin::new(&mut rx).poll(&mut cx()), Poll::Ready(Err(_))));
    }

    #[test]
    fn oneshot_double_send_fails() {
        let (tx, rx) = channel::<i32>();
        tx.send(42).unwrap();
        // Can't call send again — sender consumed by send().
        // But verify the receiver got the value.
        let mut rx = rx;
        assert!(matches!(Pin::new(&mut rx).poll(&mut cx()), Poll::Ready(Ok(42))));
    }
}
