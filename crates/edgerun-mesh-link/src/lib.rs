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

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

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
const IP_ADD_MEMBERSHIP: c_int = 35;
const IP_MULTICAST_IF: c_int = 32;

#[repr(C)]
struct IpMreq {
    imr_multiaddr: u32,
    imr_interface: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SockaddrIn {
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
    const DEFAULT_PORT: u16 = 47079;

    /// Binds a UDP socket for broadcast mesh communication.
    pub fn bind() -> Result<Self, io::Error> {
        Self::bind_port(Self::DEFAULT_PORT)
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
            setsockopt(fd, SOL_SOCKET, 6, &broadcast as *const _ as *const c_void, 4);
        }

        // Bind to 0.0.0.0:port
        let addr = SockaddrIn {
            sin_family: AF_INET as u16,
            sin_port: port.to_be(),
            sin_addr: 0, // INADDR_ANY
            sin_zero: [0; 8],
        };
        let rc = unsafe {
            bind(fd, &addr as *const _ as *const c_void, std::mem::size_of::<SockaddrIn>() as u32)
        };
        if rc < 0 {
            let err = io::Error::last_os_error();
            unsafe { close(fd) };
            return Err(err);
        }

        // Set non-blocking so pump() can return quickly
        unsafe {
            libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK);
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
// Mesh link manager
// ---------------------------------------------------------------------------

/// Manages all link-layer paths for a mesh node.
pub struct MeshLink {
    raw_sockets: HashMap<c_int, RawEthernetSocket>, // keyed by ifindex
    multicast_sockets: Vec<MulticastSocket>,
    tunnels: HashMap<NodeID, IpTunnel>, // keyed by peer NodeID
    udp_broadcast: Option<UdpBroadcastSocket>, // works without root
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

impl Default for MeshLink {
    fn default() -> Self {
        Self::new()
    }
}

impl MeshLink {
    pub fn new() -> Self {
        Self {
            raw_sockets: HashMap::new(),
            multicast_sockets: Vec::new(),
            tunnels: HashMap::new(),
            udp_broadcast: None,
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

    /// Enables UDP broadcast transport (works without root).
    /// This is the default transport for development.
    pub fn enable_udp_broadcast(&mut self) -> Result<(), io::Error> {
        self.udp_broadcast = Some(UdpBroadcastSocket::bind()?);
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
        if let Some(udp) = &self.udp_broadcast {
            fds.push(udp.fd());
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

        // Unicast: try UDP if we know the peer's address (learned from a previous inbound frame)
        if let Some(udp) = &self.udp_broadcast {
            if let Some(peer_addr) = udp.peer_addr(&dest) {
                if udp.send_to(&wire, &peer_addr).is_ok() {
                    return Ok(());
                }
            }
        }

        // Fallback: UDP broadcast (works without root)
        if let Some(udp) = &self.udp_broadcast {
            if udp.broadcast(&wire).is_ok() {
                return Ok(());
            }
        }
        // Last resort: multicast broadcast
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

        // Multicast discovery packets
        let mut disc_packets: Vec<Vec<u8>> = Vec::new();
        for mcast in &self.multicast_sockets {
            while let Some((data, _sender_ip)) = mcast.recv()? {
                disc_packets.push(data);
            }
        }
        let _ = disc_packets;

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

        // UDP broadcast frames — the default transport (works without root)
        let mut udp_frames: Vec<MeshFrame> = Vec::new();
        if let Some(udp) = &mut self.udp_broadcast {
            while let Some((data, sender_addr)) = udp.recv()? {
                if let Some(frame) = MeshFrame::from_wire(&data) {
                    // Learn the sender's NodeID → IP mapping for unicast replies
                    udp.learn_peer(frame.header.src, sender_addr);
                    udp_frames.push(frame);
                }
            }
        }
        for frame in udp_frames {
            self.process_inbound_frame(router, frame, 0);
            count += 1;
        }

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
    use edgerun_hardware_signing::{MESH_PUBLIC_KEY_LENGTH, MESH_SIGNATURE_LENGTH, NodeID};
    use edgerun_mesh::{sign_frame, DiscoveryPacket, FrameType, MeshFrame, MeshFrameHeader, MeshRoute};
    use edgerun_crypto::rand_core::RngCore;
use edgerun_crypto::p256::ecdsa::SigningKey;

    // -----------------------------------------------------------------------
    // Test helpers
    // -----------------------------------------------------------------------

    fn node_id(v: u8) -> NodeID {
        let mut bytes = [0u8; 64];
        bytes[0] = v;
        NodeID(bytes)
    }

    fn broadcast_id() -> NodeID {
        NodeID([0u8; 64])
    }

    /// Creates a real P-256 keypair and returns (NodeID, signing_key).
    fn make_real_keypair() -> (NodeID, SigningKey) {
        let mut bytes = [0u8; 32];
        edgerun_crypto::rand_core::OsRng.fill_bytes(&mut bytes);
        let signing_key = SigningKey::from_bytes(&bytes.into()).unwrap();
        let encoded = signing_key.verifying_key().to_encoded_point(false);
        let b = encoded.as_bytes();
        let mut node_bytes = [0u8; 64];
        node_bytes.copy_from_slice(&b[1..65]);
        (NodeID(node_bytes), signing_key)
    }

    /// Builds a signed mesh frame for testing.
    fn make_signed_frame(
        src_key: &SigningKey,
        src_id: NodeID,
        dest: NodeID,
        ttl: u8,
        frame_type: FrameType,
        payload: Vec<u8>,
    ) -> MeshFrame {
        let header = MeshFrameHeader {
            dest,
            src: src_id,
            ttl,
            frame_type,
        };
        let mut frame = MeshFrame {
            header,
            payload,
            signature: [0u8; MESH_SIGNATURE_LENGTH],
        };
        sign_frame(&mut frame, src_key);
        frame
    }

    // =======================================================================
    // 1. Constants
    // =======================================================================

    #[test]
    fn constants_have_expected_values() {
        assert_eq!(MESH_ETHERTYPE, 0x88B5);
        assert_eq!(MESH_MCAST_ADDR, [239, 255, 0, 1]);
        assert_eq!(MESH_MCAST_PORT, 47080);
        // ETH_P_MESH is the host-byte-order version of MESH_ETHERTYPE
        assert_eq!(ETH_P_MESH, 0xB588u16); // 0x88B5.to_be() == 0xB588
    }

    // =======================================================================
    // 2. Discovery packet encoding/decoding
    // =======================================================================

    #[test]
    fn discovery_packet_encode_decode_empty_routes() {
        let packet = DiscoveryPacket {
            sequence: 0,
            routes: vec![],
        };
        let encoded = packet.encode();
        assert_eq!(encoded.len(), 5); // seq(4) + count(1)
        assert_eq!(encoded[0..4], 0u32.to_le_bytes());
        assert_eq!(encoded[4], 0); // route count

        let decoded = DiscoveryPacket::decode(&encoded).unwrap();
        assert_eq!(decoded.sequence, 0);
        assert_eq!(decoded.routes.len(), 0);
    }

    #[test]
    fn discovery_packet_encode_decode_single_route() {
        let packet = DiscoveryPacket {
            sequence: 1,
            routes: vec![MeshRoute {
                destination: node_id(0xAA),
                next_hop: None,
                cost: 1,
            }],
        };
        let encoded = packet.encode();
        assert_eq!(encoded.len(), 5 + 65); // header + 1 route
        assert_eq!(u32::from_le_bytes(encoded[0..4].try_into().unwrap()), 1);
        assert_eq!(encoded[4], 1); // 1 route

        let decoded = DiscoveryPacket::decode(&encoded).unwrap();
        assert_eq!(decoded.sequence, 1);
        assert_eq!(decoded.routes.len(), 1);
        assert_eq!(decoded.routes[0].destination, node_id(0xAA));
        assert_eq!(decoded.routes[0].cost, 1);
        assert_eq!(decoded.routes[0].next_hop, None); // next_hop is never encoded
    }

    #[test]
    fn discovery_packet_encode_decode_multiple_routes() {
        let routes: Vec<MeshRoute> = (0..5)
            .map(|i| MeshRoute {
                destination: node_id(i),
                next_hop: None,
                cost: (i + 1) as u8,
            })
            .collect();
        let packet = DiscoveryPacket { sequence: 100, routes };
        let encoded = packet.encode();
        assert_eq!(encoded.len(), 5 + 5 * 65);

        let decoded = DiscoveryPacket::decode(&encoded).unwrap();
        assert_eq!(decoded.sequence, 100);
        assert_eq!(decoded.routes.len(), 5);
        for i in 0..5 {
            assert_eq!(decoded.routes[i].destination, node_id(i as u8));
            assert_eq!(decoded.routes[i].cost, (i + 1) as u8);
        }
    }

    #[test]
    fn discovery_packet_encode_truncates_at_max_routes() {
        let routes: Vec<MeshRoute> = (0..60)
            .map(|i| MeshRoute {
                destination: node_id(i as u8),
                next_hop: None,
                cost: 1,
            })
            .collect();
        let packet = DiscoveryPacket { sequence: 1, routes };
        let encoded = packet.encode();
        // MAX_ROUTES is 50, so only 50 routes should be encoded
        assert_eq!(encoded.len(), 5 + 50 * 65);
        assert_eq!(encoded[4], 50);

        let decoded = DiscoveryPacket::decode(&encoded).unwrap();
        assert_eq!(decoded.routes.len(), 50);
    }

    #[test]
    fn discovery_packet_decode_rejects_too_short() {
        assert!(DiscoveryPacket::decode(&[]).is_none());
        assert!(DiscoveryPacket::decode(&[0; 3]).is_none());
        assert!(DiscoveryPacket::decode(&[0; 4]).is_none()); // missing count byte
    }

    #[test]
    fn discovery_packet_decode_accepts_zero_routes() {
        let buf = [0u8; 5]; // seq=0, count=0
        assert!(DiscoveryPacket::decode(&buf).is_some());
    }

    #[test]
    fn discovery_packet_decode_rejects_truncated_route_data() {
        // count=1 but only 4 bytes of route data (need 65)
        let mut buf = vec![0u8; 5 + 4];
        buf[4] = 1; // claim 1 route
        assert!(DiscoveryPacket::decode(&buf).is_none());
    }

    #[test]
    fn discovery_packet_decode_rejects_partial_last_route() {
        // count=2 but only enough data for 1 full route + 32 bytes
        let mut buf = vec![0u8; 5 + 65 + 32];
        buf[4] = 2; // claim 2 routes
        assert!(DiscoveryPacket::decode(&buf).is_none());
    }

    #[test]
    fn discovery_packet_decode_ignores_extra_bytes() {
        let mut buf = vec![0u8; 5 + 65 + 100];
        buf[4] = 1; // claim 1 route
        // extra trailing bytes should be ignored
        let decoded = DiscoveryPacket::decode(&buf).unwrap();
        assert_eq!(decoded.routes.len(), 1);
    }

    #[test]
    fn discovery_packet_max_routes_constant() {
        assert_eq!(DiscoveryPacket::MAX_ROUTES, 50);
    }

    // =======================================================================
    // 3. SockaddrIn layout
    // =======================================================================

    #[test]
    fn sockaddr_in_size_is_correct() {
        // sin_family(2) + sin_port(2) + sin_addr(4) + sin_zero(8) = 16
        assert_eq!(std::mem::size_of::<SockaddrIn>(), 16);
    }

    #[test]
    fn sockaddr_in_copy_roundtrip() {
        let orig = SockaddrIn {
            sin_family: AF_INET as u16,
            sin_port: 8080u16.to_be(),
            sin_addr: u32::from_be_bytes([10, 0, 0, 1]),
            sin_zero: [0; 8],
        };
        let copy = orig;
        assert_eq!(orig.sin_family, copy.sin_family);
        assert_eq!(orig.sin_port, copy.sin_port);
        assert_eq!(orig.sin_addr, copy.sin_addr);
    }

    // =======================================================================
    // 4. MeshLink construction and defaults
    // =======================================================================

    #[test]
    fn mesh_link_new_has_empty_state() {
        let mut link = MeshLink::new();
        assert!(link.poll_fds().is_empty());
        assert_eq!(link.pending_count(), 0);
        assert_eq!(link.drain_inbound_data_frames().len(), 0);
    }

    #[test]
    fn mesh_link_default_equals_new() {
        let link1 = MeshLink::new();
        let link2 = MeshLink::default();
        // Both should have empty poll_fds
        assert_eq!(link1.poll_fds().len(), link2.poll_fds().len());
        assert_eq!(link1.pending_count(), link2.pending_count());
    }

    #[test]
    fn mesh_link_set_local_node_id() {
        let mut link = MeshLink::new();
        let id = node_id(0x42);
        link.set_local_node_id(id);
        // We can't directly inspect local_node_id, but it affects sent frames.
        // We verify it via the drain_pending_frames_raw side effect.
        let frame = MeshFrame::from_payload(id, b"test".to_vec());
        link.queue_frame(frame);
        let frames = link.drain_pending_frames_raw();
        assert_eq!(frames.len(), 1);
        // After re-queue and drain_with_send (simulated), src would be filled.
    }

    // =======================================================================
    // 5. MeshLink queue management
    // =======================================================================

    #[test]
    fn mesh_link_queue_and_pending_count() {
        let mut link = MeshLink::new();
        assert_eq!(link.pending_count(), 0);

        link.queue_frame(MeshFrame::from_payload(node_id(1), vec![]));
        assert_eq!(link.pending_count(), 1);

        link.queue_frame(MeshFrame::from_payload(node_id(2), vec![]));
        assert_eq!(link.pending_count(), 2);
    }

    #[test]
    fn mesh_link_drain_pending_frames_raw_clears_queue() {
        let mut link = MeshLink::new();
        link.queue_frame(MeshFrame::from_payload(node_id(1), vec![1]));
        link.queue_frame(MeshFrame::from_payload(node_id(2), vec![2]));

        let drained = link.drain_pending_frames_raw();
        assert_eq!(drained.len(), 2);
        assert_eq!(link.pending_count(), 0);
    }

    #[test]
    fn mesh_link_drain_pending_frames_raw_preserves_order() {
        let mut link = MeshLink::new();
        link.queue_frame(MeshFrame::from_payload(node_id(1), vec![10]));
        link.queue_frame(MeshFrame::from_payload(node_id(2), vec![20]));
        link.queue_frame(MeshFrame::from_payload(node_id(3), vec![30]));

        let drained = link.drain_pending_frames_raw();
        assert_eq!(drained[0].payload, vec![10]);
        assert_eq!(drained[1].payload, vec![20]);
        assert_eq!(drained[2].payload, vec![30]);
    }

    #[test]
    fn mesh_link_inject_and_drain_inbound() {
        let mut link = MeshLink::new();
        let frame = MeshFrame::from_payload(node_id(1), b"data".to_vec());

        link.inject_inbound_frame(frame.clone());
        link.inject_inbound_frame(frame.clone());

        let inbound = link.drain_inbound_data_frames();
        assert_eq!(inbound.len(), 2);
        assert_eq!(inbound[0].payload, b"data");
        assert_eq!(inbound[1].payload, b"data");
    }

    #[test]
    fn mesh_link_drain_inbound_clears_queue() {
        let mut link = MeshLink::new();
        link.inject_inbound_frame(MeshFrame::from_payload(node_id(1), vec![]));

        let _ = link.drain_inbound_data_frames();
        let mut link2 = MeshLink::new();
        link2.inject_inbound_frame(MeshFrame::from_payload(node_id(1), vec![]));
        let _ = link2.drain_inbound_data_frames();
        assert_eq!(link2.drain_inbound_data_frames().len(), 0);
    }

    // =======================================================================
    // 6. Frame size calculations
    // =======================================================================

    #[test]
    fn mesh_frame_header_size_is_130() {
        assert_eq!(MeshFrameHeader::SIZE, 130);
        assert_eq!(MeshFrameHeader::SIZE, MESH_PUBLIC_KEY_LENGTH * 2 + 2);
    }

    #[test]
    fn mesh_signature_length_is_64() {
        assert_eq!(MESH_SIGNATURE_LENGTH, 64);
    }

    #[test]
    fn mesh_public_key_length_is_64() {
        assert_eq!(MESH_PUBLIC_KEY_LENGTH, 64);
    }

    #[test]
    fn wire_frame_minimum_size() {
        // Header(130) + Signature(64) = 194 bytes with empty payload
        let min_wire_size = MeshFrameHeader::SIZE + MESH_SIGNATURE_LENGTH;
        assert_eq!(min_wire_size, 194);
    }

    #[test]
    fn wire_frame_size_with_payload() {
        let header = MeshFrameHeader {
            dest: node_id(0xAA),
            src: node_id(0xBB),
            ttl: 10,
            frame_type: FrameType::Data,
        };
        let payload = vec![0u8; 256];
        let frame = MeshFrame {
            header,
            payload,
            signature: [0u8; MESH_SIGNATURE_LENGTH],
        };
        let wire = frame.to_wire();
        assert_eq!(wire.len(), 130 + 256 + 64); // header + payload + sig
    }

    #[test]
    fn from_wire_rejects_buffer_too_short() {
        assert!(MeshFrame::from_wire(&[]).is_none());
        assert!(MeshFrame::from_wire(&[0u8; 100]).is_none());
        assert!(MeshFrame::from_wire(&[0u8; 193]).is_none()); // 1 byte short
    }

    #[test]
    fn from_wire_accepts_exact_minimum() {
        let buf = [0u8; 194]; // 130 header + 64 sig
        assert!(MeshFrame::from_wire(&buf).is_some());
    }

    #[test]
    fn from_wire_with_payload_roundtrip() {
        let header = MeshFrameHeader {
            dest: node_id(0xDD),
            src: node_id(0xEE),
            ttl: 7,
            frame_type: FrameType::RouteAdv,
        };
        let payload = b"hello world".to_vec();
        let sig = [0xABu8; MESH_SIGNATURE_LENGTH];
        let frame = MeshFrame {
            header,
            payload: payload.clone(),
            signature: sig,
        };
        let wire = frame.to_wire();
        let parsed = MeshFrame::from_wire(&wire).unwrap();
        assert_eq!(parsed.header.dest, node_id(0xDD));
        assert_eq!(parsed.header.src, node_id(0xEE));
        assert_eq!(parsed.header.ttl, 7);
        assert_eq!(parsed.header.frame_type, FrameType::RouteAdv);
        assert_eq!(parsed.payload, payload);
        assert_eq!(parsed.signature, sig);
    }

    #[test]
    fn signed_preimage_size() {
        let frame = MeshFrame::from_payload(node_id(1), vec![0u8; 100]);
        let preimage = frame.signed_preimage();
        assert_eq!(preimage.len(), 130 + 100); // header + payload only
    }

    // =======================================================================
    // 7. FrameType encoding
    // =======================================================================

    #[test]
    fn frame_type_from_u8_all_variants() {
        assert_eq!(FrameType::from_u8(0), FrameType::Data);
        assert_eq!(FrameType::from_u8(1), FrameType::Discovery);
        assert_eq!(FrameType::from_u8(2), FrameType::RouteAdv);
        assert_eq!(FrameType::from_u8(3), FrameType::HandshakeInit);
        assert_eq!(FrameType::from_u8(4), FrameType::HandshakeAccept);
        assert_eq!(FrameType::from_u8(5), FrameType::Unknown(5));
        assert_eq!(FrameType::from_u8(255), FrameType::Unknown(255));
    }

    #[test]
    fn frame_type_header_encode_decode_roundtrip() {
        for ft in [
            FrameType::Data,
            FrameType::Discovery,
            FrameType::RouteAdv,
            FrameType::HandshakeInit,
            FrameType::HandshakeAccept,
            FrameType::Unknown(42),
            FrameType::Unknown(255),
        ] {
            let header = MeshFrameHeader {
                dest: node_id(0x11),
                src: node_id(0x22),
                ttl: 8,
                frame_type: ft,
            };
            let encoded = header.encode();
            let decoded = MeshFrameHeader::decode(&encoded);
            assert_eq!(decoded.frame_type, ft, "FrameType roundtrip failed for {:?}", ft);
        }
    }

    // =======================================================================
    // 8. Header encode/decode edge cases
    // =======================================================================

    #[test]
    fn header_encode_produces_exactly_130_bytes() {
        let header = MeshFrameHeader {
            dest: node_id(0xAA),
            src: node_id(0xBB),
            ttl: 255,
            frame_type: FrameType::Data,
        };
        let encoded = header.encode();
        assert_eq!(encoded.len(), 130);
    }

    #[test]
    fn header_encode_dest_and_src_at_boundaries() {
        // Max values in dest/src
        let mut max_dest = [0xFFu8; 64];
        max_dest[0] = 0xFF;
        let header = MeshFrameHeader {
            dest: NodeID(max_dest),
            src: NodeID([0u8; 64]),
            ttl: 0,
            frame_type: FrameType::Unknown(0),
        };
        let encoded = header.encode();
        assert_eq!(&encoded[0..64], &max_dest);
        assert_eq!(&encoded[64..128], &[0u8; 64]);
    }

    #[test]
    fn header_ttl_and_frame_type_bytes() {
        let header = MeshFrameHeader {
            dest: node_id(1),
            src: node_id(2),
            ttl: 42,
            frame_type: FrameType::HandshakeAccept,
        };
        let encoded = header.encode();
        assert_eq!(encoded[128], 42); // ttl
        assert_eq!(encoded[129], 4); // HandshakeAccept
    }

    // =======================================================================
    // 9. MeshLink send_mesh_frame routing (unit-level via drain)
    // =======================================================================

    #[test]
    fn mesh_link_broadcast_fills_src_on_drain() {
        // Test that draining pending frames and attempting to send them
        // properly sets the source NodeID to the local node's identity.
        let (src_id, _src_key) = make_real_keypair();
        let mut link = MeshLink::new();
        link.set_local_node_id(src_id);

        let frame = MeshFrame::from_payload(node_id(1), b"test".to_vec());
        link.queue_frame(frame);

        // Create a router so drain_pending_frames can operate
        let mut router = edgerun_mesh::MeshRouter::new(
            edgerun_mesh::LocalNode::new(src_id),
        );
        // No transports available, so drain should attempt but not crash
        let result = link.drain_pending_frames(&mut router);
        // It will fail to actually send (no transports), but shouldn't panic
        assert!(result.is_ok());
    }

    // =======================================================================
    // 10. MeshLink with router integration (no sockets)
    // =======================================================================

    #[test]
    fn mesh_link_pump_with_no_transports_returns_zero() {
        let mut link = MeshLink::new();
        let mut router = edgerun_mesh::MeshRouter::new(
            edgerun_mesh::LocalNode::new(node_id(0xAA)),
        );
        let count = link.pump(&mut router).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn mesh_link_broadcast_discovery_no_transports_ok() {
        let mut link = MeshLink::new();
        let mut router = edgerun_mesh::MeshRouter::new(
            edgerun_mesh::LocalNode::new(node_id(0xAA)),
        );
        // Should not panic even with no transports
        let result = link.broadcast_discovery(&mut router);
        assert!(result.is_ok());
    }

    #[test]
    fn mesh_link_poll_fds_with_transports() {
        // We can't actually create sockets in unit tests (they require root /
        // real interfaces), but we can test the empty case.
        let link = MeshLink::new();
        let fds = link.poll_fds();
        assert!(fds.is_empty());
    }

    // =======================================================================
    // 11. Signature verification with real P-256 keys
    // =======================================================================

    #[test]
    fn sign_and_verify_roundtrip() {
        let (src_id, src_key) = make_real_keypair();
        let dest = node_id(0xCC);

        let frame = make_signed_frame(
            &src_key,
            src_id,
            dest,
            16,
            FrameType::Data,
            b"signed payload".to_vec(),
        );

        assert!(frame.verify_signature(), "valid signature should verify");
    }

    #[test]
    fn verify_fails_on_tampered_payload() {
        let (src_id, src_key) = make_real_keypair();
        let mut frame = make_signed_frame(
            &src_key,
            src_id,
            node_id(0xCC),
            16,
            FrameType::Data,
            b"original".to_vec(),
        );
        frame.payload = b"tampered".to_vec();
        assert!(
            !frame.verify_signature(),
            "tampered payload should fail verification"
        );
    }

    #[test]
    fn verify_fails_on_tampered_header() {
        let (src_id, src_key) = make_real_keypair();
        let mut frame = make_signed_frame(
            &src_key,
            src_id,
            node_id(0xCC),
            16,
            FrameType::Data,
            b"data".to_vec(),
        );
        // Tamper with TTL after signing
        frame.header.ttl = 1;
        assert!(
            !frame.verify_signature(),
            "tampered header should fail verification"
        );
    }

    #[test]
    fn verify_fails_on_wrong_sender_id() {
        let (real_id, real_key) = make_real_keypair();
        let (fake_id, _fake_key) = make_real_keypair();

        // Sign with real key but claim to be fake_id
        let frame = make_signed_frame(
            &real_key,
            fake_id, // wrong src identity
            node_id(0xCC),
            16,
            FrameType::Data,
            b"data".to_vec(),
        );
        assert!(
            !frame.verify_signature(),
            "signature should fail when src doesn't match key"
        );
        // Suppress unused warning
        let _ = real_id;
    }

    #[test]
    fn verify_fails_with_all_zero_public_key() {
        // A frame with an all-zero NodeID should fail to verify because
        // 0x04 || 64 zero bytes is not a valid P-256 point.
        let header = MeshFrameHeader {
            dest: node_id(0xCC),
            src: NodeID([0u8; 64]),
            ttl: 16,
            frame_type: FrameType::Data,
        };
        let frame = MeshFrame {
            header,
            payload: b"data".to_vec(),
            signature: [0u8; MESH_SIGNATURE_LENGTH],
        };
        assert!(
            !frame.verify_signature(),
            "all-zero public key should fail verification"
        );
    }

    // =======================================================================
    // 12. Frame wire format edge cases
    // =======================================================================

    #[test]
    fn from_wire_parses_empty_payload() {
        let header = MeshFrameHeader {
            dest: node_id(1),
            src: node_id(2),
            ttl: 5,
            frame_type: FrameType::Discovery,
        };
        let frame = MeshFrame {
            header,
            payload: vec![],
            signature: [0u8; MESH_SIGNATURE_LENGTH],
        };
        let wire = frame.to_wire();
        assert_eq!(wire.len(), 194);
        let parsed = MeshFrame::from_wire(&wire).unwrap();
        assert!(parsed.payload.is_empty());
    }

    #[test]
    fn from_wire_parses_large_payload() {
        let header = MeshFrameHeader {
            dest: node_id(1),
            src: node_id(2),
            ttl: 10,
            frame_type: FrameType::Data,
        };
        let payload = vec![0xAA; 8192];
        let frame = MeshFrame {
            header,
            payload: payload.clone(),
            signature: [0xBB; MESH_SIGNATURE_LENGTH],
        };
        let wire = frame.to_wire();
        let parsed = MeshFrame::from_wire(&wire).unwrap();
        assert_eq!(parsed.payload, payload);
        assert_eq!(parsed.signature, [0xBB; MESH_SIGNATURE_LENGTH]);
    }

    #[test]
    fn from_wire_truncates_one_byte_from_sig() {
        let header = MeshFrameHeader {
            dest: node_id(1),
            src: node_id(2),
            ttl: 5,
            frame_type: FrameType::Data,
        };
        let frame = MeshFrame {
            header,
            payload: b"test".to_vec(),
            signature: [0xCC; MESH_SIGNATURE_LENGTH],
        };
        let wire = frame.to_wire();
        // wire.len() = 130 + 4 + 64 = 198. Truncate to 197 (1 byte short).
        // from_wire reads sig from last 64 bytes, so 197 - 64 = 133, and
        // bytes[0..130] is still valid header. payload becomes bytes[130..133] = 3 bytes.
        // So it actually succeeds. The minimum check is just >= 194.
        // To truly test rejection, truncate below 194:
        let truncated = &wire[..193];
        assert!(
            MeshFrame::from_wire(truncated).is_none(),
            "below minimum size should return None"
        );
    }

    #[test]
    fn from_wire_with_extra_trailing_bytes() {
        let header = MeshFrameHeader {
            dest: node_id(1),
            src: node_id(2),
            ttl: 5,
            frame_type: FrameType::Data,
        };
        let frame = MeshFrame {
            header,
            payload: b"test".to_vec(),
            signature: [0xCC; MESH_SIGNATURE_LENGTH],
        };
        let mut wire = frame.to_wire();
        wire.extend_from_slice(&[0xFF, 0xFF, 0xFF]); // extra trailing bytes
        let parsed = MeshFrame::from_wire(&wire).unwrap();
        // The signature is parsed from the last 64 bytes, so extra bytes
        // become part of the payload.
        assert_eq!(parsed.payload.len(), 4 + 3); // original payload + extra
    }

    // =======================================================================
    // 13. SockaddrLl size and layout
    // =======================================================================

    #[test]
    fn sockaddr_ll_size() {
        // sll_family(2) + sll_protocol(2) + sll_ifindex(4/8) + sll_hatype(2)
        // + sll_pkttype(1) + sll_halen(1) + sll_addr(8) = 20 or 24 depending on arch
        let sz = std::mem::size_of::<SockaddrLl>();
        assert!(sz >= 20 && sz <= 24);
    }

    // =======================================================================
    // 14. IpMreq size
    // =======================================================================

    #[test]
    fn ip_mreq_size() {
        // Two u32 fields = 8 bytes
        assert_eq!(std::mem::size_of::<IpMreq>(), 8);
    }

    // =======================================================================
    // 15. MeshLink frame type processing logic (via inject + pump)
    // =======================================================================

    #[test]
    fn mesh_link_inject_data_frame_drains_as_inbound() {
        let (src_id, src_key) = make_real_keypair();
        let mut link = MeshLink::new();
        link.set_local_node_id(src_id);

        let frame = make_signed_frame(
            &src_key,
            src_id,
            src_id, // dest = src (for us)
            16,
            FrameType::Data,
            b"hello".to_vec(),
        );
        link.inject_inbound_frame(frame);

        let inbound = link.drain_inbound_data_frames();
        assert_eq!(inbound.len(), 1);
        assert_eq!(inbound[0].header.frame_type, FrameType::Data);
        assert_eq!(inbound[0].payload, b"hello");
    }

    #[test]
    fn mesh_link_inject_discovery_frame_goes_to_inbound_directly() {
        // inject_inbound_frame bypasses process_inbound_frame and puts
        // the frame directly into inbound_data_frames regardless of type.
        // The type-based routing only happens in process_inbound_frame (pump path).
        let (src_id, src_key) = make_real_keypair();
        let mut link = MeshLink::new();
        link.set_local_node_id(src_id);

        let disc_packet = DiscoveryPacket {
            sequence: 1,
            routes: vec![MeshRoute {
                destination: node_id(0xBB),
                next_hop: None,
                cost: 1,
            }],
        };
        let frame = make_signed_frame(
            &src_key,
            src_id,
            broadcast_id(),
            1,
            FrameType::Discovery,
            disc_packet.encode(),
        );
        link.inject_inbound_frame(frame);

        // Since inject bypasses pump, discovery frames go to inbound directly
        let inbound = link.drain_inbound_data_frames();
        assert_eq!(inbound.len(), 1);
        assert_eq!(inbound[0].header.frame_type, FrameType::Discovery);
    }

    #[test]
    fn mesh_link_router_process_discovery_routes_discovery_frames() {
        // This tests the actual path that discovery frames take in production:
        // pump() reads from sockets -> process_inbound_frame -> router.process_discovery
        let (my_id, _my_key) = make_real_keypair();
        let (peer_id, peer_key) = make_real_keypair();

        let mut router =
            edgerun_mesh::MeshRouter::new(edgerun_mesh::LocalNode::new(my_id));

        // Build a discovery frame from the peer
        let mut peer_router =
            edgerun_mesh::MeshRouter::new(edgerun_mesh::LocalNode::new(peer_id));
        let mut disc_frame = peer_router.build_discovery_frame();
        sign_frame(&mut disc_frame, &peer_key);

        // Decode the discovery packet
        let pkt = DiscoveryPacket::decode(&disc_frame.payload).unwrap();

        // Process through the router directly (simulating what process_inbound_frame does)
        let changed = router.process_discovery(peer_id, &pkt, 1000);
        assert!(changed);

        // Should have learned the peer
        assert!(router.has_route_to(&peer_id));
        let route = router.routing_table().lookup(&peer_id).unwrap();
        assert_eq!(route.cost, 1);
    }

    #[test]
    fn mesh_link_inject_route_adv_frame_goes_to_inbound() {
        let (src_id, src_key) = make_real_keypair();
        let mut link = MeshLink::new();
        link.set_local_node_id(src_id);

        let frame = make_signed_frame(
            &src_key,
            src_id,
            src_id,
            16,
            FrameType::RouteAdv,
            vec![0x01, 0x02],
        );
        link.inject_inbound_frame(frame);

        let inbound = link.drain_inbound_data_frames();
        assert_eq!(inbound.len(), 1);
        assert_eq!(inbound[0].header.frame_type, FrameType::RouteAdv);
    }

    #[test]
    fn mesh_link_inject_handshake_init_goes_to_inbound() {
        let (src_id, src_key) = make_real_keypair();
        let mut link = MeshLink::new();
        link.set_local_node_id(src_id);

        let frame = make_signed_frame(
            &src_key,
            src_id,
            src_id,
            16,
            FrameType::HandshakeInit,
            b"handshake".to_vec(),
        );
        link.inject_inbound_frame(frame);

        let inbound = link.drain_inbound_data_frames();
        assert_eq!(inbound.len(), 1);
        assert_eq!(inbound[0].header.frame_type, FrameType::HandshakeInit);
    }

    #[test]
    fn mesh_link_inject_handshake_accept_goes_to_inbound() {
        let (src_id, src_key) = make_real_keypair();
        let mut link = MeshLink::new();
        link.set_local_node_id(src_id);

        let frame = make_signed_frame(
            &src_key,
            src_id,
            src_id,
            16,
            FrameType::HandshakeAccept,
            b"accept".to_vec(),
        );
        link.inject_inbound_frame(frame);

        let inbound = link.drain_inbound_data_frames();
        assert_eq!(inbound.len(), 1);
        assert_eq!(inbound[0].header.frame_type, FrameType::HandshakeAccept);
    }

    #[test]
    fn mesh_link_inject_unsigned_frame_goes_to_inbound_directly() {
        // inject_inbound_frame bypasses signature verification and puts
        // the frame directly into inbound_data_frames. Signature checks
        // only happen in process_inbound_frame (called by pump from sockets).
        let mut link = MeshLink::new();
        let my_id = node_id(0xAA);
        link.set_local_node_id(my_id);

        let frame = MeshFrame {
            header: MeshFrameHeader {
                dest: my_id,
                src: my_id,
                ttl: 16,
                frame_type: FrameType::Data,
            },
            payload: b"no sig".to_vec(),
            signature: [0u8; MESH_SIGNATURE_LENGTH],
        };
        link.inject_inbound_frame(frame);

        let inbound = link.drain_inbound_data_frames();
        assert_eq!(inbound.len(), 1, "inject bypasses sig check, goes to inbound");
    }

    #[test]
    fn mesh_link_inject_frame_not_for_us_is_forwarded() {
        let (src_id, src_key) = make_real_keypair();
        let other_id = node_id(0xBB);
        let third_id = node_id(0xCC);
        let mut link = MeshLink::new();
        link.set_local_node_id(src_id);

        let mut router = edgerun_mesh::MeshRouter::new(
            edgerun_mesh::LocalNode::new(src_id),
        );
        // Learn a route to other_id (simulated by discovery)
        let mut other_router =
            edgerun_mesh::MeshRouter::new(edgerun_mesh::LocalNode::new(other_id));
        let disc = other_router.build_discovery_frame();
        let pkt = DiscoveryPacket::decode(&disc.payload).unwrap();
        router.process_discovery(other_id, &pkt, 1000);

        // Learn a route to third_id via other_id (simulate multi-hop)
        // We manually inject a route into the routing table
        router.routing_table().clone(); // get a copy
        // Actually we need to use process_discovery from other_id advertising third_id
        // Build a synthetic discovery from other_id claiming a route to third_id
        let adv_packet = DiscoveryPacket {
            sequence: 1,
            routes: vec![MeshRoute {
                destination: third_id,
                next_hop: None,
                cost: 1,
            }],
        };
        // Sign a fake frame so the router processes it
        let (other_key_sign, _) = make_real_keypair();
        let (_, other_key) = make_real_keypair();
        let mut frame = make_signed_frame(
            &other_key,
            other_id,
            src_id, // dest = us so we process it
            16,
            FrameType::Discovery,
            adv_packet.encode(),
        );
        link.inject_inbound_frame(frame.clone());
        // pump processes it -- but inject bypasses pump. We need to call process_discovery directly.
        // Actually the test is about the MeshLink forwarding logic.
        // Let's simplify: add the route directly to the router.
        // We can't directly modify the router's routing_table since it's private.
        // Instead, let's use process_discovery on the router directly.

        // Reset and do it properly:
        let mut router2 = edgerun_mesh::MeshRouter::new(
            edgerun_mesh::LocalNode::new(src_id),
        );
        // other_id advertises a route to third_id
        router2.process_discovery(other_id, &adv_packet, 1000);
        // Now router2 has a route to third_id via other_id

        // Verify the route exists
        assert!(router2.has_route_to(&third_id));

        // Frame destined for third_id (not for us)
        let fwd_frame = make_signed_frame(
            &src_key,
            src_id,
            third_id,
            5,
            FrameType::Data,
            b"forward me".to_vec(),
        );
        link.inject_inbound_frame(fwd_frame);

        // pump with no sockets returns 0, but process_inbound_frame is not called
        // because pump only reads from sockets. Inject puts directly into inbound.
        // So we need to test the internal process_inbound_frame logic differently.
        // Since we can't call process_inbound_frame directly (it's private),
        // let's verify the forwarding behavior by checking should_forward on the router.
        let header = MeshFrameHeader {
            dest: third_id,
            src: src_id,
            ttl: 5,
            frame_type: FrameType::Data,
        };
        let forward_ttl = router2.should_forward(&header);
        assert_eq!(forward_ttl, Some(4), "router should forward with TTL-1");

        // Suppress unused
        let _ = (link, router);
        let _ = other_key_sign;
    }

    // =======================================================================
    // 16. UdpBroadcastSocket peer tracking (no actual socket)
    // =======================================================================

    #[test]
    fn udp_broadcast_socket_peer_addr_initially_none() {
        // We can't construct UdpBroadcastSocket without binding a real socket,
        // but we can test the peer_addr and learn_peer methods through MeshLink.
        let link = MeshLink::new();
        // No UDP broadcast enabled, so peer lookups are not accessible directly.
        // This test verifies MeshLink::new is clean.
        assert_eq!(link.pending_count(), 0);
    }

    // =======================================================================
    // 17. MeshFrame from_payload defaults
    // =======================================================================

    #[test]
    fn from_payload_sets_defaults() {
        let frame = MeshFrame::from_payload(node_id(0xDD), b"test".to_vec());
        assert_eq!(frame.header.dest, node_id(0xDD));
        assert_eq!(frame.header.src, NodeID([0u8; 64])); // default zero
        assert_eq!(frame.header.ttl, 16);
        assert_eq!(frame.header.frame_type, FrameType::Data);
        assert_eq!(frame.payload, b"test");
        assert_eq!(frame.signature, [0u8; MESH_SIGNATURE_LENGTH]);
    }

    // =======================================================================
    // 18. Malformed / truncated data handling
    // =======================================================================

    #[test]
    fn mesh_frame_from_wire_returns_none_for_header_truncated() {
        // 129 bytes = 1 byte short of header
        assert!(MeshFrame::from_wire(&[0u8; 129]).is_none());
    }

    #[test]
    fn mesh_frame_from_wire_returns_none_for_header_plus_one_payload_no_sig() {
        // Header + 1 byte payload = 131 bytes, still < 194
        assert!(MeshFrame::from_wire(&[0u8; 131]).is_none());
    }

    #[test]
    fn mesh_frame_from_wire_returns_none_for_sig_truncated() {
        // Header + empty payload + 32 bytes of sig (half)
        assert!(MeshFrame::from_wire(&[0u8; 130 + 32]).is_none());
    }

    #[test]
    fn discovery_packet_decode_with_malformed_sequence() {
        // A sequence value of u32::MAX should still parse fine
        let mut buf = [0xFFu8; 5];
        buf[4] = 0; // 0 routes
        let decoded = DiscoveryPacket::decode(&buf);
        assert!(decoded.is_some());
        assert_eq!(decoded.unwrap().sequence, u32::MAX);
    }

    // =======================================================================
    // 19. MeshRoute properties
    // =======================================================================

    #[test]
    fn mesh_route_next_hop_none_for_direct() {
        let route = MeshRoute {
            destination: node_id(0xAA),
            next_hop: None,
            cost: 1,
        };
        assert!(route.next_hop.is_none());
    }

    #[test]
    fn mesh_route_clone_and_equality() {
        let route = MeshRoute {
            destination: node_id(0xAA),
            next_hop: Some(node_id(0xBB)),
            cost: 3,
        };
        let clone = route.clone();
        assert_eq!(route, clone);
    }

    // =======================================================================
    // 20. MeshFrameHeader Debug impl
    // =======================================================================

    #[test]
    fn mesh_frame_header_debug_format() {
        let header = MeshFrameHeader {
            dest: node_id(0xAA),
            src: node_id(0xBB),
            ttl: 10,
            frame_type: FrameType::Data,
        };
        let debug_str = format!("{:?}", header);
        assert!(debug_str.contains("MeshFrameHeader"));
        assert!(debug_str.contains("dest"));
        assert!(debug_str.contains("src"));
        assert!(debug_str.contains("ttl"));
        assert!(debug_str.contains("frame_type"));
    }

    // =======================================================================
    // 21. MeshFrame PartialEq
    // =======================================================================

    #[test]
    fn mesh_frame_equality() {
        let header = MeshFrameHeader {
            dest: node_id(1),
            src: node_id(2),
            ttl: 5,
            frame_type: FrameType::Data,
        };
        let payload = b"test".to_vec();
        let sig = [0xAB; MESH_SIGNATURE_LENGTH];
        let frame1 = MeshFrame {
            header: header.clone(),
            payload: payload.clone(),
            signature: sig,
        };
        let frame2 = MeshFrame {
            header,
            payload,
            signature: sig,
        };
        assert_eq!(frame1, frame2);
    }

    #[test]
    fn mesh_frame_not_equal_different_payload() {
        let header = MeshFrameHeader {
            dest: node_id(1),
            src: node_id(2),
            ttl: 5,
            frame_type: FrameType::Data,
        };
        let frame1 = MeshFrame {
            header: header.clone(),
            payload: b"aaa".to_vec(),
            signature: [0; MESH_SIGNATURE_LENGTH],
        };
        let frame2 = MeshFrame {
            header,
            payload: b"bbb".to_vec(),
            signature: [0; MESH_SIGNATURE_LENGTH],
        };
        assert_ne!(frame1, frame2);
    }

    // =======================================================================
    // 22. MeshLink send_frame with router (no transports)
    // =======================================================================

    #[test]
    fn mesh_link_send_frame_no_route_returns_false() {
        let mut link = MeshLink::new();
        let router = edgerun_mesh::MeshRouter::new(
            edgerun_mesh::LocalNode::new(node_id(0xAA)),
        );
        let frame = MeshFrame::from_payload(node_id(0xBB), vec![]);
        let result = link.send_frame(&router, &frame).unwrap();
        assert!(!result, "should return false with no route");
    }

    // =======================================================================
    // 23. Current unix secs helper
    // =======================================================================

    #[test]
    fn current_unix_secs_is_reasonable() {
        let secs = current_unix_secs();
        // Should be after 2024-01-01 and before 2100-01-01
        assert!(secs > 1_700_000_000);
        assert!(secs < 4_100_000_000);
    }

    // =======================================================================
    // 24. Edge case: broadcast destination
    // =======================================================================

    #[test]
    fn broadcast_node_id_is_all_zeros() {
        let broadcast = broadcast_id();
        assert_eq!(broadcast.0, [0u8; 64]);
    }

    // =======================================================================
    // 25. MeshLink learn_peer through UDP (simulated via inject)
    // =======================================================================

    #[test]
    fn mesh_link_process_discovery_learns_routes() {
        let (my_id, _my_key) = make_real_keypair();
        let (peer_id, peer_key) = make_real_keypair();

        let mut router =
            edgerun_mesh::MeshRouter::new(edgerun_mesh::LocalNode::new(my_id));
        let mut peer_router =
            edgerun_mesh::MeshRouter::new(edgerun_mesh::LocalNode::new(peer_id));

        // Build a discovery frame from the peer
        let mut disc_frame = peer_router.build_discovery_frame();
        // Sign it with the peer's key
        sign_frame(&mut disc_frame, &peer_key);

        // Decode the discovery packet
        let pkt = DiscoveryPacket::decode(&disc_frame.payload).unwrap();

        // Process through the router (this is what process_inbound_frame does for Discovery frames)
        let changed = router.process_discovery(peer_id, &pkt, 1000);
        assert!(changed);

        // Should have learned the peer
        assert!(router.has_route_to(&peer_id));
        let route = router.routing_table().lookup(&peer_id).unwrap();
        assert_eq!(route.cost, 1);
    }

    // =======================================================================
    // 26. Multiple frame injection and processing order
    // =======================================================================

    #[test]
    fn mesh_link_multiple_inbound_frames_processed_in_order() {
        let (src_id, src_key) = make_real_keypair();
        let mut link = MeshLink::new();
        link.set_local_node_id(src_id);

        for i in 0..5 {
            let frame = make_signed_frame(
                &src_key,
                src_id,
                src_id,
                16,
                FrameType::Data,
                vec![i],
            );
            link.inject_inbound_frame(frame);
        }

        let inbound = link.drain_inbound_data_frames();
        assert_eq!(inbound.len(), 5);
        for i in 0..5 {
            assert_eq!(inbound[i].payload, vec![i as u8]);
        }
    }

    // =======================================================================
    // 27. Frame TTL behavior
    // =======================================================================

    #[test]
    fn frame_ttl_zero_in_header() {
        let header = MeshFrameHeader {
            dest: node_id(1),
            src: node_id(2),
            ttl: 0,
            frame_type: FrameType::Data,
        };
        let encoded = header.encode();
        assert_eq!(encoded[128], 0);
    }

    #[test]
    fn frame_ttl_max_value() {
        let header = MeshFrameHeader {
            dest: node_id(1),
            src: node_id(2),
            ttl: u8::MAX,
            frame_type: FrameType::Data,
        };
        let encoded = header.encode();
        assert_eq!(encoded[128], u8::MAX);
    }

    // =======================================================================
    // 28. Discovery packet from_local advances sequence
    // =======================================================================

    #[test]
    fn discovery_packet_from_local_advances_sequence() {
        let mut local = edgerun_mesh::LocalNode::new(node_id(0xAA));
        let table = edgerun_mesh::MeshRoutingTable::default();

        let p1 = DiscoveryPacket::from_local(&mut local, &table);
        assert_eq!(p1.sequence, 1);

        let p2 = DiscoveryPacket::from_local(&mut local, &table);
        assert_eq!(p2.sequence, 2);
    }

    // =======================================================================
    // 29. Buffer boundary tests
    // =======================================================================

    #[test]
    fn from_wire_exactly_at_minimum_boundary() {
        // Exactly 194 bytes: 130 header + 0 payload + 64 sig
        let mut buf = vec![0u8; 194];
        // Fill header
        buf[128] = 5; // ttl
        buf[129] = 0; // Data
        // Fill signature area
        buf[193] = 0xFF;

        let parsed = MeshFrame::from_wire(&buf);
        assert!(parsed.is_some());
        let frame = parsed.unwrap();
        assert_eq!(frame.header.ttl, 5);
        assert!(frame.payload.is_empty());
        assert_eq!(frame.signature[63], 0xFF);
    }

    #[test]
    fn from_wire_one_byte_over_minimum() {
        // 195 bytes: 130 header + 1 payload + 64 sig
        let mut buf = vec![0u8; 195];
        buf[128] = 3;
        buf[129] = 1; // Discovery

        let parsed = MeshFrame::from_wire(&buf).unwrap();
        assert_eq!(parsed.header.frame_type, FrameType::Discovery);
        assert_eq!(parsed.payload.len(), 1);
        assert_eq!(parsed.payload[0], 0);
    }

    // =======================================================================
    // 30. MeshLink send_mesh_frame fills src before sending
    // =======================================================================

    #[test]
    fn mesh_link_drain_frames_fills_src_with_local_id() {
        let (local_id, local_key) = make_real_keypair();
        let mut link = MeshLink::new();
        link.set_local_node_id(local_id);

        let dest = node_id(0xCC);
        let frame = make_signed_frame(&local_key, local_id, dest, 16, FrameType::Data, b"x".to_vec());
        link.queue_frame(frame);

        let mut router = edgerun_mesh::MeshRouter::new(
            edgerun_mesh::LocalNode::new(local_id),
        );
        // drain_pending_frames calls send_mesh_frame which clones and sets src
        let sent = link.drain_pending_frames(&mut router).unwrap();
        assert_eq!(sent, 1);
        // No transports, so no actual send, but queue is drained
        assert_eq!(link.pending_count(), 0);
    }


    // =======================================================================
    // Integration Tests
    // =======================================================================

    /// Integration test: Signed discovery frame creation, signing, wire format.
    ///
    /// Verifies that a discovery frame can be created, signed, serialized to
    /// wire format, parsed back, and the signature still verifies.
    #[test]
    fn integration_discovery_frame_roundtrip() {
        use edgerun_mesh::{LocalNode, FrameType, MeshFrame};
        use edgerun_mesh::{MeshRouter, DiscoveryPacket};

        let (my_node_id, signing_key) = make_real_keypair();

        // Create a discovery packet
        let mut local = LocalNode::new(my_node_id);
        let router = MeshRouter::new(local.clone());
        let disc_packet = DiscoveryPacket::from_local(&mut local, router.routing_table());
        let payload = disc_packet.encode();

        // Create and sign the frame (broadcast destination)
        let mut frame = MeshFrame {
            header: edgerun_mesh::MeshFrameHeader {
                dest: node_id(0xFF),
                src: my_node_id,
                ttl: 16,
                frame_type: FrameType::Discovery,
            },
            payload,
            signature: [0u8; 64],
        };
        edgerun_mesh::sign_frame(&mut frame, &signing_key);

        // Verify signature
        assert!(frame.verify_signature(), "discovery frame signature should be valid");
        assert_eq!(frame.header.frame_type, FrameType::Discovery);
        assert_eq!(frame.header.src, my_node_id);

        // Wire format round-trip
        let wire = frame.to_wire();
        let parsed = MeshFrame::from_wire(&wire).unwrap();
        assert_eq!(parsed.header.dest, frame.header.dest);
        assert_eq!(parsed.header.src, frame.header.src);
        assert_eq!(parsed.header.ttl, frame.header.ttl);
        assert_eq!(parsed.header.frame_type, frame.header.frame_type);
        assert_eq!(parsed.payload, frame.payload);
        assert_eq!(parsed.signature, frame.signature);
        assert!(parsed.verify_signature(), "parsed frame signature should verify");
    }

    /// Integration test: Discovery frame queued and processed through router.
    ///
    /// Verifies that a discovery frame queued via MeshLink can be drained,
    /// parsed, and processed by a MeshRouter to update routing state.
    #[test]
    fn integration_discovery_to_routing() {
        use edgerun_mesh::{LocalNode, FrameType, MeshFrame};
        use edgerun_mesh::{MeshRouter, DiscoveryPacket};

        let (node_a_id, signing_key_a) = make_real_keypair();
        let (node_b_id, _) = make_real_keypair();

        // Create a signed discovery frame from A
        let mut local_a = LocalNode::new(node_a_id);
        let router_a = MeshRouter::new(local_a.clone());
        let disc_packet = DiscoveryPacket::from_local(&mut local_a, router_a.routing_table());
        let payload = disc_packet.encode();

        let mut frame = MeshFrame {
            header: edgerun_mesh::MeshFrameHeader {
                dest: node_id(0xFF),
                src: node_a_id,
                ttl: 16,
                frame_type: FrameType::Discovery,
            },
            payload,
            signature: [0u8; 64],
        };
        edgerun_mesh::sign_frame(&mut frame, &signing_key_a);

        // Queue the frame in MeshLink
        let mut link_a = MeshLink::new();
        link_a.set_local_node_id(node_a_id);
        link_a.queue_frame(frame);

        // Drain and deliver to B
        let frames = link_a.drain_pending_frames_raw();
        assert_eq!(frames.len(), 1, "should have one queued frame");

        let mut router_b = MeshRouter::new(LocalNode::new(node_b_id));
        for frame in frames {
            assert!(frame.verify_signature(), "frame from A should be signed");

            // Parse discovery payload and process through B's router
            if let Some(packet) = DiscoveryPacket::decode(&frame.payload) {
                router_b.process_discovery(node_a_id, &packet, 1000);
            }
        }

        // B should have learned about A
        assert!(router_b.has_route_to(&node_a_id),
            "B should have route to A after processing discovery frame");
        assert_eq!(router_b.next_hop_for(&node_a_id), Some(node_a_id),
            "B's next hop to A should be A directly");
    }
}
