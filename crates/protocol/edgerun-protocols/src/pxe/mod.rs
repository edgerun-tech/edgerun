//! PXE/iPXE and UNDI firmware ABI constants and parameter blocks.
//!
//! This module owns the stable ABI data layout. It does not invoke BIOS
//! interrupts, touch firmware, probe devices, or own network resources.

pub const PXENV_STOP: u16 = 0x0000;
pub const PXENV_UNDI_ISR: u16 = 0x0002;
pub const PXENV_UNDI_GET_INFORMATION: u16 = 0x0010;
pub const PXENV_UNDI_GET_STATISTICS: u16 = 0x0011;
pub const PXENV_UNDI_STARTUP: u16 = 0x0018;
pub const PXENV_START_UNDI: u16 = 0x0019;
pub const PXENV_UNDI_OPEN: u16 = 0x001A;
pub const PXENV_UNDI_CLOSE: u16 = 0x001B;
pub const PXENV_UNDI_TRANSMIT: u16 = 0x001C;
pub const PXENV_UNDI_RECEIVE: u16 = 0x001D;
pub const PXENV_START_NBP: u16 = 0x0020;
pub const PXENV_STOP_NBP: u16 = 0x0021;

pub const INT1A_VECTOR: u8 = 0x1a;
pub const PXENV_BIOS_INFO: u16 = 0x0086;

pub const MAC_ADDR_LEN: usize = 16;
pub const MAX_PACKET_SIZE: usize = 1514;
pub const MAX_MCAST_ADDRESSES: usize = 16;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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

impl Pxenv {
    pub fn for_call(hook_id: u16) -> Self {
        Self {
            hook_id,
            ..Self::default()
        }
    }

    pub fn startup() -> Self {
        Self::for_call(PXENV_UNDI_STARTUP)
    }

    pub fn open() -> Self {
        Self::for_call(PXENV_UNDI_OPEN)
    }

    pub fn close() -> Self {
        Self::for_call(PXENV_UNDI_CLOSE)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct SUndi {
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct UndiInfo {
    pub status: u16,
    pub mac_addr: [u8; MAC_ADDR_LEN],
    pub mac_len: u8,
    pub media_header: u16,
    pub max_packet: u16,
    pub rx_align: u8,
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn pxenv_helpers_set_expected_hook_ids() {
        assert_eq!(Pxenv::startup().hook_id, PXENV_UNDI_STARTUP);
        assert_eq!(Pxenv::open().hook_id, PXENV_UNDI_OPEN);
        assert_eq!(Pxenv::close().hook_id, PXENV_UNDI_CLOSE);
        assert_eq!(Pxenv::for_call(PXENV_START_NBP).hook_id, PXENV_START_NBP);
    }

    #[test]
    fn abi_layouts_are_c_compatible() {
        assert_eq!(align_of::<Pxenv>(), 4);
        assert_eq!(size_of::<Pxenv>(), 48);
        assert_eq!(size_of::<SUndi>(), 40);
        assert_eq!(size_of::<UndiInfo>(), 26);
    }
}
