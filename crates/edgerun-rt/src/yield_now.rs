//! Yield — cooperatively yield the current task back to the reactor.
//!
//! Uses a thread-local flag to signal the worker, avoiding the expensive
//! shared-mutex waker path. This eliminates contention when many tasks
//! yield simultaneously.

use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

// Thread-local flag set by `yieldnow` to request immediate re-enqueue.
// The worker checks this after each poll. If true, the task is pushed
// back to the ready queue **without** going through the waker (which
// would take a shared mutex).
thread_local! {
    static YIELD_NOW: Cell<bool> = const { Cell::new(false) };
}

/// Check and consume the yield request flag from the current worker thread.
/// Returns `true` if the last poll called `yield_now`.
#[inline]
pub(crate) fn take_yield_request() -> bool {
    YIELD_NOW.replace(false)
}

/// Yields the current task back to the reactor, allowing other ready
/// tasks to make progress.
///
/// On first poll, returns `Pending` and signals the worker to re-enqueue
/// the task. On second poll, returns `Ready(())`.
pub fn yieldnow() -> YieldNow {
    YieldNow { done: false }
}

/// Future returned by [`yieldnow()`].
pub struct YieldNow {
    done: bool,
}

impl Future for YieldNow {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };

        if this.done {
            return Poll::Ready(());
        }

        this.done = true;
        // Signal the worker to re-enqueue us immediately.
        // This avoids calling `cx.waker().wake_by_ref()` which would
        // take a shared mutex on the global ready queue — the main source
        // of contention when many tasks yield simultaneously.
        YIELD_NOW.set(true);

        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static NOOP_WAKER: std::sync::LazyLock<std::task::Waker> =
        std::sync::LazyLock::new(|| {
            static VTABLE: std::task::RawWakerVTable =
                std::task::RawWakerVTable::new(clone_noop, wake_noop, wake_noop, drop_noop);
            const fn clone_noop(_: *const ()) -> std::task::RawWaker {
                std::task::RawWaker::new(std::ptr::null(), &VTABLE)
            }
            const fn wake_noop(_: *const ()) {}
            const fn drop_noop(_: *const ()) {}
            unsafe {
                std::task::Waker::from_raw(std::task::RawWaker::new(
                    std::ptr::null(),
                    &VTABLE,
                ))
            }
        });

    fn cx() -> Context<'static> {
        Context::from_waker(&NOOP_WAKER)
    }

    #[test]
    fn yield_first_pending() {
        let mut fut = yieldnow();
        assert!(matches!(Pin::new(&mut fut).poll(&mut cx()), Poll::Pending));
        assert!(fut.done);
    }

    #[test]
    fn yield_second_ready() {
        let mut fut = yieldnow();
        assert!(matches!(Pin::new(&mut fut).poll(&mut cx()), Poll::Pending));
        assert!(matches!(Pin::new(&mut fut).poll(&mut cx()), Poll::Ready(())));
    }
}
