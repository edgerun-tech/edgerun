//! Timers and intervals.

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
// Timers
// ===========================================================================

pub fn sleep(duration: u64) -> impl Future<Output = ()> {
    let now = unsafe { core::arch::x86_64::_rdtsc() };
    let deadline = now + duration * 10_000_000; // rough conversion
    super::Sleep::new(deadline)
}

pub fn interval(period: u64) -> Interval {
    Interval::new(period)
}

pub fn interval_at(_period: u64) -> Interval {
    Interval::new(_period)
}

pub fn timeout<T>(duration: u64, f: T) -> Timeout<T> {
    Timeout { future: f }
}

pub fn timeout_at<T>(_deadline: u64, f: T) -> Timeout<T> {
    Timeout { future: f }
}

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

pub struct Timeout<T> {
    future: T,
}

impl<T: Future + Unpin> Future for Timeout<T> {
    type Output = Result<T::Output, Elapsed>;
    
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        match Pin::new(&mut this.future).poll(cx) {
            Poll::Ready(v) => Poll::Ready(Ok(v)),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[derive(Debug)]
pub struct Elapsed;

// Control-C stub
pub struct CtrlC;

impl CtrlC {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CtrlC {
    fn default() -> Self {
        Self::new()
    }
}

impl Future for CtrlC {
    type Output = ();
    
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub fn ctrl_c() -> CtrlC {
    CtrlC::new()
}

pub struct MissedTickBehavior;

impl MissedTickBehavior {
    pub fn skip() -> Self {
        Self
    }
    
    pub fn backlog() -> Self {
        Self
    }
}