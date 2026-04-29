//! CPU identification and per-CPU state for SMP

#[cfg(target_arch = "x86_64")]
use crate::arch::x86_64::send_ipi;
#[cfg(target_arch = "x86_64")]
use crate::timer::timer_ticks;
use core::sync::atomic::{AtomicU8, Ordering};

const MAX_CPUS: usize = 64;

pub static CPU_COUNT: AtomicU8 = AtomicU8::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CpuId(pub u8);

impl CpuId {
    pub fn new(id: u8) -> Self {
        Self(id)
    }
    pub fn index(self) -> u8 {
        self.0
    }
}

impl From<u8> for CpuId {
    fn from(id: u8) -> Self {
        Self(id)
    }
}
impl From<CpuId> for u8 {
    fn from(id: CpuId) -> Self {
        id.0
    }
}

#[derive(Clone, Copy)]
pub struct PerCpu {
    pub id: CpuId,
    pub stack_top: usize,
    pub tls_ptr: usize,
    pub run_queue_head: usize,
    pub run_queue_tail: usize,
    pub idle_count: u64,
}

pub static mut CPU_DATA: [PerCpu; MAX_CPUS] = [PerCpu {
    id: CpuId(0),
    stack_top: 0,
    tls_ptr: 0,
    run_queue_head: 0,
    run_queue_tail: 0,
    idle_count: 0,
}; MAX_CPUS];

#[inline]
pub fn this() -> &'static PerCpu {
    #[cfg(target_arch = "x86_64")]
    {
        let id: usize;
        unsafe {
            core::arch::asm!("mov rax, fs:4", out("rax") id);
        }
        unsafe { &CPU_DATA[id as usize] }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        unsafe { &CPU_DATA[0] }
    }
}

pub fn this_cpu() -> CpuId {
    this().id
}

pub fn cpu_init(id: CpuId, stack_top: usize) {
    unsafe {
        CPU_DATA[id.0 as usize].id = id;
    }
    unsafe {
        CPU_DATA[id.0 as usize].stack_top = stack_top;
    }
}

pub fn all_cpus() -> impl Iterator<Item = CpuId> {
    (0..CPU_COUNT.load(Ordering::Acquire) as u8).map(CpuId)
}

pub fn wake(_cpu: CpuId) {
    #[cfg(target_arch = "x86_64")]
    send_ipi(_cpu.0, 0xFEE0);
}

pub fn smp_init(count: u8) -> bool {
    if count <= 1 {
        return true;
    }

    #[cfg(target_arch = "x86_64")]
    {
        for cpu in 1..count {
            send_ipi(cpu, 0x4600);
            timer_ticks();
        }
    }
    CPU_COUNT.store(count, Ordering::Release);
    true
}
