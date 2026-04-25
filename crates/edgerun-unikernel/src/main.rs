//! Edgerun unikernel binary entry

#![no_std]

fn main() -> ! {
    loop {
        unsafe { core::arch::asm!("hlt") };
    }
}