//! Edgerun unikernel - bare shell

#![no_std]
#![no_main]

use edgerun_virtio::find_virtio_net;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! { 
    loop { unsafe { core::arch::asm!("hlt", options(noreturn)); } }
}

#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    main();
    loop { unsafe { core::arch::asm!("hlt", options(noreturn)); } }
}

#[no_mangle]
pub unsafe extern "C" fn main() {
    let _net = find_virtio_net();
    loop {
        unsafe { core::arch::asm!("pause") };
    }
}