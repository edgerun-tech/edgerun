//! Link-layer transport for the edgerun mesh.
//!
//! Provides three transport paths:
//! 1. **Raw Ethernet** (`AF_PACKET`) — direct L2 communication by MAC address.
//!    Used when two peers share a broadcast domain (same LAN/WiFi).
//! 2. **Multicast UDP** — periodic discovery packets sent to a multicast group
//!    on each active interface.
//! 3. **IP Tunnel** — a UDP pipe to a known peer IP, used when no L2 path
//!    exists (cross-subnet, NAT, internet).
//!
//! All paths carry the same `MeshFrame` wire format.  The link layer
//! attaches the sender's MAC to incoming raw Ethernet frames so the
//! router can learn peer identities from Ethernet source addresses.

#![no_std]

extern crate alloc;

#[cfg(not(target_os = "none"))]
extern crate std;

#[cfg(target_os = "none")]
extern crate self as std;

pub mod prelude {
    pub mod v1 {
        pub use alloc::format;
        pub use alloc::vec;
        pub use alloc::vec::Vec;
        pub use core::clone::Clone;
        pub use core::cmp::{Eq, Ord, PartialEq, PartialOrd};
        pub use core::convert::{AsMut, AsRef, From, Into, TryFrom, TryInto};
        pub use core::default::Default;
        pub use core::marker::{Copy, Send, Sized, Sync};
        pub use core::mem::drop;
        pub use core::option::Option::{self, None, Some};
        pub use core::result::Result::{self, Err, Ok};
    }
}

pub mod collections {
    pub use alloc::collections::{BTreeMap as HashMap, BTreeSet as HashSet, VecDeque};
}

pub mod io {
    use core::fmt;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum ErrorKind {
        WouldBlock,
        Other,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct Error {
        kind: ErrorKind,
    }

    impl Error {
        #[must_use]
        pub const fn new(kind: ErrorKind) -> Self {
            Self { kind }
        }

        #[must_use]
        pub const fn last_os_error() -> Self {
            Self {
                kind: ErrorKind::Other,
            }
        }

        #[must_use]
        pub const fn kind(&self) -> ErrorKind {
            self.kind
        }
    }

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self.kind {
                ErrorKind::WouldBlock => f.write_str("operation would block"),
                ErrorKind::Other => f.write_str("I/O error"),
            }
        }
    }

    impl core::error::Error for Error {}

    pub type Result<T> = core::result::Result<T, Error>;
}

pub mod os {
    pub mod raw {
        #[allow(non_camel_case_types)]
        pub type c_int = i32;
        #[allow(non_camel_case_types)]
        pub type c_void = core::ffi::c_void;
    }
}

pub mod time {
    pub use core::time::Duration;

    #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
    pub struct SystemTime(Duration);

    pub const UNIX_EPOCH: SystemTime = SystemTime(Duration::from_secs(0));

    impl SystemTime {
        #[must_use]
        pub const fn now() -> Self {
            UNIX_EPOCH
        }

        pub fn duration_since(
            &self,
            earlier: SystemTime,
        ) -> core::result::Result<Duration, Duration> {
            self.0.checked_sub(earlier.0).ok_or(earlier.0)
        }
    }
}

pub mod option {
    pub use core::option::*;
}

pub mod result {
    pub use core::result::*;
}

pub mod string {
    pub use alloc::string::*;
}

pub mod vec {
    pub use alloc::vec::*;
}

pub use alloc::format;
pub use core::{mem, ptr};

use std::os::raw::{c_int, c_void};

/// EtherType for edgerun mesh frames (unassigned, in the experiment range).
pub const MESH_ETHERTYPE: u16 = 0x88B5;

/// Multicast group for mesh discovery (239.255.0.1).
const MESH_MCAST_ADDR: [u8; 4] = [239, 255, 0, 1];
const MESH_MCAST_PORT: u16 = 47080;

/// Raw Ethernet protocol number for our EtherType (host byte order).
const ETH_P_MESH: u16 = MESH_ETHERTYPE.to_be();

// Packet socket options (reserved for future BPF filtering)
const SO_ATTACH_FILTER: c_int = 26;
const SOL_PACKET: c_int = 263;
const PACKET_ADD_MEMBERSHIP: c_int = 1;
const PACKET_MR_MULTICAST: c_int = 0;

// IP multicast options
const AF_INET: c_int = 2;
const SOCK_DGRAM: c_int = 2;
const IPPROTO_UDP: c_int = 17;
const IP_ADD_MEMBERSHIP: c_int = 35;
const IP_MULTICAST_IF: c_int = 32;

mod raw_ethernet;

mod link_manager;
mod multicast;
mod tunnel;
mod udp_broadcast;

pub use link_manager::MeshLink;
pub use multicast::MulticastSocket;
pub use raw_ethernet::RawEthernetSocket;
pub use tunnel::IpTunnel;
pub use udp_broadcast::UdpBroadcastSocket;

unsafe extern "C" {
    fn socket(domain: c_int, ty: c_int, protocol: c_int) -> c_int;
    fn bind(fd: c_int, addr: *const c_void, len: u32) -> c_int;
    fn sendto(
        fd: c_int,
        buf: *const c_void,
        len: usize,
        flags: c_int,
        addr: *const c_void,
        addrlen: u32,
    ) -> isize;
    fn recvfrom(
        fd: c_int,
        buf: *mut c_void,
        len: usize,
        flags: c_int,
        addr: *mut c_void,
        addrlen: *mut u32,
    ) -> isize;
    fn setsockopt(
        fd: c_int,
        level: c_int,
        optname: c_int,
        optval: *const c_void,
        optlen: u32,
    ) -> c_int;
    fn close(fd: c_int) -> c_int;
}

const AF_PACKET: c_int = 17;
const SOCK_RAW: c_int = 3;
const SOL_SOCKET: c_int = 1;

#[repr(C)]
struct SockaddrLl {
    sll_family: u16,
    sll_protocol: u16,
    sll_ifindex: c_int,
    sll_hatype: u16,
    sll_pkttype: u8,
    sll_halen: u8,
    sll_addr: [u8; 8],
}

#[cfg(test)]
mod tests;
