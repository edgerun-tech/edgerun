//! Sleep, timeout, interval, and signal (ctrl_c) primitives.
//!
//! ## Fixes applied:
//! - `ctrl_c()` no longer busy-spins via `wake_by_ref` — uses a 100ms timer
//! - `Timeout` properly registers timer on first poll

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

use crate::reactor::Instant;
use crate::runtime::current_rt;

// ===========================================================================
// Sleep
// ===========================================================================

pub fn sleep(d: Duration) -> Sleep {
    Sleep {
        deadline: Instant::now() + d,
        registered: false,
    }
}

pub struct Sleep {
    deadline: Instant,
    registered: bool,
}

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if Instant::now() >= self.deadline {
            Poll::Ready(())
        } else {
            let this = unsafe { self.get_unchecked_mut() };
            if !this.registered {
                current_rt()
                    .reactor
                    .register_timer(this.deadline, cx.waker().clone());
                this.registered = true;
            }
            Poll::Pending
        }
    }
}

// ===========================================================================
// Timeout
// ===========================================================================

pub fn timeout<F>(d: Duration, f: F) -> Timeout<F> {
    Timeout {
        inner: Some(f),
        deadline: Instant::now() + d,
        registered: false,
    }
}

pub struct Timeout<F> {
    inner: Option<F>,
    deadline: Instant,
    registered: bool,
}

impl<F: Future> Future for Timeout<F> {
    type Output = Result<F::Output, Elapsed>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let deadline = this.deadline;

        if Instant::now() >= deadline {
            return Poll::Ready(Err(Elapsed));
        }

        let fut = match this.inner.as_mut() {
            Some(f) => f,
            None => return Poll::Ready(Err(Elapsed)),
        };

        match unsafe { Pin::new_unchecked(fut) }.poll(cx) {
            Poll::Ready(v) => Poll::Ready(Ok(v)),
            Poll::Pending => {
                // Only register timer once — avoid accumulating duplicate
                // wakers on the timer heap when the inner future is polled
                // multiple times (e.g. by multiple wakers).
                if !this.registered {
                    current_rt()
                        .reactor
                        .register_timer(deadline, cx.waker().clone());
                    this.registered = true;
                }
                Poll::Pending
            }
        }
    }
}

#[derive(Debug)]
pub struct Elapsed;

impl std::fmt::Display for Elapsed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("deadline elapsed")
    }
}

impl std::error::Error for Elapsed {}

// ===========================================================================
// Interval
// ===========================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissedTickBehavior {
    Skip,
}

pub fn interval(d: Duration) -> Interval {
    Interval {
        d,
        next: Instant::now() + d,
        _mb: MissedTickBehavior::Skip,
    }
}

/// Create an interval that starts ticking at the given deadline.
pub fn interval_at(start: Instant, d: Duration) -> Interval {
    Interval {
        d,
        next: start,
        _mb: MissedTickBehavior::Skip,
    }
}

pub struct Interval {
    d: Duration,
    next: Instant,
    _mb: MissedTickBehavior,
}

impl Interval {
    pub fn set_missed_tick_behavior(&mut self, b: MissedTickBehavior) {
        self._mb = b;
    }
    pub fn tick(&mut self) -> IntervalTick<'_> {
        IntervalTick { interval: self, registered: false }
    }
}

pub struct IntervalTick<'a> {
    interval: &'a mut Interval,
    registered: bool,
}

impl Future for IntervalTick<'_> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        if Instant::now() >= this.interval.next {
            this.interval.next += this.interval.d;
            Poll::Ready(())
        } else {
            if !this.registered {
                current_rt()
                    .reactor
                    .register_timer(this.interval.next, cx.waker().clone());
                this.registered = true;
            }
            Poll::Pending
        }
    }
}

// ===========================================================================
// Signal — ctrl_c
// ===========================================================================

static GOT_SIGINT: AtomicBool = AtomicBool::new(false);

/// Future that resolves when SIGINT (Ctrl+C) is received.
///
/// Uses a 100ms timer to avoid busy-spinning the CPU.
pub fn ctrl_c() -> CtrlC {
    extern "C" fn handler(_: libc::c_int) {
        GOT_SIGINT.store(true, Ordering::Release);
    }
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        unsafe {
            libc::signal(libc::SIGINT, handler as *const () as libc::sighandler_t);
        }
    });
    CtrlC { deadline: None }
}

pub struct CtrlC {
    /// Deadline of the currently registered timer. Only register a new
    /// timer when this deadline has passed, preventing duplicate timer
    /// entries on the heap.
    deadline: Option<Instant>,
}

impl Future for CtrlC {
    type Output = std::io::Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if GOT_SIGINT.load(Ordering::Acquire) {
            return Poll::Ready(Ok(()));
        }
        let this = unsafe { self.get_unchecked_mut() };
        let now = Instant::now();

        match this.deadline {
            Some(d) if now >= d => {
                let new_deadline = now + Duration::from_millis(100);
                current_rt().reactor.register_timer(new_deadline, cx.waker().clone());
                this.deadline = Some(new_deadline);
            }
            None => {
                let d = now + Duration::from_millis(100);
                current_rt().reactor.register_timer(d, cx.waker().clone());
                this.deadline = Some(d);
            }
            Some(_) => {}
        }
        Poll::Pending
    }
}
