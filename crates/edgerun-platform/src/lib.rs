//! edgerun-platform: Bare-metal platform abstraction

#![no_std]

pub mod cpu;
pub mod timer;
pub mod irq;
pub mod tls;
pub mod arch;
pub mod waker;

mod allocator;
pub use allocator::Allocator;

pub use cpu::{CpuId, PerCpu, this_cpu, all_cpus, smp_init};
pub use timer::{MonoTime, Timer};
pub use irq::{Irq, IrqHandler, IrqController, Ipi};
pub use tls::TlsArea;
pub use waker::make_ipi_waker;

#[inline]
pub unsafe fn halt() -> ! {
    #[cfg(target_arch = "x86_64")]
    core::arch::asm!("hlt", options(noreturn));
    #[cfg(not(target_arch = "x86_64"))]
    loop {}
}

#[inline]
pub unsafe fn yield_cpu() {
    core::arch::asm!("pause");
}