#![no_std]
#![no_main]

extern crate alloc;

use core::alloc::{GlobalAlloc, Layout};
use core::panic::PanicInfo;

#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(
    r#"
.section .multiboot
.align 4
.long 0x1BADB002
.long 0x00000003
.long -(0x1BADB002 + 0x00000003)

.section .text.entry
.global _start
_start:
    cli
    lea rsp, [rip + _stack_top]
    call edgerun_relay_qemu_smoke_main
1:
    hlt
    jmp 1b

.section .bss.stack
.align 16
_stack_bottom:
    .skip 65536
_stack_top:
"#
);

#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator;

struct BumpAllocator;

const HEAP_START: usize = 0x10_0000;
const HEAP_END: usize = 0x40_0000;
static NEXT: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(HEAP_START);

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();
        if size == 0 {
            return core::ptr::null_mut();
        }
        let mut current = NEXT.load(core::sync::atomic::Ordering::Acquire);
        loop {
            let aligned = (current + align - 1) & !(align - 1);
            let next = aligned.saturating_add(size);
            if next > HEAP_END {
                return core::ptr::null_mut();
            }
            match NEXT.compare_exchange(
                current,
                next,
                core::sync::atomic::Ordering::AcqRel,
                core::sync::atomic::Ordering::Acquire,
            ) {
                Ok(_) => return aligned as *mut u8,
                Err(value) => current = value,
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_relay_qemu_smoke_main() -> ! {
    serial_write(b"edgerun-relay qemu smoke: start\n");
    match edgerun_relay::self_signed_wss_config(&["localhost"]) {
        Ok(config)
            if !config.certificate().cert_der.is_empty()
                && !config.certificate().cert_chain_der.is_empty() =>
        {
            serial_write(b"edgerun-relay qemu smoke: wss cert in memory\n");
            qemu_exit(0);
        }
        Ok(_) => {
            serial_write(b"edgerun-relay qemu smoke: empty cert material\n");
            qemu_exit(1);
        }
        Err(_) => {
            serial_write(b"edgerun-relay qemu smoke: wss config failed\n");
            qemu_exit(1);
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    serial_write(b"edgerun-relay qemu smoke: panic\n");
    qemu_exit(1);
}

fn serial_write(bytes: &[u8]) {
    for &byte in bytes {
        unsafe {
            outb(0x3f8, byte);
        }
    }
}

fn qemu_exit(code: u8) -> ! {
    unsafe {
        outb(0x501, (code << 1) | 1);
    }
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

#[cfg(target_arch = "x86_64")]
unsafe fn outb(port: u16, value: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") port, in("al") value);
    }
}
