//! Unbounded mpsc channel — no backpressure, always succeeds.
//!
//! Unlike the bounded `mpsc`, this channel never blocks the sender.
//! Use for control paths where dropping or backing up messages is
//! unacceptable (e.g., mesh commands, shutdown signals).

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crate::sync::{Condvar, Mutex};
use std::task::{Context, Poll, Waker};

/// Creates an unbounded mpsc channel.
pub fn channel<T>() -> (UnboundedSender<T>, UnboundedReceiver<T>) {
    let inner = Arc::new(UnboundedInner {
        q: Mutex::new(VecDeque::new()),
        recv_waker: Mutex::new(None),
        closed: AtomicBool::new(false),
        sender_cvar: Condvar::new(),
        sender_count: Mutex::new(1),
    });
    (
        UnboundedSender { inner: inner.clone() },
        UnboundedReceiver { inner },
    )
}

struct UnboundedInner<T> {
    q: Mutex<VecDeque<T>>,
    recv_waker: Mutex<Option<Waker>>,
    closed: AtomicBool,
    /// Used by `blocking_recv` to wait without spinning.
    sender_cvar: Condvar,
    sender_count: Mutex<usize>,
}

impl<T> UnboundedInner<T> {
    fn wake_receiver(&self) {
        if let Some(waker) = self.recv_waker.lock().take() {
            waker.wake();
        }
        self.sender_cvar.notify_one();
    }
}

/// Unbounded sender. Cloneable.
pub struct UnboundedSender<T> {
    inner: Arc<UnboundedInner<T>>,
}

impl<T> Clone for UnboundedSender<T> {
    fn clone(&self) -> Self {
        *self.inner.sender_count.lock() += 1;
        Self { inner: self.inner.clone() }
    }
}

impl<T> Drop for UnboundedSender<T> {
    fn drop(&mut self) {
        let mut count = self.inner.sender_count.lock();
        *count -= 1;
        if *count == 0 {
            self.inner.closed.store(true, Ordering::Release);
            self.inner.wake_receiver();
        }
    }
}

impl<T> UnboundedSender<T> {
    /// Sends a value. Never fails (unless the channel is closed).
    pub fn send(&self, val: T) -> Result<(), SendError<T>> {
        if self.inner.closed.load(Ordering::Relaxed) {
            return Err(SendError(val));
        }
        self.inner.q.lock().push_back(val);
        self.inner.wake_receiver();
        Ok(())
    }

    /// Returns the approximate number of messages in the channel.
    pub fn len(&self) -> usize {
        self.inner.q.lock().len()
    }

    /// Returns true if the channel has no messages.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns true if all senders have been dropped.
    pub fn is_closed(&self) -> bool {
        self.inner.closed.load(Ordering::Acquire)
    }
}

/// Unbounded receiver. Not cloneable (single consumer).
pub struct UnboundedReceiver<T> {
    inner: Arc<UnboundedInner<T>>,
}

impl<T> UnboundedReceiver<T> {
    /// Receives a value, blocking if the channel is empty.
    /// Returns `None` when all senders are dropped and the queue is empty.
    pub fn blocking_recv(&mut self) -> Option<T> {
        let mut q = self.inner.q.lock();
        loop {
            if let Some(v) = q.pop_front() {
                return Some(v);
            }
            if self.inner.closed.load(Ordering::Acquire) {
                return None;
            }
            self.inner.sender_cvar.wait(&mut q);
        }
    }

    /// Tries to receive a value without blocking.
    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        let mut q = self.inner.q.lock();
        if let Some(v) = q.pop_front() {
            return Ok(v);
        }
        if self.inner.closed.load(Ordering::Acquire) {
            Err(TryRecvError::Disconnected)
        } else {
            Err(TryRecvError::Empty)
        }
    }

    /// Async receive.
    pub fn recv(&self) -> UnboundedRecvFut<'_, T> {
        UnboundedRecvFut { inner: &self.inner }
    }
}

impl<T> Drop for UnboundedReceiver<T> {
    fn drop(&mut self) {
        self.inner.closed.store(true, Ordering::Release);
    }
}

/// Future returned by `UnboundedReceiver::recv()`.
pub struct UnboundedRecvFut<'a, T> {
    inner: &'a UnboundedInner<T>,
}

impl<T> Future for UnboundedRecvFut<'_, T> {
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut q = self.inner.q.lock();
        if let Some(v) = q.pop_front() {
            return Poll::Ready(Some(v));
        }
        if self.inner.closed.load(Ordering::Acquire) {
            return Poll::Ready(None);
        }
        *self.inner.recv_waker.lock() = Some(cx.waker().clone());

        // Double-check: a value may have arrived while we were registering.
        if let Some(v) = q.pop_front() {
            // Clear our waker.
            self.inner.recv_waker.lock().take();
            return Poll::Ready(Some(v));
        }
        if self.inner.closed.load(Ordering::Acquire) {
            return Poll::Ready(None);
        }

        Poll::Pending
    }
}

/// Error returned by `UnboundedSender::send()` when the channel is closed.
#[derive(Debug)]
pub struct SendError<T>(pub T);

impl<T> std::fmt::Display for SendError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "channel closed")
    }
}

/// Error returned by `UnboundedReceiver::try_recv()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TryRecvError {
    /// The channel is empty but senders still exist.
    Empty,
    /// All senders are dropped and the queue is empty.
    Disconnected,
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
    fn unbounded_send_recv() {
        let (tx, rx) = channel::<i32>();
        tx.send(42).unwrap();
        assert_eq!(rx.try_recv(), Ok(42));
    }

    #[test]
    fn unbounded_multiple() {
        let (tx, rx) = channel::<i32>();
        tx.send(1).unwrap();
        tx.send(2).unwrap();
        tx.send(3).unwrap();
        assert_eq!(rx.try_recv(), Ok(1));
        assert_eq!(rx.try_recv(), Ok(2));
        assert_eq!(rx.try_recv(), Ok(3));
    }

    #[test]
    fn unbounded_recv_pending() {
        let (_tx, rx) = channel::<i32>();
        let mut fut = rx.recv();
        assert_eq!(Pin::new(&mut fut).poll(&mut cx()), Poll::Pending);
    }

    #[test]
    fn unbounded_recv_ready() {
        let (tx, rx) = channel::<i32>();
        tx.send(99).unwrap();
        let mut fut = rx.recv();
        assert_eq!(Pin::new(&mut fut).poll(&mut cx()), Poll::Ready(Some(99)));
    }

    #[test]
    fn unbounded_disconnected() {
        let (tx, rx) = channel::<i32>();
        tx.send(1).unwrap();
        drop(tx);
        let mut rx = rx;
        assert_eq!(rx.try_recv(), Ok(1));
        assert_eq!(rx.try_recv(), Err(TryRecvError::Disconnected));
    }

    #[test]
    fn unbounded_clone_send() {
        let (tx, rx) = channel::<i32>();
        let tx2 = tx.clone();
        tx.send(1).unwrap();
        tx2.send(2).unwrap();
        drop(tx);
        drop(tx2);
        let mut rx = rx;
        assert_eq!(rx.try_recv(), Ok(1));
        assert_eq!(rx.try_recv(), Ok(2));
        assert_eq!(rx.try_recv(), Err(TryRecvError::Disconnected));
    }
}
