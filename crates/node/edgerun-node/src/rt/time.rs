pub use core::time::Duration;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub struct Instant {
    ticks: u64,
}

impl Instant {
    pub fn now() -> Self {
        Self {
            ticks: edgerun_platform::timer::timer_ticks(),
        }
    }

    pub const fn from_ticks(ticks: u64) -> Self {
        Self { ticks }
    }

    pub const fn deadline_tsc(&self) -> u64 {
        self.ticks
    }

    pub fn elapsed(&self) -> Duration {
        let now = Self::now();
        if now.ticks <= self.ticks {
            Duration::ZERO
        } else {
            Duration::from_micros(edgerun_platform::timer::ticks_to_us(now.ticks - self.ticks))
        }
    }
}

impl core::ops::Add<Duration> for Instant {
    type Output = Instant;

    fn add(self, rhs: Duration) -> Self::Output {
        Instant {
            ticks: self
                .ticks
                .saturating_add(edgerun_platform::timer::us_to_ticks(rhs.as_micros() as u64)),
        }
    }
}

impl core::ops::Sub<Duration> for Instant {
    type Output = Instant;

    fn sub(self, rhs: Duration) -> Self::Output {
        Instant {
            ticks: self
                .ticks
                .saturating_sub(edgerun_platform::timer::us_to_ticks(rhs.as_micros() as u64)),
        }
    }
}

impl core::ops::Sub<Instant> for Instant {
    type Output = Duration;

    fn sub(self, rhs: Instant) -> Self::Output {
        if self.ticks <= rhs.ticks {
            Duration::ZERO
        } else {
            Duration::from_micros(edgerun_platform::timer::ticks_to_us(self.ticks - rhs.ticks))
        }
    }
}
