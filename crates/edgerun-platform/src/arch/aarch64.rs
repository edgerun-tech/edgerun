//! ARM64 (aarch64) GICv2 implementation

#![allow(unsafe_op_in_unsafe_fn)]

/// GICv2 distributor base
const GICD_BASE: usize = 0x1F000000;
/// GICv2 CPU interface base  
const GICC_BASE: usize = 0x1F000100;

/// GICD registers
const GICD_CTLR: usize = 0;
const GICD_TYPER: usize = 4;
const GICD_IIDR: usize = 8;
const GICD_IGROUPR: usize = 0x80;
const GICD_ISENABLER: usize = 0x100;
const GICD_ICENABLER: usize = 0x180;
const GICD_ISPENDD: usize = 0x200;
const GICD_ICPENDD: usize = 0x280;
const GICD_IPRIORITYR: usize = 0x400;
const GICD_ITARGETSR: usize = 0x800;
const GICD_ICFGR: usize = 0xC00;

/// GICC registers
const GICC_CTLR: usize = 0;
const GICC_PMR: usize = 4;
const GICC_BPR: usize = 8;
const GICC_IAR: usize = 0xC;
const GICC_EOIR: usize = 0x10;
const GICC_HPPIR: usize = 0x18;
const GICC_ABPR: usize = 0x1C;

/// Send Software Generated Interrupt (SGI) to another CPU
#[inline]
pub fn send_sgi(cpu: u8, irq: u8) {
    let val = ((irq as u32) << 12) | ((cpu as u32) << 24) | 0x1_0000; // route to specific CPU
    unsafe {
        core::arch::asm!(
            "str wzr, [{0}, 0x40]",
            in("x0") GICD_BASE + 0x1000, // GICD_SGIR
        );
    }
}

/// Enable interrupt
#[inline]
pub fn enable_irq(irq: u8) {
    unsafe {
        let reg = GICD_BASE + GICD_ISENABLER + (irq as usize / 32) * 4;
        let bits = 1u32 << (irq % 32);
        core::arch::asm!(
            "ldr w1, [{0}]",
            "orr w1, w1, {1}",
            "str w1, [{0}]",
            in("x0") reg,
            in("x1") bits,
        );
    }
}

/// Disable interrupt
#[inline]
pub fn disable_irq(irq: u8) {
    unsafe {
        let reg = GICD_BASE + GICD_ICENABLER + (irq as usize / 32) * 4;
        let bits = 1u32 << (irq % 32);
        core::arch::asm!(
            "ldr w1, [{0}]",
            "bic w1, w1, {1}",
            "str w1, [{0}]",
            in("x0") reg,
            in("x1") bits,
        );
    }
}

/// End Of Interrupt
#[inline]
pub fn eoi(irq: u8) {
    unsafe {
        core::arch::asm!(
            "strb wzr, [{0}, 0x20]",
            in("x0") GICC_BASE + 0x10, // GICC_EOIR + irq
        );
    }
}

/// Read Interrupt Acknowledge Register (get current IRQ)
#[inline]
pub fn iar() -> u32 {
    let irq: u32;
    unsafe {
        core::arch::asm!(
            "ldr wzr, [{0}]",
            out("w0") irq,
            in("x0") GICC_BASE + 0xC, // GICC_IAR
        );
    }
    irq
}

/// Set priority mask
#[inline]
pub fn set_pmr(pri: u8) {
    unsafe {
        core::arch::asm!(
            "strb wzr, [{0}]",
            in("x0") GICC_BASE + 4, // GICC_PMR
        );
    }
}

/// Enable GIC
#[inline]
pub fn enable() {
    unsafe {
        core::arch::asm!(
            "mov w0, 1",
            "str w0, [{0}]",
            in("x0") GICC_BASE + 0, // GICC_CTLR
        );
        core::arch::asm!(
            "mov w0, 3",
            "str w0, [{0}]",
            in("x0") GICD_BASE + 0, // GICD_CTLR
        );
    }
}

/// Get current CPU ID
#[inline]
pub fn this_cpu_id() -> u8 {
    // MPIDR_EL1 bits [15:8] = cluster, bits [7:0] = CPU in cluster
    let mpidr: u64;
    core::arch::asm!("mrs {}, MPIDR_EL1", out("x0") mpidr);
    ((mpidr >> 8) & 0xFF) as u8
}