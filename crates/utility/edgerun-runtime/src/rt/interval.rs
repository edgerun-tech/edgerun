//! Interval utilities

extern crate edgerun_platform;

use crate::rt::Instant;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};
use core::time::Duration;

pub struct Interval {
    duration: u64,
    next: u64,
    missed_tick_behavior: MissedTickBehavior,
    waker: Option<Waker>,
}

impl Interval {
    fn new(duration: Duration) -> Self {
        let duration_us = edgerun_platform::timer::us_to_ticks(duration.as_micros() as u64);
        let next = edgerun_platform::timer::timer_ticks() + duration_us;
        Self {
            duration: duration_us,
            next,
            missed_tick_behavior: MissedTickBehavior::Burst,
            waker: None,
        }
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

    pub fn set_missed_tick_behavior(&mut self, behavior: MissedTickBehavior) {
        self.missed_tick_behavior = behavior;
    }

    pub fn tick(&mut self) -> Tick<'_> {
        Tick { interval: self }
    }
}

pub struct Tick<'a> {
    interval: &'a mut Interval,
}

impl Future for Tick<'_> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        Pin::new(&mut *this.interval).poll(cx)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MissedTickBehavior {
    Burst,
    Skip,
    Backlog,
}

impl MissedTickBehavior {
    pub const fn burst() -> Self {
        Self::Burst
    }

    pub const fn skip() -> Self {
        Self::Skip
    }

    pub const fn backlog() -> Self {
        Self::Backlog
    }
}

impl Future for Interval {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.poll_next() {
            self.waker = None;
            Poll::Ready(())
        } else {
            if self
                .waker
                .as_ref()
                .map_or(true, |registered| !registered.will_wake(cx.waker()))
            {
                self.waker = Some(cx.waker().clone());
            }
            Poll::Pending
        }
    }
}

impl Drop for Interval {
    fn drop(&mut self) {
        self.waker = None;
    }
}

pub fn interval(period: Duration) -> Interval {
    Interval::new(period)
}

pub fn interval_at(start: Instant, period: Duration) -> Interval {
    let duration = edgerun_platform::timer::us_to_ticks(period.as_micros() as u64);
    Interval {
        duration,
        next: start.deadline_tsc(),
        missed_tick_behavior: MissedTickBehavior::Burst,
        waker: None,
    }
}
