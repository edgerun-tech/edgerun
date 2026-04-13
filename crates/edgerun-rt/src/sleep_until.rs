//! Sleep until a specific [`Instant`] — duration-independent sleep.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use crate::Instant;
use crate::Elapsed;

/// Sleep until the given deadline.
pub fn sleep_until(deadline: Instant) -> SleepUntil {
    SleepUntil { deadline, registered: false }
}

/// Future returned by [`sleep_until()`].
pub struct SleepUntil {
    deadline: Instant,
    registered: bool,
}

impl SleepUntil {
    /// Returns the deadline for this sleep.
    pub fn deadline(&self) -> Instant {
        self.deadline
    }
}

impl Future for SleepUntil {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if Instant::now() >= self.deadline {
            Poll::Ready(())
        } else {
            let this = unsafe { self.get_unchecked_mut() };
            if !this.registered {
                crate::runtime::current_rt()
                    .reactor
                    .register_timer(this.deadline, cx.waker().clone());
                this.registered = true;
            }
            Poll::Pending
        }
    }
}

/// Timeout at a specific [`Instant`] instead of after a [`Duration`].
pub fn timeout_at<F>(deadline: Instant, f: F) -> TimeoutAt<F> {
    TimeoutAt { inner: Some(f), deadline, registered: false }
}

/// Future returned by [`timeout_at()`].
pub struct TimeoutAt<F> {
    inner: Option<F>,
    deadline: Instant,
    registered: bool,
}

impl<F: Future> Future for TimeoutAt<F> {
    type Output = Result<F::Output, Elapsed>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };

        if Instant::now() >= this.deadline {
            return Poll::Ready(Err(crate::timers::Elapsed));
        }

        let fut = match this.inner.as_mut() {
            Some(f) => f,
            None => return Poll::Ready(Err(crate::timers::Elapsed)),
        };

        match unsafe { Pin::new_unchecked(fut) }.poll(cx) {
            Poll::Ready(v) => Poll::Ready(Ok(v)),
            Poll::Pending => {
                // Only register timer once.
                if !this.registered {
                    crate::runtime::current_rt()
                        .reactor
                        .register_timer(this.deadline, cx.waker().clone());
                    this.registered = true;
                }
                Poll::Pending
            }
        }
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
    fn sleep_until_past() {
        let deadline = Instant::now() - Duration::from_millis(1);
        let mut fut = sleep_until(deadline);
        assert!(matches!(Pin::new(&mut fut).poll(&mut cx()), Poll::Ready(())));
    }

    #[test]
    fn sleep_until_deadline() {
        let deadline = Instant::now() + Duration::from_millis(5);
        let fut = sleep_until(deadline);
        assert_eq!(fut.deadline(), deadline);
    }

    #[test]
    fn timeout_at_expired() {
        let deadline = Instant::now() - Duration::from_millis(1);
        let mut fut = timeout_at(deadline, std::future::ready(42));
        assert!(matches!(Pin::new(&mut fut).poll(&mut cx()), Poll::Ready(Err(_))));
    }

    #[test]
    fn timeout_at_ready() {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut fut = timeout_at(deadline, std::future::ready(42));
        assert!(matches!(Pin::new(&mut fut).poll(&mut cx()), Poll::Ready(Ok(42))));
    }
}
