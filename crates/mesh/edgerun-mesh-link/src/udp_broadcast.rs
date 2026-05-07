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
use crate::multicast::SockaddrIn;
use crate::prelude::v1::*;
use edgerun_hardware_signing::NodeID;
use edgerun_mesh::{discovery::DiscoveryPacket, router::MeshRouter, FrameType, MeshFrame};
use std::collections::{HashMap, VecDeque};
use std::io;
use std::os::raw::{c_int, c_void};

// UDP broadcast socket (works without root)
// ---------------------------------------------------------------------------

/// A UDP socket that broadcasts MeshFrames to all peers on the local network.
/// Works without root — the default transport for development.
pub struct UdpBroadcastSocket {
    fd: c_int,
    /// Learned peer IPs from inbound datagrams, keyed by NodeID.
    peer_addrs: HashMap<NodeID, SockaddrIn>,
}

impl UdpBroadcastSocket {
    const DEFAULT_PORT: u16 = EDGERUN_DEV_MESH_BROADCAST_PORT;

    /// Binds a UDP socket for broadcast mesh communication.
    pub fn bind() -> Result<Self, io::Error> {
        Self::bind_port(Self::DEFAULT_PORT)
    }

    pub fn bind_public_mesh() -> Result<Self, io::Error> {
        Self::bind_port(EDGERUN_PUBLIC_MESH_PORT)
    }

    pub fn bind_port(port: u16) -> Result<Self, io::Error> {
        let fd = unsafe { socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }

        // Allow reuse so multiple nodes can run on the same machine (different binds)
        let reuse: c_int = 1;
        unsafe {
            setsockopt(fd, SOL_SOCKET, 2, &reuse as *const _ as *const c_void, 4);
        }

        // Enable broadcast
        let broadcast: c_int = 1;
        unsafe {
            setsockopt(
                fd,
                SOL_SOCKET,
                6,
                &broadcast as *const _ as *const c_void,
                4,
            );
        }

        // Bind to 0.0.0.0:port
        let addr = SockaddrIn {
            sin_family: AF_INET as u16,
            sin_port: port.to_be(),
            sin_addr: 0, // INADDR_ANY
            sin_zero: [0; 8],
        };
        let rc = unsafe {
            bind(
                fd,
                &addr as *const _ as *const c_void,
                std::mem::size_of::<SockaddrIn>() as u32,
            )
        };
        if rc < 0 {
            let err = io::Error::last_os_error();
            unsafe { close(fd) };
            return Err(err);
        }

        // Set non-blocking so pump() can return quickly on host socket builds.
        #[cfg(target_os = "linux")]
        unsafe {
            fcntl(fd, F_SETFL, O_NONBLOCK);
        }

        Ok(Self {
            fd,
            peer_addrs: HashMap::new(),
        })
    }

    /// Receives a datagram if available. Returns (data, sender_addr).
    pub fn recv(&mut self) -> Result<Option<(Vec<u8>, SockaddrIn)>, io::Error> {
        let mut buf = vec![0u8; 8192];
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
        Ok(Some((buf, addr)))
    }

    /// Sends a datagram to the broadcast address.
    pub fn broadcast(&self, data: &[u8]) -> Result<(), io::Error> {
        let dst = SockaddrIn {
            sin_family: AF_INET as u16,
            sin_port: Self::DEFAULT_PORT.to_be(),
            sin_addr: u32::from_be_bytes([255, 255, 255, 255]),
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

    /// Sends a datagram to a specific peer (learned from an inbound frame).
    pub fn send_to(&self, data: &[u8], addr: &SockaddrIn) -> Result<(), io::Error> {
        let n = unsafe {
            sendto(
                self.fd,
                data.as_ptr() as *const c_void,
                data.len(),
                0,
                addr as *const _ as *const c_void,
                std::mem::size_of::<SockaddrIn>() as u32,
            )
        };
        if n < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    /// Looks up a peer's socket address by NodeID.
    pub fn peer_addr(&self, peer_id: &NodeID) -> Option<SockaddrIn> {
        self.peer_addrs.get(peer_id).copied()
    }

    /// Records a peer's address (called after receiving a frame from them).
    pub fn learn_peer(&mut self, peer_id: NodeID, addr: SockaddrIn) {
        self.peer_addrs.insert(peer_id, addr);
    }

    pub fn fd(&self) -> c_int {
        self.fd
    }
}

impl Drop for UdpBroadcastSocket {
    fn drop(&mut self) {
        unsafe { close(self.fd) };
    }
}

// ---------------------------------------------------------------------------
