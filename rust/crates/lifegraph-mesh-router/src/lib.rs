//! Discovery protocol and distance-vector routing for the Lifegraph mesh.
//!
//! Every node periodically multicasts a **hello packet** containing:
//! - Its NodeID (ECDSA P-256 public key — its identity and address)
//! - Its current routing table (so peers learn multi-hop paths)
//!
//! On receipt, each node runs a Bellman-Ford update:
//! ```text
//! cost(via_peer, destination) = cost(myself, via_peer) + cost(via_peer, destination)
//! ```
//! If this is better than the current known cost, the route is updated and
//! advertised in the next hello.

use lifegraph_hardware_signing::{MESH_SIGNATURE_LENGTH, NodeID};
use lifegraph_mesh::{
    FrameType, LocalNode, MeshFrame, MeshFrameHeader, MeshPeer, MeshRoute, MeshRoutingTable,
};

// ---------------------------------------------------------------------------
// Discovery packet (serialized payload)
// ---------------------------------------------------------------------------

/// Serialized discovery packet payload.
///
/// Wire layout:
///   [0..4)   sequence (u32, little-endian)
///   [4..5)   route_count (u8)
///   [5..N)   routes: each is 65 bytes:
///             [0..64) destination NodeID
///             [64]    cost
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveryPacket {
    pub sequence: u32,
    pub routes: Vec<MeshRoute>,
}

impl DiscoveryPacket {
    /// Maximum number of routes that fit in a single discovery packet
    /// without exceeding typical MTU constraints.
    /// With 65 bytes per route entry: 65 * 50 = 3250 bytes of route data.
    pub const MAX_ROUTES: usize = 50;

