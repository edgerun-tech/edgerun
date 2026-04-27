//! Small compatibility placeholders for APIs that need bare implementations.

use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::task::{Context, Poll};

pub struct File;

impl File {
    pub fn new() -> Self {
        Self
    }
}

impl Default for File {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Dir;

impl Dir {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Dir {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RateLimiter {
    capacity: usize,
    refill_interval_ms: u64,
}

pub struct TokenBucket {
    capacity: usize,
    refill_rate: usize,
}

impl TokenBucket {
    pub fn new(capacity: usize, refill_rate: usize) -> Self {
        Self {
            capacity,
            refill_rate,
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn refill_rate(&self) -> usize {
        self.refill_rate
    }
}

impl RateLimiter {
    pub fn new(capacity: usize, refill_interval_ms: u64) -> Self {
        Self {
            capacity,
            refill_interval_ms,
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn refill_interval_ms(&self) -> u64 {
        self.refill_interval_ms
    }
}

pub struct TaskMetrics {
    name: &'static str,
}

pub struct Span {
    name: &'static str,
}

impl Span {
    pub fn new(name: &'static str) -> Self {
        Self { name }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }
}

pub struct RuntimeMetrics;

impl RuntimeMetrics {
    pub fn new() -> Self {
        Self
    }
}

pub struct TaskMap;

impl TaskMap {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TaskMap {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for RuntimeMetrics {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Latch {
    count: AtomicUsize,
}

impl Latch {
    pub fn new(count: usize) -> Self {
        Self {
            count: AtomicUsize::new(count),
        }
    }

    pub fn count_down(&self) {
        let _ = self
            .count
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| n.checked_sub(1));
    }

    pub fn wait(&self) -> WaitLatch<'_> {
        WaitLatch { latch: self }
    }
}

pub struct WaitLatch<'a> {
    latch: &'a Latch,
}

impl Future for WaitLatch<'_> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.latch.count.load(Ordering::Acquire) == 0 {
            Poll::Ready(())
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

pub struct Barrier {
    needed: usize,
    arrived: AtomicUsize,
}

impl Barrier {
    pub fn new(needed: usize) -> Self {
        Self {
            needed,
            arrived: AtomicUsize::new(0),
        }
    }

    pub fn wait(&self) -> BarrierWait<'_> {
        BarrierWait {
            barrier: self,
            counted: false,
        }
    }
}

pub struct BarrierWaitResult {
    leader: bool,
}

impl BarrierWaitResult {
    pub fn is_leader(&self) -> bool {
        self.leader
    }
}

pub struct BarrierWait<'a> {
    barrier: &'a Barrier,
    counted: bool,
}

impl Future for BarrierWait<'_> {
    type Output = BarrierWaitResult;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        if !this.counted {
            this.barrier.arrived.fetch_add(1, Ordering::AcqRel);
            this.counted = true;
        }
        if this.barrier.arrived.load(Ordering::Acquire) >= this.barrier.needed {
            Poll::Ready(BarrierWaitResult { leader: false })
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

impl TaskMetrics {
    pub fn new(name: &'static str) -> Self {
        Self { name }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }
}
