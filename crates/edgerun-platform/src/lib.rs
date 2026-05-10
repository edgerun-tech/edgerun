//! edgerun-platform: Bare-metal platform abstraction

#![no_std]
#![cfg_attr(target_arch = "xtensa", feature(asm_experimental_arch))]
#![cfg_attr(
    all(
        target_arch = "xtensa",
        target_os = "none",
        feature = "esp32s3-wifi-blob"
    ),
    feature(c_variadic)
)]

extern crate alloc;

pub mod arch;
pub mod cpu;
#[cfg(all(target_arch = "xtensa", target_os = "none"))]
pub mod esp32s3;
#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "esp32s3-ble"))]
pub mod esp32s3_ble;
#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-ble-blob"
))]
pub mod esp32s3_ble_blob;
#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-ble-stub"
))]
pub mod esp32s3_ble_stub;
#[cfg(all(target_arch = "xtensa", target_os = "none"))]
pub mod esp32s3_wifi;
#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-blob"
))]
pub mod esp32s3_wifi_blob;
#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-blob"
))]
mod esp32s3_wifi_blob_init;
#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-blob"
))]
mod esp32s3_wifi_blob_stubs;
#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio"
))]
pub mod esp32s3_wifi_mmio;
pub mod irq;
pub mod timer;
pub mod tls;
pub mod waker;

mod allocator;
pub use allocator::Allocator;

pub use cpu::{CpuId, PerCpu, all_cpus, smp_init, this_cpu};
pub use irq::{Ipi, Irq, IrqController, IrqHandler};
pub use timer::{MonoTime, Timer};
pub use tls::TlsArea;
pub use waker::make_ipi_waker;

#[inline]
pub unsafe fn halt() -> ! {
    unsafe {
        #[cfg(target_arch = "x86_64")]
        core::arch::asm!("hlt", options(noreturn));
        #[cfg(target_arch = "xtensa")]
        loop {
            core::arch::asm!("waiti 0");
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "xtensa")))]
        loop {}
    }
}

#[inline]
pub unsafe fn yield_cpu() {
    unsafe {
        #[cfg(target_arch = "x86_64")]
        core::arch::asm!("pause");
        #[cfg(target_arch = "xtensa")]
        core::arch::asm!("nop");
        #[cfg(not(any(target_arch = "x86_64", target_arch = "xtensa")))]
        core::hint::spin_loop();
    }
}
