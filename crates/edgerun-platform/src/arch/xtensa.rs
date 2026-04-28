//! Xtensa (LX series) implementation
//!
//! Xtensa uses the Configurable Interrupt Controller (CIC) and Event/Exception handling

#![allow(unsafe_op_in_unsafe_fn)]

/// Interrupt numbers
pub const INTLEVEL: u32 = 0x0;
pub const NMI: u32 = 2;

/// Get current CPU ID
#[inline]
pub fn this_cpu_id() -> u8 {
    let id: u32;
    unsafe {
        core::arch::asm!("rsr {id}, PRID", id = lateout(reg) id);
    }
    id as u8
}

/// Read CCOUNT (cycle counter)
#[inline]
pub fn ccount() -> u32 {
    let c: u32;
    unsafe {
        core::arch::asm!("rsr {c}, CCOUNT", c = lateout(reg) c);
    }
    c
}

/// Read CCOMPAREn (compare register for timer)
#[inline]
pub fn ccompare() -> u32 {
    let c: u32;
    unsafe {
        core::arch::asm!("rsr {c}, CCOMPARE0", c = lateout(reg) c);
    }
    c
}

/// Set CCOMPARE to trigger timer interrupt
#[inline]
pub fn set_ccompare(deadline: u32) {
    unsafe {
        core::arch::asm!("wsr {deadline}, CCOMPARE0", deadline = in(reg) deadline);
    }
}

/// Enable interrupt
#[inline]
pub fn enable_int(level: u8) {
    let mask = 1u32 << level;
    unsafe {
        core::arch::asm!("wsr {mask}, INTENABLE", mask = in(reg) mask);
    }
}

/// Disable interrupt
#[inline]
pub fn disable_int(level: u8) {
    let mask = !(1u32 << level);
    let enabled: u32;
    unsafe {
        core::arch::asm!("rsr {enabled}, INTENABLE", enabled = lateout(reg) enabled);
        core::arch::asm!("wsr {enabled}, INTENABLE", enabled = in(reg) enabled & mask);
    }
}

/// Check pending interrupts
#[inline]
pub fn pending() -> u32 {
    let p: u32;
    unsafe {
        core::arch::asm!("rsr {p}, INTERRUPT", p = lateout(reg) p);
    }
    p
}

/// Clear interrupt
#[inline]
pub fn clear(irq: u32) {
    unsafe {
        core::arch::asm!("wsr {irq}, INTCLEAR", irq = in(reg) irq);
    }
}

/// End Of Interrupt (clear N-level pending)
#[inline]
pub fn eoi(irq: u32) {
    let mask = 1u32 << irq;
    unsafe {
        core::arch::asm!("wsr {mask}, INTCLEAR", mask = in(reg) mask);
    }
}

/// Set dispatch handler (called on IRQ)
#[inline]
pub fn set_dispatch_handler(ptr: usize) {
    unsafe {
        core::arch::asm!("wsr {ptr}, EXCSAVE1", ptr = in(reg) ptr as u32);
    }
}

/// Read EXCSAVE (dispatch handler pointer)
#[inline]
pub fn dispatch_handler() -> usize {
    let ptr: u32;
    unsafe {
        core::arch::asm!("rsr {ptr}, EXCSAVE1", ptr = lateout(reg) ptr);
    }
    ptr as usize
}

/// Memory barrier
#[inline]
pub fn mem_wmb() {
    unsafe {
        core::arch::asm!("memw");
    }
}

/// Instruction barrier
#[inline]
pub fn instr_sync() {
    unsafe {
        core::arch::asm!("isync");
    }
}

/// Enable global interrupt
#[inline]
pub fn enable() {
    unsafe {
        core::arch::asm!("rsil a2, 0", out("a2") _);
    }
}

/// Set interrupt level
#[inline]
pub fn set_level(level: u32) {
    unsafe {
        match level.min(15) {
            0 => core::arch::asm!("rsil a2, 0", out("a2") _),
            1 => core::arch::asm!("rsil a2, 1", out("a2") _),
            2 => core::arch::asm!("rsil a2, 2", out("a2") _),
            3 => core::arch::asm!("rsil a2, 3", out("a2") _),
            4 => core::arch::asm!("rsil a2, 4", out("a2") _),
            5 => core::arch::asm!("rsil a2, 5", out("a2") _),
            6 => core::arch::asm!("rsil a2, 6", out("a2") _),
            7 => core::arch::asm!("rsil a2, 7", out("a2") _),
            8 => core::arch::asm!("rsil a2, 8", out("a2") _),
            9 => core::arch::asm!("rsil a2, 9", out("a2") _),
            10 => core::arch::asm!("rsil a2, 10", out("a2") _),
            11 => core::arch::asm!("rsil a2, 11", out("a2") _),
            12 => core::arch::asm!("rsil a2, 12", out("a2") _),
            13 => core::arch::asm!("rsil a2, 13", out("a2") _),
            14 => core::arch::asm!("rsil a2, 14", out("a2") _),
            _ => core::arch::asm!("rsil a2, 15", out("a2") _),
        }
    }
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
    dispatch_handler()
}
