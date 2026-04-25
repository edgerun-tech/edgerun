//! Rate limiter - token bucket algorithm.


extern crate alloc;

use alloc::sync::Arc;
use core::pin::Pin;
use core::sync::atomic::{AtomicU64, Ordering};
use core::task::Poll;

pub struct RateLimiter {
    inner: Arc<Inner>,
}

struct Inner {
    capacity: AtomicU64,
    available: AtomicU64,
    refill_interval: AtomicU64,
    last_refill: AtomicU64,
}

impl RateLimiter {
    pub fn new(capacity: u64, refill_interval_ms: u64) -> Self {
        Self {
            inner: Arc::new(Inner {
                capacity: AtomicU64::new(capacity),
                available: AtomicU64::new(capacity),
                refill_interval: AtomicU64::new(refill_interval_ms),
                last_refill: AtomicU64::new(0),
            }),
        }
    }

    pub fn try_acquire(&self) -> bool {
        self.refill();
        let available = self.inner.available.load(Ordering::Acquire);
        if available > 0 {
            self.inner.available.store(available - 1, Ordering::Release);
            true
        } else {
            false
        }
    }

    pub fn acquire(&self) -> AcquireFuture {
        AcquireFuture { limiter: self.inner.clone() }
    }

    fn refill(&self) {
        let now = current_time_ms();
        let last = self.inner.last_refill.load(Ordering::Acquire);
        let interval = self.inner.refill_interval.load(Ordering::Acquire);
        
        if now.saturating_sub(last) >= interval {
            let capacity = self.inner.capacity.load(Ordering::Acquire);
            self.inner.available.store(capacity, Ordering::Release);
            self.inner.last_refill.store(now, Ordering::Release);
        }
    }

    pub fn available(&self) -> u64 {
        self.refill();
        self.inner.available.load(Ordering::Acquire)
    }
}

pub struct AcquireFuture {
    limiter: Arc<Inner>,
}

impl core::future::Future for AcquireFuture {
    type Output = bool;

    fn poll(self: Pin<&mut Self>, _cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.limiter.available.load(Ordering::Acquire) > 0)
    }
}

fn current_time_ms() -> u64 {
    (unsafe { core::arch::x86_64::_rdtsc() }) / 10_000_000
}

pub struct TokenBucket {
    tokens: AtomicU64,
    max_tokens: u64,
    refill_rate: u64,
}

impl TokenBucket {
    pub fn new(max_tokens: u64, refill_rate: u64) -> Self {
        Self {
            tokens: AtomicU64::new(max_tokens),
            max_tokens,
            refill_rate,
        }
    }

    pub fn try_take(&self, ones: u64) -> bool {
        let current = self.tokens.load(Ordering::Acquire);
        if current >= ones {
            self.tokens.store(current - ones, Ordering::Release);
            true
        } else {
            false
        }
    }

    pub fn refill(&self) {
        let new_tokens = self.tokens.load(Ordering::Acquire).saturating_add(self.refill_rate);
        let capped = new_tokens.min(self.max_tokens);
        self.tokens.store(capped, Ordering::Release);
    }
}