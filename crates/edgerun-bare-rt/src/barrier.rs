//! Async barrier - coordinates a fixed number of tasks.
//!
//! When `wait()` is called on the barrier, the task pends until
//! `n` tasks have called `wait()`. Then all are released simultaneously.

#![no_std]

extern crate alloc;

use alloc::sync::Arc;
use core::cell::UnsafeCell;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::task::{Context, Poll, Waker};

const Acquire: Ordering = Ordering::Acquire;
const AcqRel: Ordering = Ordering::AcqRel;
const Release: Ordering = Ordering::Release;

// ===========================================================================
// Barrier
// ===========================================================================

struct BarrierInner {
    count: AtomicUsize,
    n: usize,
    generation: AtomicUsize,
    waker: UnsafeCell<Option<Waker>>,
}

impl BarrierInner {
    fn new(n: usize) -> Self {
        Self {
            count: AtomicUsize::new(0),
            n,
            generation: AtomicUsize::new(0),
            waker: UnsafeCell::new(None),
        }
    }
}

/// An async barrier for coordinating `n` tasks.
pub struct Barrier {
    inner: Arc<BarrierInner>,
}

impl Barrier {
    pub fn new(n: usize) -> Self {
        Self {
            inner: Arc::new(BarrierInner::new(n)),
        }
    }

    pub fn wait(&self) -> BarrierWait<'_> {
        BarrierWait {
            barrier: self,
            generation: self.inner.generation.load(Acquire),
            registered: false,
        }
    }

    pub fn n(&self) -> usize {
        self.inner.n
    }
}

/// Future returned by `Barrier::wait()`.
pub struct BarrierWait<'a> {
    barrier: &'a Barrier,
    generation: usize,
    registered: bool,
}

pub struct BarrierWaitResult {
    pub is_leader: bool,
}

impl Future for BarrierWait<'_> {
    type Output = BarrierWaitResult;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let current_gen = this.barrier.inner.generation.load(Acquire);
        if current_gen > this.generation {
            return Poll::Ready(BarrierWaitResult { is_leader: false });
        }

        let prev = this.barrier.inner.count.fetch_add(1, AcqRel);
        if prev + 1 >= this.barrier.inner.n {
            this.barrier.inner.generation.fetch_add(1, Release);
            unsafe {
                if let Some(w) = (*this.barrier.inner.waker.get()).take() {
                    w.wake();
                }
            }
            Poll::Ready(BarrierWaitResult { is_leader: true })
        } else {
            if !this.registered {
                unsafe { *this.barrier.inner.waker.get() = Some(cx.waker().clone()) };
                this.registered = true;
            }
            Poll::Pending
        }
    }
}