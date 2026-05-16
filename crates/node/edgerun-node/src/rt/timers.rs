//! Timer-based async primitives

use crate::rt::Instant;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::{Context, Poll, Waker};
use core::time::Duration;

pub struct Sleep {
    deadline: Instant,
    waker: Option<Waker>,
}

impl Unpin for Sleep {}

impl Sleep {
    pub fn new(deadline: Instant) -> Self {
        Self {
            deadline,
            waker: None,
        }
    }

    pub fn after(delay: Duration) -> Self {
        Self::new(Instant::now() + delay)
    }

    pub fn deadline(&self) -> Instant {
        self.deadline
    }

    pub fn is_ready(&self) -> bool {
        Instant::now() >= self.deadline
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if this.is_ready() {
            this.waker = None;
            Poll::Ready(())
        } else {
            if this
                .waker
                .as_ref()
                .map_or(true, |registered| !registered.will_wake(cx.waker()))
            {
                this.waker = Some(cx.waker().clone());
            }
            Poll::Pending
        }
    }
}

impl Drop for Sleep {
    fn drop(&mut self) {
        self.waker = None;
    }
}

pub async fn sleep(delay: Duration) {
    Sleep::after(delay).await;
}

pub fn sleep_until(deadline: Instant) -> Sleep {
    Sleep::new(deadline)
}

#[derive(Debug)]
pub struct Elapsed;

crate::rt::error::impl_error!(Elapsed, |_this, f| { write!(f, "timeout elapsed") });

pub struct Timeout<F> {
    inner: Option<F>,
    deadline: Instant,
    elapsed: AtomicBool,
}

impl<F> Unpin for Timeout<F> {}

impl<F> Timeout<F> {
    pub fn new(delay: Duration, future: F) -> Self {
        Self::at(Instant::now() + delay, future)
    }

    pub fn at(deadline: Instant, future: F) -> Self {
        Self {
            inner: Some(future),
            deadline,
            elapsed: AtomicBool::new(false),
        }
    }

    pub fn is_elapsed(&self) -> bool {
        self.elapsed.load(Ordering::Acquire)
    }

    pub fn into_inner(self) -> Option<F> {
        self.inner
    }
}

impl<F> Future for Timeout<F>
where
    F: Future,
{
    type Output = Result<F::Output, Elapsed>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        if Instant::now() >= this.deadline {
            this.elapsed.store(true, Ordering::Release);
            return Poll::Ready(Err(Elapsed));
        }

        if let Some(ref mut inner) = this.inner {
            return match unsafe { Pin::new_unchecked(inner) }.poll(cx) {
                Poll::Ready(v) => Poll::Ready(Ok(v)),
                Poll::Pending => Poll::Pending,
            };
        }

        Poll::Pending
    }
}

pub fn timeout<F>(delay: Duration, future: F) -> Timeout<F>
where
    F: Future,
{
    Timeout::new(delay, future)
}

pub fn timeout_at<F>(deadline: Instant, future: F) -> Timeout<F>
where
    F: Future,
{
    Timeout::at(deadline, future)
}

pub type SleepUntil = Sleep;
pub type TimeoutAt<F> = Timeout<F>;
