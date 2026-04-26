//! Interval utilities

extern crate edgerun_platform;

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use core::time::Duration;

pub struct Interval {
    duration: u64,
    next: u64,
}

impl Interval {
    fn new(duration: Duration) -> Self {
        let duration_us = edgerun_platform::timer::us_to_ticks(duration.as_micros() as u64);
        let next = edgerun_platform::timer::timer_ticks() + duration_us;
        Self { duration: duration_us, next }
    }

    fn poll_next(&mut self) -> bool {
        let now = edgerun_platform::timer::timer_ticks();
        if now >= self.next {
            self.next = now + self.duration;
            true
        } else {
            false
        }
    }
}

impl Future for Interval {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.poll_next() {
            cx.waker().wake_by_ref();
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

pub fn interval(period: Duration) -> Interval {
    Interval::new(period)
}

pub fn interval_at(_start: Duration, period: Duration) -> Interval {
    Interval::new(period)
}