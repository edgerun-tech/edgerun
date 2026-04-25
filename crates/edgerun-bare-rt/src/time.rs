//! Time types for no_std bare-metal runtime.

#![no_std]

pub use core::time::Duration;

#[derive(Clone, Copy, Debug)]
pub struct Instant {
    pub(crate) tsc: u64,
}

impl Instant {
    pub fn now() -> Self {
        Self {
            tsc: Self::read_tsc(),
        }
    }

    pub fn zero() -> Self {
        Self { tsc: 0 }
    }

    pub fn checked_add(&self, duration: Duration) -> Option<Instant> {
        let millis = duration.as_millis() as u64;
        let tsc_per_ms: u64 = 10_000_000 / 1_000_0;
        Some(Instant {
            tsc: self.tsc.checked_add(millis.saturating_mul(tsc_per_ms))?,
        })
    }

    pub fn checked_sub(&self, other: Instant) -> Option<Duration> {
        let diff = self.tsc.checked_sub(other.tsc)?;
        let tsc_per_ms: u64 = 10_000_000 / 1_000_0;
        if tsc_per_ms == 0 {
            return Some(Duration::ZERO);
        }
        let ms = diff / tsc_per_ms;
        Some(Duration::from_millis(ms))
    }

    pub fn elapsed(&self) -> Duration {
        Self::now().checked_sub(*self).unwrap_or(Duration::ZERO)
    }

    pub fn is_elapsed(&self) -> bool {
        Self::now() >= *self
    }

    pub fn deadline_tsc(&self) -> u64 {
        self.tsc
    }

    fn read_tsc() -> u64 {
        unsafe { core::arch::x86_64::_rdtsc() }
    }
}

impl PartialOrd for Instant {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Instant {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.tsc.cmp(&other.tsc)
    }
}

impl PartialEq for Instant {
    fn eq(&self, other: &Self) -> bool {
        self.tsc == other.tsc
    }
}

impl Eq for Instant {}

impl core::ops::Add<Duration> for Instant {
    type Output = Instant;
    fn add(self, duration: Duration) -> Self::Output {
        self.checked_add(duration).expect("overflow")
    }
}

impl core::ops::Sub<Instant> for Instant {
    type Output = Duration;
    fn sub(self, other: Instant) -> Self::Output {
        self.checked_sub(other).expect("underflow")
    }
}