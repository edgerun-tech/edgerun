//! Edgerun unikernel binary entry
//! Boot via iPXE -> _start -> main

#![no_std]
#![no_main]

use edgerun_bare_rt::Duration;
use edgerun_ipxe::Pxenv;
use edgerun_rtl8125::{Rtl8125, REG_CMD, CMD_RESET};
use edgerun_tftp::{TftpHeader, TftpData, OP_RRQ, OP_DATA, OP_ACK, TFTP_BLOCK_SIZE};

static mut STACK: [u8; 16384] = [0; 16384];
static mut BSS: [u8; 4096] = [0; 4096];
static mut RX_BUF: [u8; 8192] = [0; 8192];
static mut TX_BUF: [u8; 8192] = [0; 8192];

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! { 
    loop { unsafe { core::arch::asm!("hlt", options(noreturn)); } }
}

#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    let stack_top = STACK.as_ptr().add(STACK.len()) as usize;
    core::arch::asm!("mov rsp, {}", in(reg) stack_top, options(nostack));
    
    BSS.fill(0);
    
    main();
    
    loop { core::arch::asm!("hlt", options(noreturn)); }
}

#[no_mangle]
pub extern "C" fn main() {
    init_hardware();
    boot_network();
}

fn init_hardware() {
    let mut nic = Rtl8125::new(0x3000);
    nic.reset();
}

fn boot_network() {
    let mut pxenv = Pxenv::default();
    let (server_ip, filename) = dhcp(&mut pxenv);
    
    if server_ip != 0 {
        let kernel_addr = load_via_tftp(server_ip, filename);
        if kernel_addr != 0 {
            jump_to_kernel(kernel_addr);
        }
    }
}

fn dhcp(pxenv: &mut Pxenv) -> (u32, &'static str) {
    (0, "edgerun.bin")
}

fn load_via_tftp(server_ip: u32, filename: &str) -> usize {
    let mut block_num: u16 = 1;
    let mut tftp = TftpHeader::default();
    let mut data = TftpData::default();
    
    0
}

fn jump_to_kernel(addr: usize) -> ! {
    unsafe {
        let func: extern "C" fn() -> ! = core::mem::transmute(addr);
        func();
    }
}