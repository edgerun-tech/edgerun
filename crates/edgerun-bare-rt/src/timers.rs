//! Timer-based async primitives

extern crate edgerun_platform;

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use core::time::Duration;
use core::sync::atomic::{AtomicBool, Ordering};

pub struct Sleep {
    deadline: u64,
}

impl Unpin for Sleep {}

impl Sleep {
    pub fn new(delay: Duration) -> Self {
        let deadline = edgerun_platform::timer::timer_ticks() 
            + edgerun_platform::timer::us_to_ticks(delay.as_micros() as u64);
        Self { deadline }
    }

    pub fn is_ready(&self) -> bool {
        edgerun_platform::timer::timer_ticks() >= self.deadline
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.is_ready() {
            Poll::Ready(())
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

pub async fn sleep(delay: Duration) {
    Sleep::new(delay).await;
}

#[derive(Debug)]
pub struct Elapsed;

impl core::fmt::Display for Elapsed {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "timeout elapsed")
    }
}

pub struct Timeout<F> {
    inner: Option<F>,
    deadline: u64,
    elapsed: AtomicBool,
}

impl<F> Unpin for Timeout<F> {}

impl<F> Timeout<F> {
    pub fn new(delay: Duration, future: F) -> Self {
        let deadline = edgerun_platform::timer::timer_ticks() 
            + edgerun_platform::timer::us_to_ticks(delay.as_micros() as u64);
        Self { inner: Some(future), deadline, elapsed: AtomicBool::new(false) }
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
    F: Future + Unpin,
{
    type Output = Result<F::Output, Elapsed>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        
        if edgerun_platform::timer::timer_ticks() >= this.deadline {
            this.elapsed.store(true, Ordering::Release);
            return Poll::Ready(Err(Elapsed));
        }
        
        if let Some(ref mut inner) = this.inner {
            return match Pin::new(inner).poll(cx) {
                Poll::Ready(v) => Poll::Ready(Ok(v)),
                Poll::Pending => Poll::Pending,
            };
        }
        
        Poll::Pending
    }
}

pub fn timeout<F>(delay: Duration, future: F) -> Timeout<F>
where
    F: Future + Unpin,
{
    Timeout::new(delay, future)
}