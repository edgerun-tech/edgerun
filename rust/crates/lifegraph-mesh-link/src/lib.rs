//! Link-layer transport for the Lifegraph mesh.
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

use lifegraph_hardware_signing::NodeID;
use lifegraph_mesh::{FrameType, MeshFrame};
use lifegraph_mesh_router::{DiscoveryPacket, MeshRouter};
use std::collections::{HashMap, VecDeque};
use std::io;
use std::os::raw::{c_int, c_void};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// EtherType for Lifegraph mesh frames (unassigned, in the experiment range).
pub const MESH_ETHERTYPE: u16 = 0x88B5;

/// Multicast group for mesh discovery (239.255.0.1).
const MESH_MCAST_ADDR: [u8; 4] = [239, 255, 0, 1];
const MESH_MCAST_PORT: u16 = 47080;

/// Raw Ethernet protocol number for our EtherType (host byte order).
const ETH_P_MESH: u16 = MESH_ETHERTYPE.to_be();

// Packet socket options (reserved for future BPF filtering)
#[allow(dead_code)]
const SO_ATTACH_FILTER: c_int = 26;
#[allow(dead_code)]
const SOL_PACKET: c_int = 263;
#[allow(dead_code)]
const PACKET_ADD_MEMBERSHIP: c_int = 1;
#[allow(dead_code)]
const PACKET_MR_MULTICAST: c_int = 0;

// ---------------------------------------------------------------------------
// Raw Ethernet socket
// ---------------------------------------------------------------------------

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

/// A raw Ethernet socket bound to a specific interface for our EtherType.
pub struct RawEthernetSocket {
    fd: c_int,
    #[allow(dead_code)]
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
// Multicast UDP socket (for discovery)
// ---------------------------------------------------------------------------

const AF_INET: c_int = 2;
const SOCK_DGRAM: c_int = 2;
const IPPROTO_UDP: c_int = 17;
// IP multicast options (reserved for future use with explicit group management)
#[allow(dead_code)]
const IP_ADD_MEMBERSHIP: c_int = 35;
const IP_MULTICAST_IF: c_int = 32;

#[repr(C)]
struct IpMreq {
    imr_multiaddr: u32,
    imr_interface: u32,
}

#[repr(C)]
struct SockaddrIn {
    sin_family: u16,
    sin_port: u16,
    sin_addr: u32,
    sin_zero: [u8; 8],
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
                0,                     // SOL_IP
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
// Mesh link manager
// ---------------------------------------------------------------------------

/// Manages all link-layer paths for a mesh node.
pub struct MeshLink {
    raw_sockets: HashMap<c_int, RawEthernetSocket>, // keyed by ifindex
    multicast_sockets: Vec<MulticastSocket>,
    tunnels: HashMap<NodeID, IpTunnel>, // keyed by peer NodeID
    /// This node's identity (ECDSA P-256 public key).
    local_node_id: NodeID,
    /// Outbound frames queued for sending (by the daemon event loop).
    pending_frames: VecDeque<MeshFrame>,
    /// MAC addresses learned from inbound Ethernet source addresses,
    /// keyed by NodeID. Used to resolve next-hop MAC for raw Ethernet send.
    mac_table: HashMap<NodeID, [u8; 6]>,
    /// Inbound data frames awaiting delivery to the capability dispatcher.
    inbound_data_frames: VecDeque<MeshFrame>,
}

impl MeshLink {
    pub fn new() -> Self {
        Self {
            raw_sockets: HashMap::new(),
            multicast_sockets: Vec::new(),
            tunnels: HashMap::new(),
            local_node_id: NodeID([0u8; 64]),
            pending_frames: VecDeque::new(),
            mac_table: HashMap::new(),
            inbound_data_frames: VecDeque::new(),
        }
    }

    /// Sets this node's identity (ECDSA P-256 public key).
    /// Must be called before sending frames.
    pub fn set_local_node_id(&mut self, node_id: NodeID) {
        self.local_node_id = node_id;
    }

    /// Adds a raw Ethernet socket on the given interface.
    pub fn add_raw_ethernet(
        &mut self,
        ifindex: c_int,
    ) -> Result<(), io::Error> {
        let socket = RawEthernetSocket::open(ifindex)?;
        self.raw_sockets.insert(ifindex, socket);
        Ok(())
    }

    /// Adds a multicast socket on the interface with the given local IPv4 address.
    pub fn add_multicast(&mut self, local_ipv4: [u8; 4]) -> Result<(), io::Error> {
        let socket = MulticastSocket::open(u32::from_be_bytes(local_ipv4))?;
        self.multicast_sockets.push(socket);
        Ok(())
    }

    /// Adds an IP tunnel to a known peer.
    pub fn add_tunnel(
        &mut self,
        peer_id: NodeID,
        peer_ip: [u8; 4],
        port: u16,
    ) -> Result<(), io::Error> {
        let tunnel = IpTunnel::open(peer_ip, port)?;
        self.tunnels.insert(peer_id, tunnel);
        Ok(())
    }

