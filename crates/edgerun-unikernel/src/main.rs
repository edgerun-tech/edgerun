//! Edgerun unikernel - bare shell with networking

#![no_std]
#![no_main]

extern crate edgerun_bare_rt as rt;
extern crate edgerun_virtio;
extern crate edgerun_platform;

use rt::{DhcpClient, TftpClient, TftpConfig};

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
    rt::timer::set_now(0);
    
    let mut net = match edgerun_virtio::find_virtio_net() {
        Some(n) => n,
        None => loop { core::arch::asm!("hlt") },
    };
    
    net.init();
    
    let mac = net.get_mac();
    let mut dhcp = DhcpClient::new(mac);
    
    let _tftp = TftpConfig::new(0xC0A80101, "edgerun.bin");
    
    if net.is_link_up() {
        rt::log::log(3, "VirtIO Net: OK");
        rt::log::log(3, "MAC: ");
    }
    
    loop {
        rt::timer::set_now(rt::timer::now() + 1);
        unsafe { core::arch::asm!("pause") };
    }
}