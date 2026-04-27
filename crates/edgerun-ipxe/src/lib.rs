//! iPXE API wrapper
//! Provides FFI to iPXE burned into SPI flash
//! Use iPXE's built-in RTL8125 driver via UNDI interface

#![no_std]

pub const PXENV_STOP: u16 = 0x0000;
pub const PXENV_START_UNDI: u16 = 0x0019;
pub const PXENV_START_NBP: u16 = 0x0020;
pub const PXENV_STOP_NBP: u16 = 0x0021;
pub const PXENV_UNDI_STARTUP: u16 = 0x0018;
pub const PXENV_UNDI_OPEN: u16 = 0x001A;
pub const PXENV_UNDI_CLOSE: u16 = 0x001B;
pub const PXENV_UNDI_TRANSMIT: u16 = 0x001C;
pub const PXENV_UNDI_RECEIVE: u16 = 0x001D;
pub const PXENV_UNDI_ISR: u16 = 0x0002;
pub const PXENV_UNDI_GET_INFORMATION: u16 = 0x0010;
pub const PXENV_UNDI_GET_STATISTICS: u16 = 0x0011;

pub const INT1A_VECTOR: u8 = 0x1a;
pub const PXENV_BIOS_INFO: u16 = 0x0086;

pub const MAC_ADDR_LEN: usize = 16;

pub const MAX_PACKET_SIZE: usize = 1514;
pub const MAX_MCAST_ADDRESSES: usize = 16;

#[derive(Clone, Copy, Default)]
#[repr(C)]
pub struct Pxenv {
    pub hook_id: u16,
    pub reserved: u16,
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
    pub esi: u32,
    pub edi: u32,
    pub eflags: u16,
    pub cs: u16,
    pub ss: u32,
    pub fs: u16,
    pub gs: u16,
    pub ip: u16,
    pub sp: u16,
    pub bp: u16,
}

#[derive(Clone, Copy, Default)]
#[repr(C)]
pub struct S_UNDI {
    pub junk: [u8; 8],
    pub flags: u8,
    pub pending: u8,
    pub rq: u8,
    pub tq: u8,
    pub base: [u8; 8],
    pub hw_addr: [u8; MAC_ADDR_LEN],
    pub hw_addr_len: u8,
    pub link_status: u16,
}

#[derive(Clone, Copy, Default)]
#[repr(C)]
pub struct UndiInfo {
    pub status: u16,
    pub mac_addr: [u8; MAC_ADDR_LEN],
    pub mac_len: u8,
    pub media_header: u16,
    pub max_packet: u16,
    pub rx_align: u8,
}

#[no_mangle]
pub unsafe extern "C" fn undi_call(func: u16, pxenv: &mut Pxenv) {
    core::arch::asm!(
        "int 0x1a",
        in("ax") func,
        in("si") pxenv,
    );
}

pub unsafe fn call_undi(func: u16, pxenv: &mut Pxenv) {
    undi_call(func, pxenv);
}

impl Pxenv {
    pub fn startup() -> Self {
        let mut pxenv = Pxenv::default();
        pxenv.hook_id = PXENV_UNDI_STARTUP;
        unsafe { call_undi(PXENV_UNDI_STARTUP, &mut pxenv) };
        pxenv
    }

    pub fn open() -> Self {
        let mut pxenv = Pxenv::default();
        pxenv.hook_id = PXENV_UNDI_OPEN;
        unsafe { call_undi(PXENV_UNDI_OPEN, &mut pxenv) };
        pxenv
    }

    pub fn close() -> Self {
        let mut pxenv = Pxenv::default();
        pxenv.hook_id = PXENV_UNDI_CLOSE;
        unsafe { call_undi(PXENV_UNDI_CLOSE, &mut pxenv) };
        pxenv
    }
}
