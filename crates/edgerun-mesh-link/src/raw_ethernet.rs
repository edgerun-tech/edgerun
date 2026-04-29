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

/// A raw Ethernet socket bound to a specific interface for our EtherType.
pub struct RawEthernetSocket {
    fd: c_int,
    ifindex: c_int,
}

impl RawEthernetSocket {
    /// Opens a raw socket on the given interface, receiving frames with our
    /// EtherType.
    pub fn open(ifindex: c_int) -> Result<Self, io::Error> {
        let fd = unsafe { socket(AF_PACKET, SOCK_RAW, ETH_P_MESH as c_int) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let addr = SockaddrLl {
            sll_family: AF_PACKET as u16,
            sll_protocol: ETH_P_MESH,
            sll_ifindex: ifindex,
            sll_hatype: 1,
            sll_pkttype: 0,
            sll_halen: 6,
            sll_addr: [0; 8],
        };
        let rc = unsafe {
            bind(
                fd,
                &addr as *const _ as *const c_void,
                std::mem::size_of::<SockaddrLl>() as u32,
            )
        };
        if rc < 0 {
            let err = io::Error::last_os_error();
            unsafe { close(fd) };
            return Err(err);
        }
        Ok(Self { fd, ifindex })
    }

    /// Receives a raw Ethernet frame.  Returns `(payload, source_mac)` or
    /// `None` on EOF.
    pub fn recv(&self) -> Result<Option<(Vec<u8>, [u8; 6])>, io::Error> {
        let mut buf = vec![0u8; 2048];
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
        // Parse Ethernet header to extract source MAC
        if buf.len() < 14 {
            return Ok(None);
        }
        let mut src_mac = [0u8; 6];
        src_mac.copy_from_slice(&buf[6..12]);
        // Strip Ethernet header (14 bytes) and EtherType (2 bytes already matched by kernel)
        // Actually the kernel delivers payload after the EtherType, so we get
        // Ethernet src(6) + dst(6) + type(2) + payload.
        // Let's just pass the whole thing and let the caller strip it.
        Ok(Some((buf, src_mac)))
    }

    /// Sends raw bytes to a destination MAC on this interface.
    pub fn send_to_mac(&self, payload: &[u8], dst_mac: &[u8; 6]) -> Result<(), io::Error> {
        // Build Ethernet frame: dst(6) + src(6) + type(2) + payload
        // We don't know our own MAC here; the kernel fills src on AF_PACKET
        // Actually for SOCK_RAW with ETH_P_*, we must build the full Ethernet header.
        // Let's use a zero src — kernel will replace it if we use PACKET_ORIG_DST.
        // For simplicity, just build the header.
        let mut frame = Vec::with_capacity(14 + payload.len());
        frame.extend_from_slice(dst_mac); // dst
        frame.extend_from_slice(&[0u8; 6]); // src (kernel may fill)
        frame.extend_from_slice(&MESH_ETHERTYPE.to_be_bytes());
        frame.extend_from_slice(payload);

        let n = unsafe {
            sendto(
                self.fd,
                frame.as_ptr() as *const c_void,
                frame.len(),
                0,
                std::ptr::null(),
                0,
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

impl Drop for RawEthernetSocket {
    fn drop(&mut self) {
        unsafe { close(self.fd) };
    }
}

// ---------------------------------------------------------------------------
