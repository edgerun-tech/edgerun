//! `Latch` - a countdown primitive that fires when the count reaches zero.

#![no_std]

extern crate alloc;

use alloc::sync::Arc;
use core::cell::UnsafeCell;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::task::{Context, Poll, Waker};

const AcqRel: Ordering = Ordering::AcqRel;
const Acquire: Ordering = Ordering::Acquire;
const Release: Ordering = Ordering::Release;

// ===========================================================================
// Latch
// ===========================================================================

struct LatchInner {
    count: AtomicUsize,
    waker: UnsafeCell<Option<Waker>>,
}

/// A countdown latch that fires when the count reaches zero.
pub struct Latch {
    inner: Arc<LatchInner>,
}

impl Latch {
    pub fn new(count: usize) -> Self {
        Self {
            inner: Arc::new(LatchInner {
                count: AtomicUsize::new(count),
                waker: UnsafeCell::new(None),
            }),
        }
    }

    pub fn count(&self) -> usize {
        self.inner.count.load(Acquire)
    }

    pub fn count_down(&self) {
        let prev = self.inner.count.fetch_sub(1, AcqRel);
        if prev == 1 {
            unsafe {
                if let Some(w) = (*self.inner.waker.get()).take() {
                    w.wake();
                }
            }
        }
    }

    pub fn reached_zero(&self) -> bool {
        self.inner.count.load(Acquire) == 0
    }

    pub fn wait(&self) -> WaitLatch<'_> {
        WaitLatch { latch: self }
    }
}

impl Clone for Latch {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

/// Future returned by [`Latch::wait`].
pub struct WaitLatch<'a> {
    latch: &'a Latch,
}

impl Future for WaitLatch<'_> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if this.latch.reached_zero() {
            return Poll::Ready(());
        }
        unsafe {
            *this.latch.inner.waker.get() = Some(cx.waker().clone());
        }
        if this.latch.reached_zero() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}