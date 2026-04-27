//! Timer/Counter abstraction

#![allow(unsafe_op_in_unsafe_fn)]

pub struct MonoTime(u64);

impl MonoTime {
    pub fn now() -> Self {
        Self(timer_ticks())
    }
    pub fn elapsed(self) -> u64 {
        timer_ticks() - self.0
    }
    pub fn ticks(&self) -> u64 {
        self.0
    }
}

pub trait Timer {
    fn now() -> u64;
    fn set_deadline(ticks: u64);
    fn wait_for_irq();
}

pub fn timer_freq() -> u64 {
    10_000_000
}

#[inline]
pub fn timer_ticks() -> u64 {
    #[cfg(target_arch = "x86_64")]
    {
        let lo: u32;
        let hi: u32;
        unsafe {
            core::arch::asm!("rdtsc", out("rax") lo, out("rdx") hi);
        }
        ((hi as u64) << 32) | (lo as u64)
    }

    #[cfg(target_arch = "aarch64")]
    {
        let ticks: u64;
        unsafe {
            core::arch::asm!("mrs x0, CNTVCT_EL0", out("x0") ticks);
        }
        ticks
    }

    #[cfg(target_arch = "riscv64")]
    {
        let ticks: u64;
        unsafe {
            core::arch::asm!("csrr x0, time", out("x0") ticks);
        }
        ticks
    }

    #[cfg(target_arch = "xtensa")]
    {
        let ticks: u32;
        unsafe {
            core::arch::asm!("rsr CCOUNT", out("x0") ticks);
        }
        ticks as u64
    }
}

pub fn ticks_to_us(ticks: u64) -> u64 {
    ticks / (timer_freq() / 1_000_000)
}
pub fn us_to_ticks(us: u64) -> u64 {
    us * (timer_freq() / 1_000_000)
}

pub fn sleep_us(us: u64) {
    let start = timer_ticks();
    let target = start + us_to_ticks(us);
    while timer_ticks() < target {
        core::hint::spin_loop();
    }
}

pub fn spin_us(us: u64) {
    let start = timer_ticks();
    let ticks_per_us = timer_freq() / 1_000_000;
    while timer_ticks() - start < us * ticks_per_us {
        core::hint::spin_loop();
    }
}
