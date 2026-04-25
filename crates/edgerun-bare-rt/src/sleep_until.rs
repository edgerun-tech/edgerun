//! Sleep utilities.

#![no_std]

extern crate alloc;

use alloc::sync::Arc;
use core::cell::UnsafeCell;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};

use super::time::Instant;

pub struct SleepInner {
    deadline_tsc: UnsafeCell<u64>,
    waker: UnsafeCell<Option<Waker>>,
}

pub struct Sleep {
    inner: Arc<SleepInner>,
}

impl Sleep {
    pub fn new(deadline: Instant) -> Self {
        Self {
            inner: Arc::new(SleepInner {
                deadline_tsc: UnsafeCell::new(deadline.deadline_tsc()),
                waker: UnsafeCell::new(None),
            }),
        }
    }

    pub fn deadline(&self) -> Instant {
        Instant { tsc: unsafe { *self.inner.deadline_tsc.get() } }
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let deadline_tsc = unsafe { *this.inner.deadline_tsc.get() };
        let now = Instant::now().deadline_tsc();
        
        if now >= deadline_tsc {
            return Poll::Ready(());
        }
        
        unsafe { *this.inner.waker.get() = Some(cx.waker().clone()) };
        Poll::Pending
    }
}

pub async fn sleep_until(deadline: Instant) {
    Sleep::new(deadline).await
}

pub fn timeout_at<T>(deadline: Instant, f: T) -> TimeoutAt<T>
where
    T: Future,
{
    TimeoutAt { future: f, deadline }
}

pub struct TimeoutAt<T> {
    future: T,
    deadline: Instant,
}

impl<T: Future + Unpin> Future for TimeoutAt<T> {
    type Output = Result<T::Output, Elapsed>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let now = Instant::now().deadline_tsc();
        
        if now >= this.deadline.deadline_tsc() {
            return Poll::Ready(Err(Elapsed));
        }
        
        match Pin::new(&mut this.future).poll(cx) {
            Poll::Ready(v) => Poll::Ready(Ok(v)),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[derive(Debug)]
pub struct Elapsed;