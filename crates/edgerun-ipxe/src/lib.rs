//! iPXE API wrapper
//! Provides FFI to iPXE burned into SPI flash

#![no_std]

pub const PXENV_STOP: u16 = 0x0000;
pub const PXENV_START_UNDI: u16 = 0x0019;
pub const PXENV_START_NBP: u16 = 0x0020;
pub const PXENV_STOP_NBP: u16 = 0x0021;

const PXENV_UNDI_ISR: u16 = 0x0002;
const PXENV_UNDI_STARTUP: u16 = 0x0018;

pub const MAC_ADDR_LEN: usize = 16;

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

pub const INT1A_VECTOR: u8 = 0x1a;
pub const PXENV_BIOS_INFO: u16 = 0x0086;