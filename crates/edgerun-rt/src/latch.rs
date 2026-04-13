//! `Latch` — a countdown primitive that fires when the count reaches zero.
//!
//! Unlike [`Barrier`](crate::Barrier), which waits for exactly N tasks to arrive,
//! a `Latch` can be decremented by any number of callers from anywhere.
//! Once the count reaches zero, all waiters are released.
//!
//! Useful for startup synchronization, waiting for N events to complete,
//! or gate-keeping operations until prerequisites are met.
//!
//! # Example
//! ```ignore
//! let latch = Latch::new(3);
//!
//! // Decrement from various places.
//! latch.count_down();
//! latch.count_down();
//! latch.count_down();
//!
//! // Wait until all three decrements have happened.
//! latch.wait().await;
//! ```

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};

use crate::sync::Mutex;

struct Inner {
    count: AtomicUsize,
    wakers: Mutex<Vec<Waker>>,
}

/// A countdown latch that fires when the count reaches zero.
pub struct Latch {
    inner: Arc<Inner>,
}

impl Latch {
    /// Creates a new `Latch` with the given count.
    pub fn new(count: usize) -> Self {
        Self {
            inner: Arc::new(Inner {
                count: AtomicUsize::new(count),
                wakers: Mutex::new(Vec::new()),
            }),
        }
    }

    /// Returns the current count.
    pub fn count(&self) -> usize {
        self.inner.count.load(Ordering::Acquire)
    }

    /// Decrements the count by one.
    ///
    /// If the count reaches zero, all waiters are woken.
    /// Calling `count_down` when the count is already zero is a no-op.
    pub fn count_down(&self) {
        let prev = self.inner.count.fetch_sub(1, Ordering::AcqRel);
        if prev == 1 {
            // Count went from 1 to 0 — wake all waiters.
            let wakers = self.inner.wakers.lock().drain(..).collect::<Vec<_>>();
            for w in wakers {
                w.wake();
            }
        }
    }

    /// Decrements the count by `n`.
    ///
    /// If the count reaches zero (or goes below), all waiters are woken.
    /// Calling `count_down_n` when the count is already zero is a no-op.
    pub fn count_down_n(&self, n: usize) {
        if n == 0 {
            return;
        }
        // Use saturating_sub to prevent usize underflow.
        let prev = self.inner.count.fetch_update(
            Ordering::AcqRel,
            Ordering::Acquire,
            |count| Some(count.saturating_sub(n)),
        ).unwrap_or(0);
        if prev <= n {
            let wakers = self.inner.wakers.lock().drain(..).collect::<Vec<_>>();
            for w in wakers {
                w.wake();
            }
        }
    }

    /// Returns `true` if the count has reached zero.
    pub fn reached_zero(&self) -> bool {
        self.inner.count.load(Ordering::Acquire) == 0
    }

    /// Returns a future that resolves when the count reaches zero.
    ///
    /// Returns immediately if the count is already zero.
    pub fn wait(&self) -> WaitLatch<'_> {
        WaitLatch { latch: self }
    }

    /// Attempts to wait without blocking.
    ///
    /// Returns `true` if the count has already reached zero.
    pub fn try_wait(&self) -> bool {
        self.reached_zero()
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
        if self.latch.reached_zero() {
            return Poll::Ready(());
        }
        self.latch.inner.wakers.lock().push(cx.waker().clone());
        // Re-check after registering to avoid a race.
        if self.latch.reached_zero() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}
