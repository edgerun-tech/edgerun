//! Minimal time abstractions for bare-metal.
//!
//! Provides `Instant` and `Duration` without std.

use core::ops::{Add, Sub};

/// Duration type for bare-metal.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Duration {
    secs: u64,
    nanos: u32,
}

impl Duration {
    pub const ZERO: Self = Self { secs: 0, nanos: 0 };
    pub const MAX: Self = Self { secs: u64::MAX, nanos: 999_999_999 };

    #[inline]
    pub const fn from_secs(secs: u64) -> Self {
        Self { secs, nanos: 0 }
    }

    #[inline]
    pub const fn from_millis(ms: u64) -> Self {
        Self {
            secs: ms / 1000,
            nanos: ((ms % 1000) as u32) * 1_000_000,
        }
    }

    #[inline]
    pub const fn from_micros(us: u64) -> Self {
        Self {
            secs: us / 1_000_000,
            nanos: ((us % 1_000_000) as u32) * 1000,
        }
    }

    #[inline]
    pub const fn as_secs(&self) -> u64 {
        self.secs
    }

    #[inline]
    pub const fn as_millis(&self) -> u64 {
        self.secs * 1000 + (self.nanos / 1_000_000) as u64
    }

    #[inline]
    pub const fn as_micros(&self) -> u64 {
        self.secs * 1_000_000 + (self.nanos / 1000) as u64
    }
}

impl Add for Duration {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        let mut secs = self.secs.saturating_add(rhs.secs);
        let mut nanos = self.nanos + rhs.nanos;
        if nanos >= 1_000_000_000 {
            nanos -= 1_000_000_000;
            secs = secs.saturating_add(1);
        }
        Self { secs, nanos }
    }
}

impl Sub for Duration {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        let secs = self.secs.saturating_sub(rhs.secs);
        let nanos = if self.nanos >= rhs.nanos {
            self.nanos - rhs.nanos
        } else {
            1_000_000_000 - (rhs.nanos - self.nanos)
        };
        Self { secs, nanos }
    }
}

/// Instant type - monotonically increasing time from some epoch.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Instant {
    ticks: u64,
}

impl Instant {
    #[inline]
    pub const fn now() -> Self {
        // Will be provided by hardware timer
        Self { ticks: 0 }
    }

    #[inline]
    pub const fn far_future() -> Self {
        Self { ticks: u64::MAX }
    }

    #[inline]
    pub fn checked_duration_since(&self, earlier: Instant) -> Option<Duration> {
        if self.ticks >= earlier.ticks {
            Some(Duration {
                secs: (self.ticks - earlier.ticks) / 1_000_000,
                nanos: ((self.ticks - earlier.ticks) % 1_000_000) as u32 * 1000,
            })
        } else {
            None
        }
    }

    #[inline]
    pub fn duration_since(&self, earlier: Instant) -> Duration {
        self.checked_duration_since(earlier).unwrap_or(Duration::ZERO)
    }
}

/// Hardware clock interface.
pub trait Clock {
    fn now(&self) -> Instant;
    fn frequency(&self) -> u64;
}

/// Clock instance - will be set up by runtime.
pub struct HardwareClock;

impl Clock for HardwareClock {
    fn now(&self) -> Instant {
        // Read from HPET or other hardware timer
        // This is architecture-specific - placeholder
        Instant::now()
    }

    fn frequency(&self) -> u64 {
        // Typically 10MHz for HPET, or 100MHz for TSC
        10_000_000
    }
}