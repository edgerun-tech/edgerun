//! Yield — cooperatively yield the current task back to the reactor.
//!
//! Useful for fair scheduling in tight loops where a task shouldn't
//! monopolize a worker thread. The task is woken immediately so it
//! will be re-scheduled, but other ready tasks get a chance to run
//! first (FIFO via the ready queue).

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::runtime::current_rt;

/// Yields the current task back to the reactor, allowing other ready
/// tasks to make progress.
///
/// On first poll, returns `Pending` and wakes itself. On second poll,
/// returns `Ready(())`.
pub fn yield_now() -> YieldNow {
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
        // Wake ourselves — the reactor's ready queue will re-schedule us
        // after other ready tasks have had a chance.
        cx.waker().wake_by_ref();

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
        let mut fut = yield_now();
        assert!(matches!(Pin::new(&mut fut).poll(&mut cx()), Poll::Pending));
        assert!(fut.done);
    }

    #[test]
    fn yield_second_ready() {
        let mut fut = yield_now();
        assert!(matches!(Pin::new(&mut fut).poll(&mut cx()), Poll::Pending));
        assert!(matches!(Pin::new(&mut fut).poll(&mut cx()), Poll::Ready(())));
    }
}
