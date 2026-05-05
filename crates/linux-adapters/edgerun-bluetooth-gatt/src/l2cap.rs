use crate::prelude::v1::*;

pub(crate) const AF_BLUETOOTH: i32 = 31;
pub(crate) const SOCK_SEQPACKET: i32 = 5;
pub(crate) const BTPROTO_L2CAP: i32 = 0;

pub(crate) const ATT_CID: u16 = 0x0004;
pub(crate) const DEFAULT_MTU: u16 = 23;
pub(crate) const MAX_MTU: u16 = 512;
pub(crate) const LE_PSM: u16 = 0x002F;

pub(crate) const OCF_L2CAP_CONN_REQ: u16 = 0x0401;
pub(crate) const OCF_L2CAP_CONFIG_REQ: u16 = 0x0411;
pub(crate) const OCF_L2CAP_DISCONN_REQ: u16 = 0x0403;
pub(crate) const OCF_L2CAP_INFO_REQ: u16 = 0x040A;

pub(crate) const HCI_EV_LE_META_EVENT: u8 = 0x3E;
pub(crate) const HCI_EV_CONN_COMPLETE: u8 = 0x13;
pub(crate) const HCI_EV_DISCONN_COMPLETE: u8 = 0x05;

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct SockAddrL2 {
    pub(crate) l2_family: u16,
    pub(crate) l2_psm: u16,
    pub(crate) l2_bdaddr: [u8; 6],
    pub(crate) l2_cid: u16,
    pub(crate) l2_bdaddr_type: u8,
}

impl Default for SockAddrL2 {
    fn default() -> Self {
        Self {
            l2_family: AF_BLUETOOTH as u16,
            l2_psm: 0,
            l2_bdaddr: [0; 6],
            l2_cid: 0,
            l2_bdaddr_type: 0,
        }
    }
}

impl SockAddrL2 {
    pub(crate) fn for_att_device(addr: &[u8; 6], addr_type: u8) -> Self {
        Self {
            l2_family: AF_BLUETOOTH as u16,
            l2_cid: ATT_CID,
            l2_bdaddr_type: addr_type,
            l2_bdaddr: *addr,
            l2_psm: 0,
        }
    }

    pub(crate) fn for_device(addr: &[u8; 6], addr_type: u8, psm: u16) -> Self {
        Self {
            l2_family: AF_BLUETOOTH as u16,
            l2_cid: 0,
            l2_bdaddr_type: addr_type,
            l2_bdaddr: *addr,
            l2_psm: psm,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum L2capChannelState {
    Closed,
    Opening,
    Open,
    ConfigRequest,
    ConfigResponse,
    Connected,
    Disconnecting,
}

pub(crate) fn parse_bdaddr_string(addr: &str) -> Option<[u8; 6]> {
    edgerun_encoding::hex::parse_mac(addr)
}

pub(crate) fn format_bdaddr_hex(bytes: &[u8]) -> String {
    edgerun_encoding::hex::format_mac_bytes(bytes).unwrap_or_default()
}

pub(crate) fn reverse_bdaddr(addr: &str) -> Option<[u8; 6]> {
    parse_bdaddr_string(addr).map(|mut a| {
        a.reverse();
        a
    })
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct PollFd {
    pub(crate) fd: i32,
    pub(crate) events: i16,
    pub(crate) revents: i16,
}

unsafe extern "C" {
    pub(crate) fn socket(domain: i32, ty: i32, protocol: i32) -> i32;
    pub(crate) fn bind(fd: i32, addr: *const core::ffi::c_void, len: u32) -> i32;
    pub(crate) fn connect(fd: i32, addr: *const core::ffi::c_void, len: u32) -> i32;
    pub(crate) fn send(fd: i32, buf: *const core::ffi::c_void, len: usize, flags: i32) -> isize;
    pub(crate) fn recv(fd: i32, buf: *mut core::ffi::c_void, len: usize, flags: i32) -> isize;
    pub(crate) fn poll(fds: *mut PollFd, nfds: usize, timeout: i32) -> i32;
    pub(crate) fn close(fd: i32) -> i32;
    pub(crate) fn dup(fd: i32) -> i32;
}
