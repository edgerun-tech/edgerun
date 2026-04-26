//! Timer abstraction for async delays



extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

pub static TIMER_TICKS: AtomicU64 = AtomicU64::new(0);

#[inline]
pub fn now() -> u64 {
    TIMER_TICKS.load(Ordering::Relaxed)
}

#[inline]
pub fn set_now(ticks: u64) {
    TIMER_TICKS.store(ticks, Ordering::Relaxed);
}

#[inline]
pub fn elapsed_since(start: u64) -> u64 {
    let now = now();
    now.saturating_sub(start)
}

pub fn us_to_ticks(us: u64) -> u64 {
    us * 10
}

pub fn ms_to_ticks(ms: u64) -> u64 {
    ms * 10_000
}

pub fn ticks_to_us(ticks: u64) -> u64 {
    ticks / 10
}

pub fn ticks_to_ms(ticks: u64) -> u64 {
    ticks / 10_000
}

pub fn spin_until(deadline: u64) {
    while now() < deadline {
        core::hint::spin_loop();
    }
}

pub fn sleep_us(us: u64) {
    let deadline = now() + us_to_ticks(us);
    spin_until(deadline);
}

pub fn sleep_ms(ms: u64) {
    let deadline = now() + ms_to_ticks(ms);
    spin_until(deadline);
}

pub struct TimerWheel {
    wheel: Vec<TimerEntry>,
}

struct TimerEntry {
    deadline: u64,
    callback: fn(),
    periodic: bool,
}

impl TimerWheel {
    pub fn new() -> Self {
        Self { wheel: Vec::new() }
    }

    pub fn schedule(&mut self, deadline: u64, callback: fn(), periodic: bool) -> usize {
        let entry = TimerEntry { deadline, callback, periodic };
        self.wheel.push(entry);
        self.wheel.len() - 1
    }

    pub fn cancel(&mut self, id: usize) {
        if id < self.wheel.len() {
            self.wheel.remove(id);
        }
    }

    pub fn poll(&mut self) {
        let now = now();
        self.wheel.retain(|entry| {
            if entry.deadline <= now {
                (entry.callback)();
                return entry.periodic;
            }
            true
        });
    }

    pub fn pending(&self) -> usize {
        self.wheel.len()
    }
}

impl Default for TimerWheel {
    fn default() -> Self {
        Self::new()
    }
}