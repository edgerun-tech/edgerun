//! Token-bucket rate limiter.
//!
//! Controls the rate at which operations proceed using the token bucket
//! algorithm. Tokens are added at a fixed rate up to a maximum capacity.
//! Each operation consumes tokens; if insufficient, it waits.
//!
//! # Example
//! ```ignore
//! use edgerun_rt::rate_limiter::RateLimiter;
//!
//! let limiter = RateLimiter::new(10.0, 20); // 10/sec, burst 20
//! limiter.acquire(1).await;
//! ```

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// A token-bucket rate limiter.
///
/// Uses atomic operations for the hot path (try_acquire) and
/// computes wait times for the async acquire path.
///
/// The refill + consume happens under a single mutex, so there is no
/// race where refill overwrites a concurrent consume.
pub struct RateLimiter {
    /// Max tokens (burst size), stored as f64 bits.
    capacity: u64,
    /// Rate (tokens/sec), stored as f64 bits.
    rate: u64,
    /// Current tokens, stored as f64 bits.
    tokens: AtomicU64,
    /// Last refill time — held under mutex during refill+consume to
    /// prevent the refill/consume race.
    last_refill: crate::sync::Mutex<Instant>,
}

impl RateLimiter {
    /// Creates a new rate limiter, initially full.
    ///
    /// - `rate`: tokens added per second
    /// - `capacity`: maximum tokens (burst size)
    pub fn new(rate: f64, capacity: u64) -> Self {
        Self {
            capacity,
            rate: rate.to_bits(),
            tokens: AtomicU64::new((capacity as f64).to_bits()),
            last_refill: crate::sync::Mutex::new(Instant::now()),
        }
    }

    fn rate_f64(&self) -> f64 {
        f64::from_bits(self.rate)
    }

    fn capacity_f64(&self) -> f64 {
        self.capacity as f64
    }

    /// Refills tokens based on elapsed time and returns the new count.
    ///
    /// Must be called while holding `last_refill` mutex to prevent
    /// races with concurrent consumers.
    fn refill_locked(&self) -> f64 {
        let rate = self.rate_f64();
        let cap = self.capacity_f64();
        let mut last = self.last_refill.lock();
        let now = Instant::now();
        let elapsed = now.duration_since(*last).as_secs_f64();
        *last = now;

        let current = f64::from_bits(self.tokens.load(Ordering::Acquire));
        let new_tokens = (current + elapsed * rate).min(cap);
        self.tokens.store(new_tokens.to_bits(), Ordering::Release);
        new_tokens
    }

    /// Current available tokens (after refill).
    pub fn available(&self) -> f64 {
        let _lock = self.last_refill.lock();
        f64::from_bits(self.tokens.load(Ordering::Acquire))
    }

    /// Try to acquire `n` tokens immediately. Returns `true` on success.
    ///
    /// Refill and consume are integrated into a single CAS loop, so there
    /// is no race where refill overwrites a concurrent consume.
    pub fn try_acquire(&self, n: u64) -> bool {
        let n_f = n as f64;
        let rate = self.rate_f64();
        let cap = self.capacity_f64();

        // CAS loop that atomically refills AND consumes.
        let mut current = f64::from_bits(self.tokens.load(Ordering::Acquire));
        loop {
            // Compute refill under the mutex to get a consistent timestamp.
            let refill_guard = self.last_refill.lock();
            let now = Instant::now();
            let elapsed = now.duration_since(*refill_guard).as_secs_f64();
            // We don't update *last here — we'll do it after the CAS succeeds.
            drop(refill_guard);

            let refilled = (current + elapsed * rate).min(cap);
            if refilled < n_f {
                return false;
            }
            let new_tokens = refilled - n_f;

            // Try to atomically transition from current -> new_tokens.
            // If this succeeds, we also update last_refill under the mutex.
            match self.tokens.compare_exchange_weak(
                current.to_bits(),
                new_tokens.to_bits(),
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    // Update the refill timestamp now that we've consumed.
                    let mut refill_guard = self.last_refill.lock();
                    *refill_guard = now;
                    return true;
                }
                Err(actual) => current = f64::from_bits(actual),
            }
        }
    }

    /// Computes how long to wait for `n` tokens.
    ///
    /// Returns `Duration::ZERO` if tokens are already available.
    pub fn wait_time(&self, n: u64) -> Duration {
        let _lock = self.last_refill.lock();
        let current = f64::from_bits(self.tokens.load(Ordering::Acquire));
        let n_f = n as f64;
        if current >= n_f {
            return Duration::ZERO;
        }
        let deficit = n_f - current;
        let rate = self.rate_f64();
        if rate <= 0.0 {
            return Duration::MAX;
        }
        Duration::from_secs_f64(deficit / rate)
    }

    /// Acquire `n` tokens, sleeping if necessary.
    pub async fn acquire(&self, n: u64) {
        loop {
            if self.try_acquire(n) {
                return;
            }
            let wait = self.wait_time(n);
            if wait.is_zero() {
                // Tokens are available but try_acquire failed (another thread
                // grabbed them). Yield and retry.
                crate::yieldnow().await;
                continue;
            }
            crate::sleep(wait).await;
        }
    }

    /// Tokens per second.
    pub fn rate(&self) -> f64 {
        self.rate_f64()
    }

    /// Maximum burst size.
    pub fn capacity(&self) -> u64 {
        self.capacity
    }
}
