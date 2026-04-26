//! x86_64 LAPIC (Local APIC) implementation

#![allow(unsafe_op_in_unsafe_fn)]

/// Send IPI to a specific CPU or broadcast (cpu=0xFF)
#[inline]
pub fn send_ipi(cpu: u8, vector: u16) {
    let icr = ((cpu as u32) << 24) | (vector as u32) | (1 << 14);
    unsafe {
        core::arch::asm!(
            "mov dx, 0xFEE00030",
            "mov eax, eax",
            "out dx, eax",
            "2: mov dx, 0xFEE00030",
            "in eax, dx",
            "and eax, 0x1000",
            "jnz 2b",
            in("eax") icr,
            out("dx") _,
        );
    }
}

/// Read LAPIC ID
#[inline]
pub fn id() -> u8 {
    unsafe {
        let id: u32;
        core::arch::asm!(
            "mov eax, 0",
            "mov dx, 0xFEE00030",
            "out dx, eax",
            "mov dx, 0xFEE00034",
            "in eax, dx",
            out("rax") id,
            out("dx") _,
        );
        ((id >> 24) & 0xFF) as u8
    }
}

/// End Of Interrupt
#[inline]
pub fn eoi() {
    unsafe {
        core::arch::asm!(
            "mov dx, 0xFEE000B0",
            "mov eax, 0",
            "out dx, eax"
        );
    }
}

/// Enable LAPIC
#[inline]
pub fn enable() {
    unsafe {
        core::arch::asm!(
            "mov dx, 0xFEE000F0",
            "mov eax, 0x1FF",
            "out dx, eax"
        );
    }
}

/// Disable LAPIC
#[inline]
pub fn disable() {
    unsafe {
        core::arch::asm!(
            "mov dx, 0xFEE000F0",
            "mov eax, 0",
            "out dx, eax"
        );
    }
}

/// Get current TSC value
#[inline]
pub fn rdtsc() -> u64 {
    let lo: u32;
    let hi: u32;
    unsafe {
        core::arch::asm!(
            "rdtsc",
            out("rax") lo,
            out("rdx") hi,
        );
    }
    ((hi as u64) << 32) | (lo as u64)
}