    /// Serialize into a byte vector.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let count = self.routes.len().min(Self::MAX_ROUTES) as u8;
        let mut out = Vec::with_capacity(5 + (count as usize) * 65);
        out.extend_from_slice(&self.sequence.to_le_bytes());
        out.push(count);
        for route in self.routes.iter().take(Self::MAX_ROUTES) {
            out.extend_from_slice(&route.destination.0);
            out.push(route.cost);
        }
        out
    }

    /// Parse from bytes. Returns `None` on malformed input.
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 5 {
            return None;
        }
        let sequence = u32::from_le_bytes(bytes[0..4].try_into().ok()?);
        let count = bytes[4] as usize;
        let expected = 5 + count * 65;
        if bytes.len() < expected {
            return None;
        }
        let mut routes = Vec::with_capacity(count);
        for i in 0..count {
            let offset = 5 + i * 65;
            let mut dest = [0u8; 64];
            dest.copy_from_slice(&bytes[offset..offset + 64]);
            let cost = bytes[offset + 64];
            routes.push(MeshRoute {
                destination: NodeID(dest),
                next_hop: None, // next_hop is determined by who sent this packet
                cost,
            });
        }
        Some(Self { sequence, routes })
    }

    /// Build from a local node and its routing table.
    #[must_use]
    pub fn from_local(local: &mut LocalNode, table: &MeshRoutingTable) -> Self {
        let seq = local.next_sequence();
        Self {
            sequence: seq,
            routes: table.iter().cloned().collect(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router state
// ---------------------------------------------------------------------------

/// The mesh router manages discovery, peer tracking, and route computation.
#[derive(Clone, Debug)]
pub struct MeshRouter {
    /// This node's local identity.
    local: LocalNode,
    /// Direct peers (cost 1) keyed by NodeID.
    peers: std::collections::HashMap<NodeID, MeshPeer>,
    /// The computed routing table.
    routing_table: MeshRoutingTable,
    /// Default TTL for outbound frames.
    default_ttl: u8,
    /// Heartbeat interval in seconds.
    heartbeat_interval_secs: u8,
}

impl MeshRouter {
    pub fn new(local: LocalNode) -> Self {
        Self {
            local,
            peers: std::collections::HashMap::new(),
            routing_table: MeshRoutingTable::default(),
            default_ttl: 16,
            heartbeat_interval_secs: 5,
        }
    }

    // -----------------------------------------------------------------------
    // Accessors
    // -----------------------------------------------------------------------

    #[must_use]
    pub fn node_id(&self) -> NodeID {
        self.local.node_id
    }

    #[must_use]
    pub fn routing_table(&self) -> &MeshRoutingTable {
        &self.routing_table
    }

    #[must_use]
    pub fn local(&self) -> &LocalNode {
        &self.local
    }

    #[must_use]
    pub fn local_mut(&mut self) -> &mut LocalNode {
        &mut self.local
    }

    #[must_use]
    pub fn default_ttl(&self) -> u8 {
        self.default_ttl
    }

    pub fn set_default_ttl(&mut self, ttl: u8) {
        self.default_ttl = ttl;
    }

    #[must_use]
    pub fn heartbeat_interval_secs(&self) -> u8 {
        self.heartbeat_interval_secs
    }

    pub fn set_heartbeat_interval_secs(&mut self, secs: u8) {
        self.heartbeat_interval_secs = secs;
    }

    // -----------------------------------------------------------------------
    // Discovery outbound
    // -----------------------------------------------------------------------

    /// Build a discovery frame to multicast to all peers.
    ///
    /// This frame's `dest` is the **broadcast NodeID** (all zeroes) to
    /// indicate "all nodes".  The payload is the serialized discovery packet.
    #[must_use]
    pub fn build_discovery_frame(&mut self) -> MeshFrame {
        let packet = DiscoveryPacket::from_local(&mut self.local, &self.routing_table);
        let payload = packet.encode();
        MeshFrame {
            header: MeshFrameHeader {
                dest: NodeID([0u8; 64]), // broadcast
                src: self.local.node_id,
                ttl: 1, // discovery only travels one hop
                frame_type: FrameType::Discovery,
            },
            payload,
            signature: [0u8; MESH_SIGNATURE_LENGTH], // signed by the link layer
        }
    }

    // -----------------------------------------------------------------------
    // Discovery inbound
    // -----------------------------------------------------------------------

    /// Process a received discovery frame from a peer.
    ///
    /// This updates the peer table, runs Bellman-Ford route updates,
    /// and returns `true` if the routing table changed.
    pub fn process_discovery(
        &mut self,
        from: NodeID,
        packet: &DiscoveryPacket,
        now_unix: i64,
    ) -> bool {
        // Update or create the peer entry
        let peer = self
            .peers
            .entry(from)
            .or_insert_with(|| MeshPeer {
                node_id: from,
                advertised_routes: Vec::new(),
                last_seen_unix: now_unix,
                missed_heartbeats: 0,
            });
        peer.last_seen_unix = now_unix;
        peer.missed_heartbeats = 0;
        peer.advertised_routes.clone_from(&packet.routes);

        // Bellman-Ford: update routes via this peer
        let mut changed = false;
        let cost_to_peer: u8 = 1; // direct peer

        for advertised in &packet.routes {
            // Skip routes that claim to go through us
            if advertised.destination == self.local.node_id {
                continue;
            }
            // Skip our own routes being echoed back
            if advertised.destination == from {
                continue;
            }
            let new_cost = advertised.cost.saturating_add(cost_to_peer);
            if new_cost == 0 {
                continue; // invalid cost
            }

            // Check if we already have a better route
            let dominated = self
                .routing_table
                .lookup(&advertised.destination)
                .is_some_and(|existing| existing.cost <= new_cost);
            if dominated {
                continue;
            }

            self.routing_table.update(MeshRoute {
                destination: advertised.destination,
                next_hop: Some(from),
                cost: new_cost,
            });
            changed = true;
        }

        // Ensure the direct route to this peer exists
        let peer_dominated = self
            .routing_table
            .lookup(&from)
            .is_some_and(|r| r.cost <= 1);
        if !peer_dominated {
            self.routing_table.update(MeshRoute {
                destination: from,
                next_hop: None,
                cost: 1,
            });
            changed = true;
        }

        // Also add a route for ourselves
        self.routing_table.update(MeshRoute {
            destination: self.local.node_id,
            next_hop: None,
            cost: 0,
        });

        changed
    }

    // -----------------------------------------------------------------------
    // Peer liveness
    // -----------------------------------------------------------------------

    /// Increment missed heartbeat counters for all peers.
    /// Call this every `heartbeat_interval_secs` seconds.
    /// Returns the list of peers that just went dead (were alive before).
    pub fn tick_heartbeat(&mut self) -> Vec<NodeID> {
        let mut went_dead = Vec::new();
        for peer in self.peers.values_mut() {
            peer.missed_heartbeats = peer.missed_heartbeats.saturating_add(1);
            if peer.missed_heartbeats == 3 {
                went_dead.push(peer.node_id);
            }
        }
        for dead_id in &went_dead {
            self.remove_peer(dead_id);
        }
        went_dead
    }

    // -----------------------------------------------------------------------
    // Peer management
    // -----------------------------------------------------------------------

    /// Removes a peer and all routes that go through it (including the direct
    /// route to the peer itself).
    pub fn remove_peer(&mut self, peer_id: &NodeID) {
        self.peers.remove(peer_id);
        self.routing_table.remove_via(peer_id);
        self.routing_table.remove_destination(peer_id);
    }

    /// Returns all currently alive peers.
    #[must_use]
    pub fn active_peers(&self) -> Vec<&MeshPeer> {
        self.peers.values().filter(|p| !p.is_dead()).collect()
    }

    /// Returns all peers including dead ones.
    #[must_use]
    pub fn all_peers(&self) -> Vec<&MeshPeer> {
        self.peers.values().collect()
    }

    // -----------------------------------------------------------------------
    // Frame forwarding decisions
    // -----------------------------------------------------------------------

    /// Given a destination NodeID, return the next-hop NodeID to forward to.
    /// Returns `None` if no route exists or the destination is ourselves.
    #[must_use]
    pub fn next_hop_for(&self, destination: &NodeID) -> Option<NodeID> {
        if *destination == self.local.node_id {
            return None;
        }
        self.routing_table
            .lookup(destination)
            .and_then(|route| {
                if route.next_hop.is_none() {
                    // Direct route — the destination is a peer
                    Some(*destination)
                } else {
                    route.next_hop
                }
            })
    }

    /// Returns `true` if we have a route to the given destination.
    #[must_use]
    pub fn has_route_to(&self, destination: &NodeID) -> bool {
        self.routing_table.lookup(destination).is_some()
    }

    /// Decrement TTL and check if a frame should be forwarded or dropped.
    /// Returns `Some(forward_ttl)` if the frame should be forwarded,
    /// or `None` if it should be dropped (TTL expired or destination is us).
    #[must_use]
    pub fn should_forward(&self, header: &MeshFrameHeader) -> Option<u8> {
        if header.dest == self.local.node_id {
            return None; // destined for us, don't forward
        }
        if header.ttl == 0 {
            return None; // expired
        }
        // We have a route to the destination?
        if !self.has_route_to(&header.dest) {
            return None; // no route, can't forward
        }
        Some(header.ttl - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node_id(v: u8) -> NodeID {
        let mut bytes = [0u8; 64];
        bytes[0] = v;
        NodeID(bytes)
    }

    fn make_router(id: u8) -> MeshRouter {
        MeshRouter::new(LocalNode::new(node_id(id)))
    }

    #[test]
    fn discovery_encode_decode_roundtrip() {
        let packet = DiscoveryPacket {
            sequence: 42,
            routes: vec![
                MeshRoute {
                    destination: node_id(0xAA),
                    next_hop: None,
                    cost: 1,
                },
                MeshRoute {
                    destination: node_id(0xBB),
                    next_hop: Some(node_id(0xAA)),
                    cost: 2,
                },
            ],
        };
        let encoded = packet.encode();
        let decoded = DiscoveryPacket::decode(&encoded).expect("decode should succeed");
        assert_eq!(decoded.sequence, 42);
        assert_eq!(decoded.routes.len(), 2);
        assert_eq!(decoded.routes[0].destination, node_id(0xAA));
        assert_eq!(decoded.routes[0].cost, 1);
        assert_eq!(decoded.routes[1].destination, node_id(0xBB));
        assert_eq!(decoded.routes[1].cost, 2);
    }

    #[test]
    fn discovery_decode_rejects_truncated() {
        assert!(DiscoveryPacket::decode(&[]).is_none());
        assert!(DiscoveryPacket::decode(&[0; 4]).is_none()); // no count byte
        assert!(DiscoveryPacket::decode(&[0; 5]).is_some()); // 0 routes is valid
        assert!(DiscoveryPacket::decode(&[1; 69]).is_none()); // count=1 but only 64 data bytes (need 65)
    }

    #[test]
    fn direct_peer_learned_from_discovery() {
        let mut router_a = make_router(0xAA);
        let mut router_b = make_router(0xBB);

        // B sends discovery to A
        let disc_b = router_b.build_discovery_frame();
        let packet =
            DiscoveryPacket::decode(&disc_b.payload).expect("should decode B's discovery");

        // A processes B's discovery
        let changed = router_a.process_discovery(node_id(0xBB), &packet, 1000);
        assert!(changed, "routing table should have changed");

        // A now has a direct route to B
        assert!(router_a.has_route_to(&node_id(0xBB)));
        let route = router_a.routing_table().lookup(&node_id(0xBB)).unwrap();
        assert_eq!(route.cost, 1);
        assert!(route.next_hop.is_none()); // direct

        // A knows B is a peer
        assert_eq!(router_a.active_peers().len(), 1);
    }

    #[test]
    fn multi_hop_route_learned_via_bellman_ford() {
        let mut a = make_router(0xAA);
        let mut b = make_router(0xBB);
        let mut c = make_router(0xCC);

        // A discovers B directly
        let disc_a = a.build_discovery_frame();
        let pkt_a = DiscoveryPacket::decode(&disc_a.payload).unwrap();
        b.process_discovery(node_id(0xAA), &pkt_a, 1000);

        // B discovers C directly
        let disc_b = b.build_discovery_frame();
        let pkt_b = DiscoveryPacket::decode(&disc_b.payload).unwrap();
        c.process_discovery(node_id(0xBB), &pkt_b, 1000);

        // C should now have:
        //   → B cost 1 (direct)
        //   → A cost 2 (via B, learned from B's discovery)
        assert!(c.has_route_to(&node_id(0xAA)));
        let route_to_a = c.routing_table().lookup(&node_id(0xAA)).unwrap();
        assert_eq!(route_to_a.cost, 2);
        assert_eq!(route_to_a.next_hop, Some(node_id(0xBB)));

        assert!(c.has_route_to(&node_id(0xBB)));
        let route_to_b = c.routing_table().lookup(&node_id(0xBB)).unwrap();
        assert_eq!(route_to_b.cost, 1);
    }

    #[test]
    fn three_node_chain_a_b_c_d() {
        let mut a = make_router(0xAA);
        let mut b = make_router(0xBB);
        let mut c = make_router(0xCC);
        let mut d = make_router(0xDD);

        // A → B
        let pkt_a = DiscoveryPacket::decode(&a.build_discovery_frame().payload).unwrap();
        b.process_discovery(node_id(0xAA), &pkt_a, 1000);

        // B → C (B now knows about A at cost 1, advertises to C)
        let pkt_b = DiscoveryPacket::decode(&b.build_discovery_frame().payload).unwrap();
        c.process_discovery(node_id(0xBB), &pkt_b, 1000);

        // C → D (C now knows about A at cost 2, B at cost 1, advertises to D)
        let pkt_c = DiscoveryPacket::decode(&c.build_discovery_frame().payload).unwrap();
        d.process_discovery(node_id(0xCC), &pkt_c, 1000);

        // D should reach A at cost 3, B at cost 2, C at cost 1
        let route_to_a = d.routing_table().lookup(&node_id(0xAA)).unwrap();
        assert_eq!(route_to_a.cost, 3);
        assert_eq!(route_to_a.next_hop, Some(node_id(0xCC)));

        let route_to_b = d.routing_table().lookup(&node_id(0xBB)).unwrap();
        assert_eq!(route_to_b.cost, 2);
        assert_eq!(route_to_b.next_hop, Some(node_id(0xCC)));

        let route_to_c = d.routing_table().lookup(&node_id(0xCC)).unwrap();
        assert_eq!(route_to_c.cost, 1);
        assert!(route_to_c.next_hop.is_none()); // direct
    }

    #[test]
    fn peer_dead_after_missed_heartbeats() {
        let mut router = make_router(0xAA);

        // Learn a peer
        let mut peer_router = make_router(0xBB);
        let disc = peer_router.build_discovery_frame();
        let pkt = DiscoveryPacket::decode(&disc.payload).unwrap();
        router.process_discovery(node_id(0xBB), &pkt, 1000);

        assert_eq!(router.active_peers().len(), 1);

        // Tick 3 times — peer should go dead
        let dead1 = router.tick_heartbeat();
        assert!(dead1.is_empty()); // 1 miss
        let dead2 = router.tick_heartbeat();
        assert!(dead2.is_empty()); // 2 misses
        let dead3 = router.tick_heartbeat();
        assert_eq!(dead3, vec![node_id(0xBB)]); // 3 misses → dead

        assert_eq!(router.active_peers().len(), 0);
        // Routes via BB should be removed
        assert!(!router.has_route_to(&node_id(0xBB)));
    }

    #[test]
    fn next_hop_for_returns_correct_forwarding_target() {
        let mut a = make_router(0xAA);
        let mut b = make_router(0xBB);

        let pkt_a = DiscoveryPacket::decode(&a.build_discovery_frame().payload).unwrap();
        b.process_discovery(node_id(0xAA), &pkt_a, 1000);

        // B's next hop to A should be A itself (direct)
        assert_eq!(b.next_hop_for(&node_id(0xAA)), Some(node_id(0xAA)));

        // No route to unknown node
        assert!(b.next_hop_for(&node_id(0xFF)).is_none());

        // No route to ourselves
        assert!(b.next_hop_for(&node_id(0xBB)).is_none());
    }

    #[test]
    fn should_forward_respects_ttl_and_destination() {
        let mut router = make_router(0xAA);

        // Frame destined for us
        let hdr_for_us = MeshFrameHeader {
            dest: node_id(0xAA),
            src: node_id(0xBB),
            ttl: 10,
            frame_type: FrameType::Data,
        };
        assert!(router.should_forward(&hdr_for_us).is_none());

        // Frame with unknown destination
        let hdr_unknown = MeshFrameHeader {
            dest: node_id(0xFF),
            src: node_id(0xBB),
            ttl: 10,
            frame_type: FrameType::Data,
        };
        assert!(router.should_forward(&hdr_unknown).is_none());

        // Frame with zero TTL
        let mut learn_a_route = make_router(0xBB);
        let pkt = DiscoveryPacket::decode(&learn_a_route.build_discovery_frame().payload).unwrap();
        router.process_discovery(node_id(0xBB), &pkt, 1000);

        let hdr_expired = MeshFrameHeader {
            dest: node_id(0xBB),
            src: node_id(0xCC),
            ttl: 0,
            frame_type: FrameType::Data,
        };
        assert!(router.should_forward(&hdr_expired).is_none());

        // Valid frame with route
        let hdr_valid = MeshFrameHeader {
            dest: node_id(0xBB),
            src: node_id(0xCC),
            ttl: 5,
            frame_type: FrameType::Data,
        };
        assert_eq!(router.should_forward(&hdr_valid), Some(4));
    }

    #[test]
    fn bellman_ford_converges_to_best_path() {
        // A—B—D  cost 2
        // A—C—D  cost 2
        // Both paths should be discovered, first one wins
        let mut a = make_router(0xAA);
        let mut b = make_router(0xBB);
        let mut c = make_router(0xCC);
        let mut d = make_router(0xDD);

        // A→B
        let pkt_a = DiscoveryPacket::decode(&a.build_discovery_frame().payload).unwrap();
        b.process_discovery(node_id(0xAA), &pkt_a, 1000);

        // A→C
        let pkt_a2 = DiscoveryPacket::decode(&a.build_discovery_frame().payload).unwrap();
        c.process_discovery(node_id(0xAA), &pkt_a2, 1000);

        // B→D (B advertises A at cost 1)
        let pkt_b = DiscoveryPacket::decode(&b.build_discovery_frame().payload).unwrap();
        d.process_discovery(node_id(0xBB), &pkt_b, 1000);

        // C→D (C advertises A at cost 1)
        let pkt_c = DiscoveryPacket::decode(&c.build_discovery_frame().payload).unwrap();
        d.process_discovery(node_id(0xCC), &pkt_c, 1000);

        // D should reach A at cost 2 (either via B or C, first learned wins)
        let route = d.routing_table().lookup(&node_id(0xAA)).unwrap();
        assert_eq!(route.cost, 2);
        // Could be via BB or CC depending on processing order
        assert!(
            route.next_hop == Some(node_id(0xBB)) || route.next_hop == Some(node_id(0xCC))
        );
    }
}
