//! Async barrier — coordinates a fixed number of tasks.
//!
//! When `wait()` is called on the barrier, the task pends until
//! `n` tasks have called `wait()`. Then all are released simultaneously.

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use crate::sync::Mutex;
use std::task::{Context, Poll, Waker};

/// An async barrier for coordinating `n` tasks.
pub struct Barrier {
    inner: std::sync::Arc<Mutex<BarrierInner>>,
    n: usize,
    /// Generation counter — incremented each time the barrier is tripped.
    generation: AtomicUsize,
}

struct BarrierInner {
    count: usize,
    waiters: VecDeque<Waker>,
}

impl Barrier {
    /// Creates a new barrier that requires `n` tasks to trip it.
    pub fn new(n: usize) -> Self {
        assert!(n > 0, "barrier count must be > 0");
        Self {
            inner: std::sync::Arc::new(Mutex::new(BarrierInner {
                count: 0,
                waiters: VecDeque::new(),
            })),
            n,
            generation: AtomicUsize::new(0),
        }
    }

    /// Waits at the barrier. If this is the `n`-th call, all waiting
    /// tasks are released. Otherwise, the task pends.
    pub fn wait(&self) -> BarrierWait<'_> {
        BarrierWait { barrier: self, generation: self.generation.load(Ordering::Acquire) }
    }

    /// Returns the number of tasks required to trip the barrier.
    pub fn n(&self) -> usize { self.n }
}

/// Future returned by `Barrier::wait()`.
pub struct BarrierWait<'a> {
    barrier: &'a Barrier,
    generation: usize,
}

impl Future for BarrierWait<'_> {
    type Output = BarrierWaitResult;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let mut inner = this.barrier.inner.lock();

        let current_gen = this.barrier.generation.load(Ordering::Acquire);
        if current_gen > this.generation {
            // The barrier was tripped after we registered.
            return Poll::Ready(BarrierWaitResult { is_leader: false });
        }

        inner.count += 1;
        if inner.count >= this.barrier.n {
            // Barrier tripped — release all waiters.
            inner.count = 0;
            this.barrier.generation.fetch_add(1, Ordering::Release);
            this.barrier.generation.fetch_add(0, Ordering::Release); // memory barrier
            let waiters = std::mem::take(&mut inner.waiters);
            drop(inner);
            for waker in waiters {
                waker.wake();
            }
            Poll::Ready(BarrierWaitResult { is_leader: true })
        } else {
            if inner.waiters.is_empty() || !inner.waiters.iter().any(|w| w.will_wake(cx.waker())) {
                inner.waiters.push_back(cx.waker().clone());
            }
            Poll::Pending
        }
    }
}

/// The result of waiting at a barrier.
pub struct BarrierWaitResult {
    /// `true` for exactly one task — the one that tripped the barrier.
    pub is_leader: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    static NOOP_WAKER: std::sync::LazyLock<Waker> = std::sync::LazyLock::new(|| {
        static VTABLE: std::task::RawWakerVTable =
            std::task::RawWakerVTable::new(clone_noop, wake_noop, wake_noop, drop_noop);
        const fn clone_noop(_: *const ()) -> std::task::RawWaker {
            std::task::RawWaker::new(std::ptr::null(), &VTABLE)
        }
        const fn wake_noop(_: *const ()) {}
        const fn drop_noop(_: *const ()) {}
        unsafe { Waker::from_raw(std::task::RawWaker::new(std::ptr::null(), &VTABLE)) }
    });

    fn cx() -> Context<'static> {
        Context::from_waker(&*NOOP_WAKER)
    }

    #[test]
    fn barrier_single_task() {
        let b = Barrier::new(1);
        let mut fut = b.wait();
        let result = Pin::new(&mut fut).poll(&mut cx());
        assert!(matches!(result, Poll::Ready(r) if r.is_leader));
    }

    #[test]
    fn barrier_two_tasks() {
        let b = Barrier::new(2);
        let mut fut1 = b.wait();
        let mut fut2 = b.wait();

        let r1 = Pin::new(&mut fut1).poll(&mut cx());
        assert!(matches!(r1, Poll::Pending));

        let r2 = Pin::new(&mut fut2).poll(&mut cx());
        assert!(matches!(r2, Poll::Ready(r) if r.is_leader));

        // fut1 should now be ready (woken by barrier trip)
        let r1_again = Pin::new(&mut fut1).poll(&mut cx());
        assert!(matches!(r1_again, Poll::Ready(r) if !r.is_leader));
    }

    #[test]
    fn barrier_reusable() {
        let b = Barrier::new(1);
        let mut fut1 = b.wait();
        assert!(matches!(Pin::new(&mut fut1).poll(&mut cx()), Poll::Ready(_)));
        // Second wait should also succeed
        let mut fut2 = b.wait();
        assert!(matches!(Pin::new(&mut fut2).poll(&mut cx()), Poll::Ready(_)));
    }
}