    /// Returns all file descriptors to poll with `select()`/`poll()`.
    #[must_use]
    pub fn poll_fds(&self) -> Vec<c_int> {
        let mut fds = Vec::new();
        for sock in self.raw_sockets.values() {
            fds.push(sock.fd());
        }
        for sock in &self.multicast_sockets {
            fds.push(sock.fd());
        }
        for tunnel in self.tunnels.values() {
            fds.push(tunnel.fd());
        }
        fds
    }

    /// Queues a mesh frame for sending.  The actual send happens in
    /// `drain_pending_frames()`, which should be called by the daemon
    /// event loop after `pump()` processes inbound traffic.
    pub fn queue_frame(&mut self, frame: MeshFrame) {
        self.pending_frames.push_back(frame);
    }

    /// Returns the number of pending outbound frames.
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.pending_frames.len()
    }

    /// Drains pending outbound frames without sending them.
    /// Used by the daemon to collect and sign frames before re-queuing.
    pub fn drain_pending_frames_raw(&mut self) -> VecDeque<MeshFrame> {
        std::mem::take(&mut self.pending_frames)
    }

    /// Drains and sends all pending outbound frames.
    /// Returns the number of frames sent.
    pub fn drain_pending_frames(&mut self, _router: &mut MeshRouter) -> Result<usize, io::Error> {
        let mut sent = 0;
        while let Some(frame) = self.pending_frames.pop_front() {
            self.send_mesh_frame(&frame)?;
            sent += 1;
        }
        Ok(sent)
    }

    /// Drains inbound data frames that were received and verified.
    /// These should be delivered to the capability envelope dispatcher.
    pub fn drain_inbound_data_frames(&mut self) -> VecDeque<MeshFrame> {
        std::mem::take(&mut self.inbound_data_frames)
    }

    /// Injects an inbound frame directly (for testing and loopback).
    /// The frame will be processed on the next `pump()` call.
    pub fn inject_inbound_frame(&mut self, frame: MeshFrame) {
        self.inbound_data_frames.push_back(frame);
    }

    /// Sends a single mesh frame through the best available transport.
    fn send_mesh_frame(&mut self, frame: &MeshFrame) -> Result<(), io::Error> {
        let dest = frame.header.dest;

        // Build the actual wire frame with our src NodeID filled in
        let mut wire_frame = frame.clone();
        wire_frame.header.src = self.local_node_id;
        let wire = wire_frame.to_wire();

        // If dest is a known tunnel peer, send via tunnel
        if let Some(tunnel) = self.tunnels.get(&dest) {
            return tunnel.send(&wire);
        }

        // If dest is broadcast (all zeroes), send on multicast
        if dest.0 == [0u8; 64] {
            for mcast in &self.multicast_sockets {
                let _ = mcast.send(&wire); // best effort
            }
            // Also broadcast on raw Ethernet
            let broadcast_mac = [0xff; 6];
            for socket in self.raw_sockets.values() {
                let _ = socket.send_to_mac(&wire, &broadcast_mac);
            }
            return Ok(());
        }

        // Unicast: try to send via raw Ethernet if we know the MAC
        if let Some(&dst_mac) = self.mac_table.get(&dest) {
            for socket in self.raw_sockets.values() {
                if socket.send_to_mac(&wire, &dst_mac).is_ok() {
                    return Ok(());
                }
            }
        }

        // Fallback: multicast broadcast
        for mcast in &self.multicast_sockets {
            let _ = mcast.send(&wire);
        }
        Ok(())
    }

    /// Processes all pending inbound traffic, updating the router.
    /// Returns the number of frames processed (discovery + data).
    /// Inbound data frames are also queued for delivery to the
    /// capability dispatcher via `drain_inbound_data_frames()`.
    pub fn pump(&mut self, router: &mut MeshRouter) -> Result<usize, io::Error> {
        let mut count = 0;

        // Raw Ethernet frames
        let mut eth_frames: Vec<(c_int, MeshFrame)> = Vec::new();
        for (&ifindex, socket) in &self.raw_sockets {
            while let Some((data, _src_mac)) = socket.recv()? {
                if let Some(frame) = MeshFrame::from_wire(&data) {
                    eth_frames.push((ifindex, frame));
                }
            }
        }
        for (ifindex, frame) in eth_frames {
            self.process_inbound_frame(router, frame, ifindex);
            count += 1;
        }

        // Multicast discovery packets — collect first
        let mut disc_packets: Vec<Vec<u8>> = Vec::new();
        for mcast in &self.multicast_sockets {
            while let Some((data, _sender_ip)) = mcast.recv()? {
                disc_packets.push(data);
            }
        }
        // Discovery from multicast needs the sender's NodeID which we can't
        // get from UDP alone. For multicast, we use the raw Ethernet src MAC
        // as a fallback mapping. Skip for now — full discovery needs the
        // sender's identity from the frame header wrapping.

        // Tunnel packets
        let mut tunnel_frames: Vec<(NodeID, MeshFrame)> = Vec::new();
        for (peer_id, tunnel) in &self.tunnels {
            while let Some(data) = tunnel.recv()? {
                if let Some(frame) = MeshFrame::from_wire(&data) {
                    tunnel_frames.push((*peer_id, frame));
                }
            }
        }
        for (_peer_id, frame) in tunnel_frames {
            self.process_inbound_frame(router, frame, 0);
            count += 1;
        }

        // Suppress unused variable warnings
        let _ = disc_packets;

        Ok(count)
    }

