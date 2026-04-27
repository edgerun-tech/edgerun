//! Xtensa (LX series) implementation
//!
//! Xtensa uses the Configurable Interrupt Controller (CIC) and Event/Exception handling

#![allow(unsafe_op_in_unsafe_fn)]

/// Platform peripheral base (PMA)
const PQMA_BASE: usize = 0x60000000;

/// Timer/clock control
const CCOUNT: usize = 0x208;

/// Interrupt controller registers
const INTENABLE: usize = 0x230;
const INTERRUPT: usize = 0x234;
const INTCLEAR: usize = 0x238;

/// CPU SAR (Special Angle Registers)
/// EXCSAVE - holds pointer to current exception handler state
const XER_SAR: usize = 35;

/// Interrupt numbers
pub const INTLEVEL: u32 = 0x0;
pub const NMI: u32 = 2;

/// Get current CPU ID
#[inline]
pub fn this_cpu_id() -> u8 {
    let id: u32;
    core::arch::asm!("rsr {}, 231", out("a0") id); // PRID
    id as u8
}

/// Read CCOUNT (cycle counter)
#[inline]
pub fn ccount() -> u32 {
    let c: u32;
    core::arch::asm!("rsr CCOUNT", out("a0") c);
    c
}

/// Read CCOMPAREn (compare register for timer)
#[inline]
pub fn ccompare() -> u32 {
    let c: u32;
    core::arch::asm!("rsr CCOMPARE0", out("a0") c);
    c
}

/// Set CCOMPARE to trigger timer interrupt
#[inline]
pub fn set_ccompare(deadline: u32) {
    core::arch::asm!("wsr CCOMPARE0, {}", in("a0") deadline);
}

/// Enable interrupt
#[inline]
pub fn enable_int(level: u8) {
    let mask = 1u32 << level;
    core::arch::asm!(
        "movi a1, {mask}",
        "wsr INTENABLE, a1",
        mask = in("a1") mask,
    );
}

/// Disable interrupt
#[inline]
pub fn disable_int(level: u8) {
    let mask = 1u32 << level;
    core::arch::asm!(
        "movi a1, {mask}",
        "wsr INTENABLE, a1",
        mask = in("a1") 0,
    );
}

/// Check pending interrupts
#[inline]
pub fn pending() -> u32 {
    let p: u32;
    core::arch::asm!("rsr INTERRUPT", out("a0") p);
    p
}

/// Clear interrupt
#[inline]
pub fn clear(irq: u32) {
    core::arch::asm!("wsr INTCLEAR, {}", in("a0") irq);
}

/// End Of Interrupt (clear N-level pending)
#[inline]
pub fn eoi(irq: u32) {
    core::arch::asm!("wsr INTCLEAR, {}", in("a0") 1u32 << irq);
}

/// Set dispatch handler (called on IRQ)
#[inline]
pub fn set_dispatch_handler(ptr: usize) {
    core::arch::asm!("wsr EXCSAVE, {}", in("a0") ptr);
}

/// Read EXCSAVE (dispatch handler pointer)
#[inline]
pub fn dispatch_handler() -> usize {
    let ptr: usize;
    core::arch::asm!("rsr EXCSAVE", out("a0") ptr);
    ptr
}

/// Memory barrier
#[inline]
pub fn mem_wmb() {
    core::arch::asm!("memw");
}

/// Instruction barrier
#[inline]
pub fn instr_sync() {
    core::arch::asm!("isync");
}

/// Enable global interrupt
#[inline]
pub fn enable() {
    core::arch::asm!("rsil a0, 0");
}

/// Set interrupt level
#[inline]
pub fn set_level(level: u32) {
    core::arch::asm!("rsil a0, {}", in("a0") level.min(15));
}

/// Initialize timer for async
pub fn timer_init() {
    // Set up CCOMPARE for 1ms tick
    let now = ccount();
    set_ccompare(now.wrapping_add(10_000));
}

/// Get Tls pointer from EXCSAVE area
#[inline]
pub fn tls() -> usize {
    let ptr: usize;
    core::arch::asm!("rsr EXCSAVE", out("a0") ptr);
    ptr
}
