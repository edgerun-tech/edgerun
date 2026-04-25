//! Broadcast channel — multi-producer, multi-consumer.
//!
//! Every receiver gets every message. If a receiver falls behind (can't
//! keep up with the sender), `recv()` returns `Err(RecvError::Lagged(n))`
//! where `n` is the number of missed messages.

use crate::sync::{Condvar, Mutex};
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

struct BroadcastInner<T> {
    buf: VecDeque<T>,
    cap: usize,
    /// Total messages ever produced (including dropped from front).
    produced: usize,
    senders: usize,
    closed: bool,
    recv_wakers: VecDeque<(usize, Waker)>,
    cvar: Condvar,
}

/// Create a new broadcast channel with the given capacity.
pub fn channel<T: Clone>(cap: usize) -> (Sender<T>, Receiver<T>) {
    Sender::new(cap)
}

/// Sender half of a broadcast channel.
pub struct Sender<T> {
    inner: Arc<Mutex<BroadcastInner<T>>>,
    cvar: Arc<Condvar>,
}

impl<T: Clone> Sender<T> {
    pub fn new(cap: usize) -> (Self, Receiver<T>) {
        let cap = cap.max(1);
        let inner = Arc::new(Mutex::new(BroadcastInner {
            buf: VecDeque::with_capacity(cap),
            cap,
            produced: 0,
            senders: 1,
            closed: false,
            recv_wakers: VecDeque::new(),
            cvar: Condvar::new(),
        }));
        let cvar = Arc::new(Condvar::new());
        let sender = Sender {
            inner: inner.clone(),
            cvar: cvar.clone(),
        };
        let receiver = Receiver {
            inner,
            cvar,
            next: 0,
        };
        (sender, receiver)
    }

    pub fn send(&self, value: T) -> Result<usize, SendError<T>> {
        let mut inner = self.inner.lock();
        if inner.closed {
            return Err(SendError(value));
        }
        inner.buf.push_back(value);
        inner.produced += 1;
        if inner.buf.len() > inner.cap {
            inner.buf.pop_front();
        }
        let count = inner.recv_wakers.len();
        let waiters = std::mem::take(&mut inner.recv_wakers);
        drop(inner);
        for (_, waker) in waiters {
            waker.wake();
        }
        self.cvar.notify_all();
        Ok(count)
    }

    pub fn receiver_count(&self) -> usize {
        // We don't track receiver count precisely; this is a best-effort.
        0
    }

    pub fn close(&mut self) {
        let mut inner = self.inner.lock();
        inner.closed = true;
        let waiters = std::mem::take(&mut inner.recv_wakers);
        drop(inner);
        for (_, waker) in waiters {
            waker.wake();
        }
        self.cvar.notify_all();
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        self.inner.lock().senders += 1;
        Self {
            inner: self.inner.clone(),
            cvar: self.cvar.clone(),
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let mut inner = self.inner.lock();
        inner.senders -= 1;
        if inner.senders == 0 {
            inner.closed = true;
            let waiters = std::mem::take(&mut inner.recv_wakers);
            drop(inner);
            for (_, waker) in waiters {
                waker.wake();
            }
        }
    }
}

/// Receiver half of a broadcast channel.
pub struct Receiver<T> {
    inner: Arc<Mutex<BroadcastInner<T>>>,
    cvar: Arc<Condvar>,
    /// Next message index to receive.
    next: usize,
}

impl<T: Clone> Receiver<T> {
    /// Create a new receiver that starts from the current end (misses past messages).
    pub fn resubscribe(&self) -> Self {
        let inner = self.inner.lock();
        let start = inner.produced;
        drop(inner);
        Receiver {
            inner: self.inner.clone(),
            cvar: self.cvar.clone(),
            next: start,
        }
    }

    pub fn blocking_recv(&mut self) -> Result<T, RecvError> {
        let mut inner = self.inner.lock();
        loop {
            let start = inner.produced.saturating_sub(inner.buf.len());
            if self.next < start {
                // Fell behind — return lag error.
                let skipped = start - self.next;
                return Err(RecvError::Lagged(skipped));
            }
            let idx = self.next.saturating_sub(start);
            if idx < inner.buf.len() {
                let val = inner.buf[idx].clone();
                self.next += 1;
                return Ok(val);
            }
            if inner.closed {
                return Err(RecvError::Closed);
            }
            self.cvar.wait(&mut inner);
        }
    }

    fn try_pop(&self, inner: &mut BroadcastInner<T>) -> Option<T> {
        let start = inner.produced.saturating_sub(inner.buf.len());
        if self.next < start {
            // Fell behind — return the oldest available message and report lag.
            // We still return the value; the lag info is lost in this API.
            return inner.buf.front().cloned();
        }
        let idx = self.next.saturating_sub(start);
        if idx < inner.buf.len() {
            Some(inner.buf[idx].clone())
        } else {
            None
        }
    }

    pub fn try_recv(&mut self) -> Result<T, TryRecvError> {
        let mut inner = self.inner.lock();
        let start = inner.produced.saturating_sub(inner.buf.len());
        if self.next < start {
            let skipped = start - self.next;
            return Err(TryRecvError::Lagged(skipped));
        }
        let idx = self.next.saturating_sub(start);
        if idx < inner.buf.len() {
            let val = inner.buf[idx].clone();
            self.next += 1;
            Ok(val)
        } else if inner.closed {
            Err(TryRecvError::Closed)
        } else {
            Err(TryRecvError::Empty)
        }
    }

    pub fn recv(&mut self) -> RecvFut<'_, T> {
        RecvFut {
            inner: &self.inner,
            next: &mut self.next,
        }
    }
}

/// Error returned by `send()`.
#[derive(Debug)]
pub struct SendError<T>(pub T);

/// Error returned by `recv()` / `blocking_recv()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecvError {
    Closed,
    /// Receiver fell behind; the value is the number of skipped messages.
    Lagged(usize),
}

/// Error returned by `try_recv()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TryRecvError {
    Empty,
    Closed,
    /// Receiver fell behind; the value is the number of skipped messages.
    Lagged(usize),
}

pub struct RecvFut<'a, T> {
    inner: &'a Mutex<BroadcastInner<T>>,
    next: &'a mut usize,
}

impl<T: Clone> Future for RecvFut<'_, T> {
    type Output = Result<T, RecvError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let mut inner = this.inner.lock();
        let start = inner.produced.saturating_sub(inner.buf.len());
        let idx = (*this.next).saturating_sub(start);

        if *this.next < start {
            // Fell behind — return lag error.
            let skipped = start - *this.next;
            return Poll::Ready(Err(RecvError::Lagged(skipped)));
        } else if idx < inner.buf.len() {
            let val = inner.buf[idx].clone();
            *this.next += 1;
            return Poll::Ready(Ok(val));
        }

        if inner.closed {
            return Poll::Ready(Err(RecvError::Closed));
        }

        // Always re-register: send() drains the waker Vec to wake waiters,
        // so after being woken our waker is no longer in the queue.
        // Stale wakers are harmless — they're drained on the next send().
        inner
            .recv_wakers
            .push_back((*this.next, cx.waker().clone()));

        Poll::Pending
    }
}
