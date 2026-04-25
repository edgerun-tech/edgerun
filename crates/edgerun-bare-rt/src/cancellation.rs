//! Cancellation token - cooperative cancellation signal.


extern crate alloc;

use alloc::sync::Arc;
use core::cell::UnsafeCell;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::{Context, Poll, Waker};

const ACQUIRE: Ordering = Ordering::Acquire;
const RELEASE: Ordering = Ordering::Release;

// ===========================================================================
// CancellationToken
// ===========================================================================

struct CancelInner {
    cancelled: AtomicBool,
    waker: UnsafeCell<Option<Waker>>,
}

impl CancelInner {
    fn new() -> Self {
        Self {
            cancelled: AtomicBool::new(false),
            waker: UnsafeCell::new(None),
        }
    }
}

/// A cooperative cancellation token that can be cloned and shared.
#[derive(Clone)]
pub struct CancellationToken {
    inner: Arc<CancelInner>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(CancelInner::new()),
        }
    }

    pub fn cancel(&self) {
        self.inner.cancelled.store(true, RELEASE);
        unsafe {
            if let Some(w) = (*self.inner.waker.get()).take() {
                w.wake();
            }
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(ACQUIRE)
    }

    pub fn cancelled(&self) -> Cancelled<'_> {
        Cancelled { token: self }
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Future returned by [`CancellationToken::cancelled()`].
pub struct Cancelled<'a> {
    token: &'a CancellationToken,
}

impl Future for Cancelled<'_> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if this.token.inner.cancelled.load(ACQUIRE) {
            return Poll::Ready(());
        }
        unsafe {
            *this.token.inner.waker.get() = Some(cx.waker().clone());
        }
        if this.token.inner.cancelled.load(ACQUIRE) {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}