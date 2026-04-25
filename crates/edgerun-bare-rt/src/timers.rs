//! Timers and intervals.


extern crate alloc;

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use super::time::{Duration, Instant};
use super::sleep_until::Elapsed;

pub fn sleep(d: Duration) -> Sleep {
    let deadline = Instant::now() + d;
    Sleep::new(deadline)
}

pub fn interval(period: Duration) -> Interval {
    Interval::new(Instant::now() + period, period)
}

pub fn interval_at(start: Instant, period: Duration) -> Interval {
    Interval::new(start + period, period)
}

pub fn timeout<F>(d: Duration, f: F) -> Timeout<F> {
    let deadline = Instant::now() + d;
    Timeout { future: f, deadline }
}

pub fn timeout_at<F>(deadline: Instant, f: F) -> Timeout<F> {
    Timeout { future: f, deadline }
}

pub struct Interval {
    next: Instant,
    period: Duration,
}

impl Interval {
    fn new(next: Instant, period: Duration) -> Self {
        Self { next, period }
    }

    pub fn tick(&mut self) -> IntervalTick<'_> {
        IntervalTick { interval: self }
    }

    pub fn period(&self) -> Duration {
        self.period
    }

    pub fn reset(&mut self) {
        self.next = Instant::now() + self.period;
    }

    fn check_and_advance(&mut self) -> bool {
        let now = Instant::now();
        if now >= self.next {
            self.next = self.next + self.period;
            if self.next <= now {
                self.next = now + self.period;
            }
            true
        } else {
            false
        }
    }
}

pub struct IntervalTick<'a> {
    interval: &'a mut Interval,
}

impl Future for IntervalTick<'_> {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.interval.check_and_advance() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

pub struct Timeout<T> {
    future: T,
    deadline: Instant,
}

impl<T: Future + Unpin> Future for Timeout<T> {
    type Output = Result<T::Output, Elapsed>;
    
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        
        if Instant::now() >= this.deadline {
            return Poll::Ready(Err(Elapsed));
        }
        
        match Pin::new(&mut this.future).poll(cx) {
            Poll::Ready(v) => Poll::Ready(Ok(v)),
            Poll::Pending => Poll::Pending,
        }
    }
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

pub use super::sleep_until::Sleep;

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