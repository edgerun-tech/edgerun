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

use super::*;
use crate::prelude::v1::*;
use edgerun_hardware_signing::NodeID;
use edgerun_mesh::{discovery::DiscoveryPacket, router::MeshRouter, FrameType, MeshFrame};
use std::collections::{HashMap, VecDeque};
use std::io;
use std::os::raw::{c_int, c_void};

// Multicast UDP socket (for discovery)
// ---------------------------------------------------------------------------

const AF_INET: c_int = 2;
const SOCK_DGRAM: c_int = 2;
const IPPROTO_UDP: c_int = 17;
// IP multicast options (reserved for future use with explicit group management)
const IP_ADD_MEMBERSHIP: c_int = 35;
const IP_MULTICAST_IF: c_int = 32;

#[repr(C)]
pub(crate) struct IpMreq {
    imr_multiaddr: u32,
    imr_interface: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SockaddrIn {
    pub sin_family: u16,
    pub sin_port: u16,
    pub sin_addr: u32,
    pub sin_zero: [u8; 8],
}

/// A UDP socket joined to the mesh multicast group on a specific interface.
pub struct MulticastSocket {
    fd: c_int,
}

impl MulticastSocket {
    /// Creates a multicast socket on the given interface (by local IP address).
    pub fn open(local_ipv4: u32) -> Result<Self, io::Error> {
        let fd = unsafe { socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let reuse: c_int = 1;
        unsafe {
            setsockopt(
                fd,
                SOL_SOCKET,
                2, // SO_REUSEADDR
                &reuse as *const _ as *const c_void,
                4,
            );
        }

        // Bind to the multicast port on INADDR_ANY
        let bind_addr = SockaddrIn {
            sin_family: AF_INET as u16,
            sin_port: MESH_MCAST_PORT.to_be(),
            sin_addr: 0, // INADDR_ANY
            sin_zero: [0; 8],
        };
        let rc = unsafe {
            bind(
                fd,
                &bind_addr as *const _ as *const c_void,
                std::mem::size_of::<SockaddrIn>() as u32,
            )
        };
        if rc < 0 {
            let err = io::Error::last_os_error();
            unsafe { close(fd) };
            return Err(err);
        }

        // Join multicast group
        let mreq = IpMreq {
            imr_multiaddr: u32::from_be_bytes(MESH_MCAST_ADDR),
            imr_interface: local_ipv4,
        };
        let rc = unsafe {
            setsockopt(
                fd,
                IPPROTO_UDP as c_int, // Actually this is SOL_IP = 0
                0,                    // SOL_IP
                &mreq as *const _ as *const c_void,
                std::mem::size_of::<IpMreq>() as u32,
            )
        };
        if rc < 0 {
            let err = io::Error::last_os_error();
            unsafe { close(fd) };
            return Err(err);
        }

        // Set multicast interface
        let rc = unsafe {
            setsockopt(
                fd,
                0, // SOL_IP
                IP_MULTICAST_IF,
                &local_ipv4 as *const _ as *const c_void,
                4,
            )
        };
        if rc < 0 {
            let err = io::Error::last_os_error();
            unsafe { close(fd) };
            return Err(err);
        }

        Ok(Self { fd })
    }

    /// Receives a multicast packet. Returns `(payload, sender_ipv4)` or `None`.
    pub fn recv(&self) -> Result<Option<(Vec<u8>, [u8; 4])>, io::Error> {
        let mut buf = vec![0u8; 4096];
        let mut addr = SockaddrIn {
            sin_family: 0,
            sin_port: 0,
            sin_addr: 0,
            sin_zero: [0; 8],
        };
        let mut addrlen = std::mem::size_of::<SockaddrIn>() as u32;
        let n = unsafe {
            recvfrom(
                self.fd,
                buf.as_mut_ptr() as *mut c_void,
                buf.len(),
                0,
                &mut addr as *mut _ as *mut c_void,
                &mut addrlen,
            )
        };
        if n < 0 {
            let err = io::Error::last_os_error();
            if err.kind() == io::ErrorKind::WouldBlock {
                return Ok(None);
            }
            return Err(err);
        }
        buf.truncate(n as usize);
        let sender = addr.sin_addr.to_be_bytes();
        Ok(Some((buf, sender)))
    }

    /// Sends data to the multicast group.
    pub fn send(&self, data: &[u8]) -> Result<(), io::Error> {
        let dst = SockaddrIn {
            sin_family: AF_INET as u16,
            sin_port: MESH_MCAST_PORT.to_be(),
            sin_addr: u32::from_be_bytes(MESH_MCAST_ADDR),
            sin_zero: [0; 8],
        };
        let n = unsafe {
            sendto(
                self.fd,
                data.as_ptr() as *const c_void,
                data.len(),
                0,
                &dst as *const _ as *const c_void,
                std::mem::size_of::<SockaddrIn>() as u32,
            )
        };
        if n < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    pub fn fd(&self) -> c_int {
        self.fd
    }
}

impl Drop for MulticastSocket {
    fn drop(&mut self) {
        unsafe { close(self.fd) };
    }
}

// ---------------------------------------------------------------------------
