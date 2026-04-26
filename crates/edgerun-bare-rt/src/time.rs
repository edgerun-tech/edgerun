#[derive(Clone, Copy, Debug, Default)]
pub struct Duration {
    secs: u64,
    nanos: u32,
}

impl Duration {
    pub fn from_secs(s: u64) -> Self { Self { secs: s, nanos: 0 } }
    pub fn from_millis(ms: u64) -> Self { Self { secs: ms / 1000, nanos: (ms % 1000) as u32 * 1_000_000 } }
    pub fn as_secs(&self) -> u64 { self.secs }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Instant(Duration);

impl Instant {
    pub fn now() -> Self {
        Self(Duration { secs: edgerun_platform::timer::timer_ticks() / 10_000_000, nanos: 0 })
    }
    pub fn elapsed(&self) -> Duration { Duration::default() }
}