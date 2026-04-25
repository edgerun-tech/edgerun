//! Edgerun unikernel - bare-metal entry point.
//! Boot via iPXE -> _start -> main

#![no_std]

use core::arch::asm;

/// Entry point - called by iPXE via direct jump.
#[unsafe(naked)]
pub extern "C" fn _start() -> ! {
    core::arch::naked_asm!(
        "mov rbp, rsp",
        "cld",
        "xor eax, eax",
        "mov ds, ax",
        "mov es, ax",
        "mov fs, ax",
        "mov gs, ax",
        "mov ss, ax",
        "call rust_entry",
        "hlt"
    );
}

/// Rust entry point after minimal setup.
#[inline(never)]
pub unsafe extern "C" fn rust_entry() {
    asm!("cli", options(nostack));
    
    init_bss();
    main();
    
    loop {
        asm!("hlt", options(noreturn));
    }
}

fn init_bss() {
    extern "C" {
        static mut _bss_start: u64;
        static mut _bss_end: u64;
    }
    
    let start = core::ptr::addr_of_mut!(_bss_start) as *mut u8;
    let end = core::ptr::addr_of_mut!(_bss_end) as *mut u8;
    let len = unsafe { end.offset_from(start) } as usize;
    if len > 0 {
        unsafe { start.write_bytes(0, len) };
    }
}

#[no_mangle]
pub extern "C" fn main() -> ! {
    loop {
        unsafe { core::arch::asm!("hlt") };
    }
}

extern "C" {
    pub static mut _bss_start: u64;
    pub static mut _bss_end: u64;
    pub static mut _end: u64;
}