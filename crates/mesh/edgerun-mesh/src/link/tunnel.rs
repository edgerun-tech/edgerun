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

use super::multicast::SockaddrIn;
use super::prelude::v1::*;
use super::*;
use crate::{FrameType, MeshFrame, discovery::DiscoveryPacket, router::MeshRouter};
use edgerun_hardware_signing::NodeID;
use std::collections::{HashMap, VecDeque};
use std::io;
use std::os::raw::{c_int, c_void};

// IP tunnel (UDP point-to-point pipe for cross-subnet peers)
// ---------------------------------------------------------------------------

/// A point-to-point UDP tunnel to a known peer IP address.
pub struct IpTunnel {
    fd: c_int,
    peer_addr: SockaddrIn,
}

impl IpTunnel {
    /// Opens a UDP socket to the given peer IP and port.
    pub fn open(peer_ip: [u8; 4], port: u16) -> Result<Self, io::Error> {
        let fd = unsafe { socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let peer_addr = SockaddrIn {
            sin_family: AF_INET as u16,
            sin_port: port.to_be(),
            sin_addr: u32::from_be_bytes(peer_ip),
            sin_zero: [0; 8],
        };
        Ok(Self { fd, peer_addr })
    }

    pub fn open_public_mesh(peer_ip: [u8; 4]) -> Result<Self, io::Error> {
        Self::open(peer_ip, EDGERUN_PUBLIC_MESH_PORT)
    }

    pub fn send(&self, data: &[u8]) -> Result<(), io::Error> {
        let n = unsafe {
            sendto(
                self.fd,
                data.as_ptr() as *const c_void,
                data.len(),
                0,
                &self.peer_addr as *const _ as *const c_void,
                std::mem::size_of::<SockaddrIn>() as u32,
            )
        };
        if n < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    pub fn recv(&self) -> Result<Option<Vec<u8>>, io::Error> {
        let mut buf = vec![0u8; 4096];
        let n = unsafe {
            recvfrom(
                self.fd,
                buf.as_mut_ptr() as *mut c_void,
                buf.len(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
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
        Ok(Some(buf))
    }

    pub fn fd(&self) -> c_int {
        self.fd
    }
}

impl Drop for IpTunnel {
    fn drop(&mut self) {
        unsafe { close(self.fd) };
    }
}

// ---------------------------------------------------------------------------
