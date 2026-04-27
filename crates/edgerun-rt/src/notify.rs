//! Async notification primitive.
//!
//! A `Notify` allows one or more tasks to be woken when notified.
//! Unlike `Condvar`, this is fully async — no thread blocking.
//! Multiple `notified()` futures can wait concurrently; a single
//! `notify()` wakes exactly one waiter (FIFO order).
//!
//! ## Thread safety
//! All state (permit counter + waiters queue) is under a single Mutex,
//! eliminating races between `notify_one()` and `Notified::poll()`.

use crate::sync::Mutex;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

struct NotifyInner {
    /// Number of unconsumed permits (pre-notifications from `notify_one()`
    /// before any `Notified` future existed).
    notified: usize,
    waiters: VecDeque<Waker>,
}

/// An async notification primitive. Thread-safe, can be cloned.
pub struct Notify {
    inner: std::sync::Arc<Mutex<NotifyInner>>,
}

impl Clone for Notify {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl Default for Notify {
    fn default() -> Self {
        Self::new()
    }
}

impl Notify {
    /// Creates a new `Notify` with zero pending notifications.
    pub fn new() -> Self {
        Self {
            inner: std::sync::Arc::new(Mutex::new(NotifyInner {
                notified: 0,
                waiters: VecDeque::new(),
            })),
        }
    }

    /// Waits for a notification. Returns immediately if there are pending
    /// notifications from prior `notify()` calls.
    pub fn notified(&self) -> Notified<'_> {
        Notified { notify: self }
    }

    /// Wakes one waiting task. If no task is waiting, increments a counter
    /// so the next `notified()` call returns immediately.
    pub fn notify_one(&self) {
        let mut inner = self.inner.lock();
        // Always increment the permit counter. If there's a waiter, the
        // woken task will find the counter > 0 and consume it. If no waiter,
        // the counter serves as a pre-notification for the next `notified()`.
        inner.notified += 1;
        if let Some(waker) = inner.waiters.pop_front() {
            drop(inner);
            waker.wake();
        }
    }

    /// Wakes all waiting tasks.
    pub fn notify_waiters(&self) {
        let mut inner = self.inner.lock();
        let count = inner.waiters.len();
        inner.notified += count;
        let waiters = std::mem::take(&mut inner.waiters);
        drop(inner);
        for waker in waiters {
            waker.wake();
        }
    }
}

/// A future that resolves when `Notify::notify_one()` is called.
pub struct Notified<'a> {
    notify: &'a Notify,
}

impl Future for Notified<'_> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let mut inner = this.notify.inner.lock();

        // Check for a pre-recorded permit.
        if inner.notified > 0 {
            inner.notified -= 1;
            return Poll::Ready(());
        }

        // No permit available — register waker. Always re-register because
        // `notify_one` consumes (pops) the waker to wake us, so on re-poll
        // our waker is no longer in the queue.
        inner.waiters.push_back(cx.waker().clone());

        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::cx;

    #[test]
    fn notify_pre_notified() {
        let n = Notify::new();
        n.notify_one();
        let mut fut = n.notified();
        assert_eq!(Pin::new(&mut fut).poll(&mut cx()), Poll::Ready(()));
    }

    #[test]
    fn notify_pre_notified_multiple() {
        let n = Notify::new();
        n.notify_one();
        n.notify_one();
        n.notify_one();
        assert_eq!(Pin::new(&mut n.notified()).poll(&mut cx()), Poll::Ready(()));
        assert_eq!(Pin::new(&mut n.notified()).poll(&mut cx()), Poll::Ready(()));
        assert_eq!(Pin::new(&mut n.notified()).poll(&mut cx()), Poll::Ready(()));
        // Fourth should be pending
        assert_eq!(Pin::new(&mut n.notified()).poll(&mut cx()), Poll::Pending);
    }

    #[test]
    fn notify_waiters_pending() {
        let n = Notify::new();
        let mut fut = n.notified();
        assert_eq!(Pin::new(&mut fut).poll(&mut cx()), Poll::Pending);
    }

    #[test]
    fn notify_wake_waiter() {
        let n = Notify::new();
        let mut fut = n.notified();
        assert_eq!(Pin::new(&mut fut).poll(&mut cx()), Poll::Pending);
        n.notify_one();
        assert_eq!(Pin::new(&mut fut).poll(&mut cx()), Poll::Ready(()));
    }

    #[test]
    fn notify_multiple_waiters() {
        let n = Notify::new();
        let mut fut1 = n.notified();
        let mut fut2 = n.notified();
        let mut fut3 = n.notified();
        assert_eq!(Pin::new(&mut fut1).poll(&mut cx()), Poll::Pending);
        assert_eq!(Pin::new(&mut fut2).poll(&mut cx()), Poll::Pending);
        assert_eq!(Pin::new(&mut fut3).poll(&mut cx()), Poll::Pending);

        n.notify_one();
        assert_eq!(Pin::new(&mut fut1).poll(&mut cx()), Poll::Ready(()));
        assert_eq!(Pin::new(&mut fut2).poll(&mut cx()), Poll::Pending);
        assert_eq!(Pin::new(&mut fut3).poll(&mut cx()), Poll::Pending);

        n.notify_one();
        assert_eq!(Pin::new(&mut fut2).poll(&mut cx()), Poll::Ready(()));

        n.notify_waiters();
        assert_eq!(Pin::new(&mut fut3).poll(&mut cx()), Poll::Ready(()));
    }
}
