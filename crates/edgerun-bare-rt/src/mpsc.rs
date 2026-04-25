//! Bounded mpsc channel with backpressure.

#![no_std]

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use core::task::{Context, Poll, Waker};

const Acquire: Ordering = Ordering::Acquire;
const Release: Ordering = Ordering::Release;
const AcqRel: Ordering = Ordering::AcqRel;

// ===========================================================================
// MPSC channel
// ===========================================================================

struct MpscInner<T> {
    queue: UnsafeCell<Vec<T>>,
    closed: AtomicBool,
    recv_waker: UnsafeCell<Option<Waker>>,
}

/// Creates a new mpsc channel with the given capacity.
pub fn channel<T: Send>(cap: usize) -> (Sender<T>, Receiver<T>) {
    let cap = cap.max(1);
    let inner = Arc::new(MpscInner {
        queue: UnsafeCell::new(Vec::with_capacity(cap)),
        closed: AtomicBool::new(false),
        recv_waker: UnsafeCell::new(None),
    });
    (Sender { inner: inner.clone() }, Receiver { inner })
}

pub struct Sender<T> {
    inner: Arc<MpscInner<T>>,
}

unsafe impl<T: Send> Send for Sender<T> {}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        self.inner.closed.store(true, Release);
    }
}

impl<T> Sender<T> {
    pub fn try_send(&self, value: T) -> Result<(), SendError<T>> {
        if self.inner.closed.load(Ordering::Relaxed) {
            return Err(SendError(value));
        }
        let q = unsafe { &mut *self.inner.queue.get() };
        if q.len() >= q.capacity() {
            return Err(SendError(value));
        }
        q.push(value);
        unsafe {
            if let Some(w) = (*self.inner.recv_waker.get()).take() {
                w.wake();
            }
        }
        Ok(())
    }

    pub fn send(&self, value: T) -> Result<(), SendError<T>> {
        self.try_send(value)
    }
}

pub struct Receiver<T> {
    inner: Arc<MpscInner<T>>,
}

unsafe impl<T: Send> Send for Receiver<T> {}

impl<T> Receiver<T> {
    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        if self.inner.closed.load(Acquire) {
            unsafe {
                if let Some(v) = (*self.inner.queue.get()).pop() {
                    return Ok(v);
                }
            }
            return Err(TryRecvError::Disconnected);
        }
        unsafe {
            (*self.inner.queue.get()).pop().ok_or(TryRecvError::Empty)
        }
    }

    pub fn recv(&self) -> RecvFut<'_, T> {
        RecvFut { receiver: self }
    }
}

pub struct RecvFut<'a, T> {
    receiver: &'a Receiver<T>,
}

unsafe impl<T: Send> Send for RecvFut<'_, T> {}

impl<T> Future for RecvFut<'_, T> {
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if this.inner.closed.load(Acquire) {
            unsafe {
                if let Some(v) = (*this.inner.queue.get()).pop() {
                    return Poll::Ready(Some(v));
                }
            }
            return Poll::Ready(None);
        }
        unsafe {
            if let Some(v) = (*this.inner.queue.get()).pop() {
                return Poll::Ready(Some(v));
            }
        }
        unsafe { *this.inner.recv_waker.get() = Some(cx.waker().clone()) };
        if this.inner.closed.load(Acquire) {
            unsafe {
                if let Some(v) = (*this.inner.queue.get()).pop() {
                    return Poll::Ready(Some(v));
                }
            }
            return Poll::Ready(None);
        }
        Poll::Pending
    }
}

#[derive(Debug)]
pub struct SendError<T>(pub T);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TryRecvError {
    Empty,
    Disconnected,
}