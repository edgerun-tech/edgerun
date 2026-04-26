//! Interrupt controller abstraction
//! 
//! LAPIC (x86), GIC (ARM), CLINT (RISC-V), GIC (xtensa)

#![allow(unsafe_op_in_unsafe_fn)]

extern crate alloc;

use core::sync::atomic::Ordering;

/// IRQ number (0-15ISA, 16-255 I/O APIC)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Irq(pub u16);

impl Irq {
    pub fn new(n: u16) -> Self {
        Self(n)
    }
    
    /// Timer IRQ
    pub const TIMER: Self = Self(0);
    /// IPI IRQ for reschedule
    pub const RESCHEDULE: Self = Self(0xFEE0);
}

/// Interrupt handler function
pub type IrqHandler = fn(Irq);

/// Interrupt controller trait
pub trait IrqController {
    fn enable(&self, irq: Irq);
    fn disable(&self, irq: Irq);
    fn eoi(&self);
    fn mask(&self, irq: Irq);
    fn unmask(&self, irq: Irq);
}

/// IPI (Inter-Processor Interrupt)
pub struct Ipi {
    pub target: u8,        // CPU ID or 0xFF for broadcast
    pub vector: u8,         // IRQ to trigger
}

impl Ipi {
    /// Send reschedule IPI to another CPU
    pub fn reschedule(cpu: u8) {
        #[cfg(target_arch = "x86_64")]
        crate::arch::x86_64::send_ipi(cpu, 0xFEE0);
    }
    
    /// Broadcast to all CPUs
    pub fn broadcast(vector: u8) {
        #[cfg(target_arch = "x86_64")]
        crate::arch::x86_64::send_ipi(0xFF, vector as u16);
    }
}

/// Enable interrupts globally
#[inline]
pub unsafe fn enable() {
    #[cfg(target_arch = "x86_64")]
    { unsafe { core::arch::asm!("sti"); } }
}

/// Disable interrupts globally, return old flags
#[inline]
pub unsafe fn disable() -> usize {
    #[cfg(target_arch = "x86_64")]
    {
        let flags: u64;
        unsafe { core::arch::asm!("pushfq", out("rax") flags); }
        unsafe { core::arch::asm!("cli"); }
        flags as usize
    }
    #[cfg(not(target_arch = "x86_64"))]
    0
}

/// Restore interrupt state
#[inline]
pub unsafe fn restore(flags: usize) {
    #[cfg(target_arch = "x86_64")]
    { unsafe { core::arch::asm!("push rax", "popfq", in("rax") flags as u64); } }
}

/// Register interrupt handler
pub fn set_handler(irq: Irq, handler: IrqHandler) {
    // IDT/Vector table setup - arch specific
}