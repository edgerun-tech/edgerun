//! edgerun-platform: Bare-metal platform abstraction

#![no_std]

#[cfg(not(target_os = "none"))]
extern crate std;

pub mod arch;
pub mod cpu;
pub mod irq;
pub mod timer;
pub mod tls;
pub mod waker;

mod allocator;
pub use allocator::Allocator;

pub use cpu::{all_cpus, smp_init, this_cpu, CpuId, PerCpu};
pub use irq::{Ipi, Irq, IrqController, IrqHandler};
pub use timer::{MonoTime, Timer};
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
