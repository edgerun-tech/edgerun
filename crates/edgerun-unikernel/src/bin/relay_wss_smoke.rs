#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;
use core::sync::atomic::{AtomicU64, Ordering};

use edgerun_platform as _;

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
    lea rsp, [rip + _stack]
    call edgerun_unikernel_relay_wss_smoke_main
1:
    hlt
    jmp 1b
"#
);

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_unikernel_relay_wss_smoke_main() -> ! {
    debug_write(b"edgerun-unikernel relay wss smoke: start\n");
    edgerun_crypto::register_random_source(qemu_random);
    debug_write(b"edgerun-unikernel relay wss smoke: rng registered\n");

    match edgerun_relay::self_signed_wss_config(&["localhost"]) {
        Ok(config)
            if !config.certificate().cert_der.is_empty()
                && !config.certificate().cert_chain_der.is_empty() =>
        {
            debug_write(b"edgerun-unikernel relay wss smoke: wss cert in memory\n");
            qemu_exit(0);
        }
        Ok(_) => {
            debug_write(b"edgerun-unikernel relay wss smoke: empty cert material\n");
            qemu_exit(1);
        }
        Err(_) => {
            debug_write(b"edgerun-unikernel relay wss smoke: wss config failed\n");
            qemu_exit(1);
        }
    }
}

static QEMU_RNG_COUNTER: AtomicU64 = AtomicU64::new(1);

fn qemu_random(buf: &mut [u8]) -> edgerun_crypto::error::Result<()> {
    let mut state = QEMU_RNG_COUNTER.fetch_add(buf.len() as u64 + 1, Ordering::AcqRel);
    for byte in buf {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        *byte = state as u8;
    }
    Ok(())
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    debug_write(b"edgerun-unikernel relay wss smoke: panic\n");
    qemu_exit(1);
}

fn debug_write(bytes: &[u8]) {
    for &byte in bytes {
        unsafe {
            outb(0xe9, byte);
        }
    }
}

fn qemu_exit(code: u32) -> ! {
    unsafe {
        outl(0xf4, code);
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

#[cfg(target_arch = "x86_64")]
unsafe fn outl(port: u16, value: u32) {
    unsafe {
        core::arch::asm!("out dx, eax", in("dx") port, in("eax") value);
    }
}
