#![allow(dead_code)]
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

use edgerun_hardware_signing::NodeID;
use edgerun_mesh::{DiscoveryPacket, FrameType, MeshFrame, MeshRouter};
use std::collections::{HashMap, VecDeque};
use std::io;
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

mod multicast;
mod tunnel;
mod udp_broadcast;
mod link_manager;

pub use raw_ethernet::RawEthernetSocket;
pub use multicast::MulticastSocket;
pub use tunnel::IpTunnel;
pub use udp_broadcast::UdpBroadcastSocket;
pub use link_manager::MeshLink;

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