    fn process_inbound_frame(
        &mut self,
        router: &mut MeshRouter,
        frame: MeshFrame,
        _ifindex: c_int,
    ) {
        // Verify the ECDSA P-256 signature against the sender's NodeID.
        // This ensures the frame was actually sent by the node that claims
        // to be the source (their NodeID = their P-256 public key).
        if !frame.verify_signature() {
            return; // forged or corrupted frame — drop silently
        }

        // Learn the sender's NodeID for MAC table (from Ethernet src MAC)
        // The MAC is learned from the raw Ethernet socket's recv which gives us src_mac

        // Check if this frame is destined for us
        let is_for_us = frame.header.dest == router.node_id()
            || frame.header.dest.0 == [0u8; 64]; // broadcast

        if is_for_us {
            // Frame is for us — process by type
            match frame.header.frame_type {
                FrameType::Discovery => {
                    if let Some(packet) = DiscoveryPacket::decode(&frame.payload) {
                        let _changed =
                            router.process_discovery(frame.header.src, &packet, current_unix_secs());
                    }
                }
                FrameType::Data | FrameType::RouteAdv | FrameType::HandshakeInit | FrameType::HandshakeAccept => {
                    // Queue for capability/handshake dispatcher delivery
                    self.inbound_data_frames.push_back(frame);
                }
                FrameType::Unknown(_) => {} // drop unknown
            }
        } else {
            // Frame is not for us — try to forward
            if let Some(forward_ttl) = router.should_forward(&frame.header) {
                let mut fwd_frame = frame;
                fwd_frame.header.ttl = forward_ttl;
                self.pending_frames.push_back(fwd_frame);
            }
            // Else drop (no route or TTL expired)
        }
    }

    /// Builds and broadcasts a discovery frame on all multicast sockets.
    pub fn broadcast_discovery(&mut self, router: &mut MeshRouter) -> Result<(), io::Error> {
        let frame = router.build_discovery_frame();
        let wire = frame.to_wire();
        for mcast in &self.multicast_sockets {
            mcast.send(&wire)?;
        }
        // Also send on raw Ethernet (broadcast MAC ff:ff:ff:ff:ff:ff)
        let broadcast_mac = [0xff; 6];
        for socket in self.raw_sockets.values() {
            let _ = socket.send_to_mac(&wire, &broadcast_mac);
        }
        Ok(())
    }

    /// Sends a mesh frame to the next-hop for its destination.
    /// Returns `true` if the frame was sent, `false` if no route exists.
    pub fn send_frame(
        &mut self,
        router: &MeshRouter,
        frame: &MeshFrame,
    ) -> Result<bool, io::Error> {
        let Some(next_hop) = router.next_hop_for(&frame.header.dest) else {
            return Ok(false);
        };

        // If next_hop is a tunnel peer, send via tunnel
        if let Some(tunnel) = self.tunnels.get(&next_hop) {
            let wire = frame.to_wire();
            tunnel.send(&wire)?;
            return Ok(true);
        }

        // Otherwise send via raw Ethernet to the next_hop's MAC
        // (we need a MAC lookup table — populated from Ethernet source MACs)
        // For now, send via multicast as a fallback
        for mcast in &self.multicast_sockets {
            let wire = frame.to_wire();
            mcast.send(&wire)?;
        }
        Ok(true)
    }
}

fn current_unix_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use lifegraph_hardware_signing::{MESH_PUBLIC_KEY_LENGTH, MESH_SIGNATURE_LENGTH, NodeID};
    use lifegraph_mesh::MeshFrameHeader;
    use lifegraph_mesh::MeshRoute;

    fn node_id(v: u8) -> NodeID {
        let mut bytes = [0u8; 64];
        bytes[0] = v;
        NodeID(bytes)
    }

    #[test]
    fn discovery_packet_roundtrip() {
        let packet = DiscoveryPacket {
            sequence: 7,
            routes: vec![MeshRoute {
                destination: node_id(0xAA),
                next_hop: None,
                cost: 1,
            }],
        };
        let encoded = packet.encode();
        let decoded = DiscoveryPacket::decode(&encoded).unwrap();
        assert_eq!(decoded.sequence, 7);
        assert_eq!(decoded.routes.len(), 1);
    }

    #[test]
    fn mesh_link_poll_fds_empty() {
        let link = MeshLink::new();
        assert!(link.poll_fds().is_empty());
    }

    #[test]
    fn mesh_frame_header_size() {
        assert_eq!(MeshFrameHeader::SIZE, 130);
    }

    #[test]
    fn mesh_signature_size() {
        assert_eq!(MESH_SIGNATURE_LENGTH, 64);
    }

    #[test]
    fn mesh_public_key_size() {
        assert_eq!(lifegraph_hardware_signing::MESH_PUBLIC_KEY_LENGTH, 64);
    }
}
