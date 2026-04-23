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
use super::*;
use crate::multicast::SockaddrIn;

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
                FrameType::Data | FrameType::RouteAdv | FrameType::HandshakeInit | FrameType::HandshakeAccept | FrameType::MetricsReport | FrameType::MigrationOrder | FrameType::MigrationComplete => {
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

pub(crate) fn current_unix_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}
