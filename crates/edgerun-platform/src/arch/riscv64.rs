//! RISC-V (rv64) CLINT implementation
//! 
//! CLINT = Core-Local Interruptor (provides software interrupts and timer)

#![allow(unsafe_op_in_unsafe_fn)]

//! CLINT base (virtual address in OpenSBI/BBL)
const CLINT_BASE: usize = 0x24000000;

/// CLINT registers (MTIME is at CLINT_BASE)
const CLINT_MSIP: usize = 0x0000;     // per-hart MSIP (write 1 to signal)
const CLINT_MTIMECMP: usize = 0x4000;    // per-hart time compare (4 bytes per hart)
const CLINT_MTIME: usize = 0xBFF8;    // time register (shared, 8 bytes)

/// CLINT_MSIP offset per hart
const CLINT_MSIP_HART: usize = 0x4;

/// Size of per-hart region
const CLINT_HART_SIZE: usize = 0x10000;

/// CSR addresses
const CSR_MSTATUS: u16 = 0x300;
const CSR_MIE: u16 = 0x304;
const CSR_MTVEC: u16 = 0x305;
const CSR_MSCRATCH: u16 = 0x340;
const CSR_MEPC: u16 = 0x341;
const CSR_MCAUSE: u16 = 0x342;
const CSR_MTVAL: u16 = 0x343;
const CSR_MIP: u16 = 0x344;
const CSR_TSELECT: u16 = 0x7A0;
const CSR_TDATA1: u16 = 0x7A1;
const CSR_TDATA2: u16 = 0x7A2;
const CSR_TDATA3: u16 = 0x7A3;
const CSR_TINFO: u16 = 0x7A4;

/// Machine software interrupt pending (for IPI)
pub const MIP_MSIP: u64 = 1 << 3;
/// Machine timer interrupt
pub const MIP_MTIP: u64 = 1 << 7;

/// Send software interrupt to a hart (IPI)
#[inline]
pub fn soft_int(hart: u8) {
    unsafe {
        let addr = CLINT_BASE + (hart as usize * CLINT_MSIP_HART);
        core::arch::asm!(
            "li t0, 1",
            "sw t0, 0({0})",
            in("x0") addr,
        );
    }
}

/// Clear software interrupt
#[inline]
pub fn clear_soft_int(hart: u8) {
    unsafe {
        let addr = CLINT_BASE + (hart as usize * CLINT_MSIP_HART);
        core::arch::asm!(
            "sw zero, 0({0})",
            in("x0") addr,
        );
    }
}

/// Get current time (mtime)
#[inline]
pub fn time() -> u64 {
    let t: u64;
    unsafe {
        core::arch::asm!(
            "ld {}, 0({})",
            out("x0") t,
            in("x0") CLINT_BASE + 0xBFF8,
        );
    }
    t
}

/// Set mtimecmp for timer interrupt
#[inline]
pub fn set_mtimecmp(hart: u8, deadline: u64) {
    unsafe {
        let addr = CLINT_BASE + CLINT_MTIMECMP + (hart as usize * 8);
        core::arch::asm!(
            "sd {}, 0({})",
            in("x0") deadline,
            in("x0") addr,
        );
    }
}

/// Get hart ID (from CSR)
#[inline]
pub fn hart_id() -> u8 {
    let h: u64;
    core::arch::asm!("csrr {}, mhartid", out("x0") h);
    h as u8
}

/// This hart's ID
#[inline]
pub fn this_cpu_id() -> u8 {
    hart_id()
}

/// Enable machine timer interrupt
#[inline]
pub fn enable_timer() {
    core::arch::asm!(
        "csrs mie, {}",
        in("x0") 1 << 7, // MTIE
    );
}

/// Disable machine timer interrupt
#[inline]
pub fn disable_timer() {
    core::arch::asm!(
        "csrc mie, {}",
        in("x0") 1 << 7,
    );
}

/// Enable machine software interrupt  
#[inline]
pub fn enable_soft_int() {
    core::arch::asm!(
        "csrs mie, {}",
        in("x0") 1 << 3, // MSIE
    );
}

/// Disable machine software interrupt
#[inline]
pub fn disable_soft_int() {
    core::arch::asm!(
        "csrc mie, {}",
        in("x0") 1 << 3,
    );
}

/// End of interrupt (clear MIP)
#[inline]
pub fn eoi() {
    // S-mode clears by writing to mip
    core::arch::asm!("csrc mip, {}", in("x0") MIP_MSIP);
}

/// Read mstatus
#[inline]
pub fn mstatus() -> u64 {
    let v: u64;
    core::arch::asm!("csrr {}, mstatus", out("x0") v);
    v
}

/// Read mpdeleg/medeleg (trap delegation)
#[inline]
pub fn medeleg() -> u64 {
    let v: u64;
    core::arch::asm!("csrr {}, medeleg", out("x0") v);
    v
}

/// Read mie (interrupt enable)
#[inline]
pub fn mie() -> u64 {
    let v: u64;
    core::arch::asm!("csrr {}, mie", out("x0") v);
    v
}

/// Read mip (interrupt pending)
#[inline]
pub fn mip() -> u64 {
    let v: u64;
    core::arch::asm!("csrr {}, mip", out("x0") v);
    v
}