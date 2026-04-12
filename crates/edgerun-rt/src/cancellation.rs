//! Cancellation token — cooperative cancellation signal.
//!
//! A `CancellationToken` can be cloned and shared across tasks. When
//! `cancel()` is called, all clones observe the cancellation.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use parking_lot::{Condvar, Mutex};
use std::task::{Context, Poll, Waker};

struct Inner {
    cancelled: AtomicBool,
    wakers: Mutex<Vec<Waker>>,
    cvar: Condvar,
}

/// A cooperative cancellation token that can be cloned and shared.
#[derive(Clone)]
pub struct CancellationToken {
    inner: Arc<Inner>,
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                cancelled: AtomicBool::new(false),
                wakers: Mutex::new(Vec::new()),
                cvar: Condvar::new(),
            }),
        }
    }

    /// Cancel all tasks waiting on this token.
    pub fn cancel(&self) {
        self.inner.cancelled.store(true, Ordering::Release);
        let wakers = self.inner.wakers.lock().drain(..).collect::<Vec<_>>();
        for w in wakers {
            w.wake();
        }
        self.inner.cvar.notify_all();
    }

    /// Returns `true` if the token has been cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::Acquire)
    }

    /// Wait until the token is cancelled.
    pub fn cancelled(&self) -> Cancelled<'_> {
        Cancelled { token: self, registered: false }
    }

    /// Blocking wait until cancelled.
    pub fn wait(&self) {
        let mut guard = self.inner.wakers.lock();
        while !self.inner.cancelled.load(Ordering::Acquire) {
            self.inner.cvar.wait(&mut guard);
        }
    }

    /// Create a child token that is cancelled when the parent is cancelled.
    pub fn child_token(&self) -> CancellationToken {
        // Children share the same inner — cancelling the parent
        // cancels all children.
        self.clone()
    }
}

/// Future returned by [`CancellationToken::cancelled()`].
pub struct Cancelled<'a> {
    token: &'a CancellationToken,
    registered: bool,
}

impl Future for Cancelled<'_> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };

        if this.token.inner.cancelled.load(Ordering::Acquire) {
            return Poll::Ready(());
        }

        if !this.registered {
            this.token.inner.wakers.lock().push(cx.waker().clone());
            this.registered = true;

            // Double-check after registering.
            if this.token.inner.cancelled.load(Ordering::Acquire) {
                return Poll::Ready(());
            }
        }

        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static NOOP_WAKER: std::sync::LazyLock<Waker> =
        std::sync::LazyLock::new(|| {
            static VTABLE: std::task::RawWakerVTable =
                std::task::RawWakerVTable::new(clone_noop, wake_noop, wake_noop, drop_noop);
            const fn clone_noop(_: *const ()) -> std::task::RawWaker {
                std::task::RawWaker::new(std::ptr::null(), &VTABLE)
            }
            const fn wake_noop(_: *const ()) {}
            const fn drop_noop(_: *const ()) {}
            unsafe {
                Waker::from_raw(std::task::RawWaker::new(
                    std::ptr::null(),
                    &VTABLE,
                ))
            }
        });

    fn cx() -> Context<'static> {
        Context::from_waker(&NOOP_WAKER)
    }

    #[test]
    fn token_not_cancelled_initially() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
    }

    #[test]
    fn token_cancelled_after_call() {
        let token = CancellationToken::new();
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn token_clone_shares_state() {
        let token = CancellationToken::new();
        let clone = token.clone();
        assert!(!clone.is_cancelled());
        token.cancel();
        assert!(clone.is_cancelled());
    }

    #[test]
    fn cancelled_ready_after_cancel() {
        let token = CancellationToken::new();
        token.cancel();
        let mut fut = token.cancelled();
        assert!(matches!(Pin::new(&mut fut).poll(&mut cx()), Poll::Ready(())));
    }

    #[test]
    fn cancelled_pending_before_cancel() {
        let token = CancellationToken::new();
        let mut fut = token.cancelled();
        assert!(matches!(Pin::new(&mut fut).poll(&mut cx()), Poll::Pending));
    }
}
