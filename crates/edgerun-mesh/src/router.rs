use crate::{
    FrameType, LocalNode, MeshFrame, MeshFrameHeader, MeshPeer, MeshRoute, MeshRoutingTable,
};
use edgerun_hardware_signing::{NodeID, MESH_SIGNATURE_LENGTH};

// ---------------------------------------------------------------------------
// Discovery packet (serialized payload)
// ---------------------------------------------------------------------------
use super::*;

/// The mesh router manages discovery, peer tracking, and route computation.
#[derive(Clone, Debug)]
pub struct MeshRouter {
    /// This node's local identity.
    pub(crate) local: LocalNode,
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
        let peer = self.peers.entry(from).or_insert_with(|| MeshPeer {
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
        self.routing_table.lookup(destination).and_then(|route| {
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
use crate::prelude::v1::*;
