//! Waker implementation using IPI - no allocation version

use core::task::{RawWaker, RawWakerVTable, Waker};
use crate::cpu::CpuId;
use crate::arch::x86_64::send_ipi;

struct IpiWakerCpu(u8);

static mut WAKE_CPU: u8 = 0;

unsafe fn ipi_clone(_d: *const ()) -> RawWaker {
    RawWaker::new(core::ptr::null(), &IPI_VTABLE)
}

unsafe fn ipi_wake(d: *const ()) {
    let cpu = if d.is_null() { WAKE_CPU } else { (*(d as *const IpiWakerCpu)).0 };
    send_ipi(cpu, 0xFEE0);
}

unsafe fn ipi_wake_by_ref(d: *const ()) {
    let cpu = if d.is_null() { WAKE_CPU } else { (*(d as *const IpiWakerCpu)).0 };
    send_ipi(cpu, 0xFEE0);
}

unsafe fn ipi_drop(_d: *const ()) {}

static IPI_VTABLE: RawWakerVTable = RawWakerVTable::new(ipi_clone, ipi_wake, ipi_wake_by_ref, ipi_drop);

pub unsafe fn make_ipi_waker(cpu_id: CpuId) -> Waker {
    WAKE_CPU = cpu_id.0;
    let w = IpiWakerCpu(cpu_id.0);
    let ptr = (&w as *const IpiWakerCpu) as *const ();
    Waker::from_raw(RawWaker::new(ptr, &IPI_VTABLE))
}