//! Sleep utilities.

#![no_std]

extern crate alloc;

use alloc::sync::Arc;
use core::cell::UnsafeCell;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::task::{Context, Poll, Waker};

const Acquire: Ordering = Ordering::Acquire;
const Release: Ordering = Ordering::Release;

// ===========================================================================
// Sleep
// ===========================================================================

struct SleepInner {
    deadline: UnsafeCell<u64>,
    waker: UnsafeCell<Option<Waker>>,
}

// Simple timer wheel for bare metal
pub struct Sleep {
    inner: Arc<SleepInner>,
}

impl Sleep {
    pub fn new(deadline: u64) -> Self {
        Self {
            inner: Arc::new(SleepInner {
                deadline: UnsafeCell::new(deadline),
                waker: UnsafeCell::new(None),
            }),
        }
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let deadline = unsafe { *this.inner.deadline.get() };
        
        // Use TSC or simple counter for time
        let now = unsafe { core::arch::x86_64::_rdtsc() };
        
        if now >= deadline {
            return Poll::Ready(());
        }
        
        unsafe { *this.inner.waker.get() = Some(cx.waker().clone()) };
        Poll::Pending
    }
}

// ===========================================================================
// Functions
// ===========================================================================

pub async fn sleep_until(deadline: u64) {
    Sleep::new(deadline).await
}

pub fn timeout_at<T>(deadline: u64, f: T) -> TimeoutAt<T>
where
    T: Future,
{
    TimeoutAt { future: f, deadline }
}

pub struct TimeoutAt<T> {
    future: T,
    deadline: u64,
}

impl<T: Future + Unpin> Future for TimeoutAt<T> {
    type Output = Result<T::Output, Elapsed>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let now = unsafe { core::arch::x86_64::_rdtsc() };
        
        if now >= this.deadline {
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

// Stub for Interval
pub struct Interval {
    period: u64,
}

impl Interval {
    pub fn new(period: u64) -> Self {
        Self { period }
    }
}

impl Future for Interval {
    type Output = ();
    
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}