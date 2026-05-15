use edgerun_hardware_signing::{MESH_PUBLIC_KEY_LENGTH, MESH_SIGNATURE_LENGTH, NodeID};

use super::*;

// Routing
// ---------------------------------------------------------------------------

/// A single entry in the mesh routing table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshRoute {
    /// The destination node's identity (its ECDSA P-256 public key).
    pub destination: NodeID,
    /// The next-hop node to forward frames towards the destination.
    /// `None` means the destination is directly reachable (cost 1).
    pub next_hop: Option<NodeID>,
    /// Total hop count to the destination.
    pub cost: u8,
}

/// The local node's mesh routing table.
///
/// Uses a simple best-path selection: lowest cost wins.
/// On equal cost, the entry that was updated most recently is kept
/// (insertion-order tiebreak).
#[derive(Clone, Debug, Default)]
pub struct MeshRoutingTable {
    routes: Vec<MeshRoute>,
}

impl MeshRoutingTable {
    /// Returns the best route to the given destination, if any.
    #[must_use]
    pub fn lookup(&self, destination: &NodeID) -> Option<&MeshRoute> {
        self.routes
            .iter()
            .filter(|r| r.destination == *destination)
            .min_by_key(|r| r.cost)
    }

    /// Returns the best route for forwarding (lowest cost), preferring
    /// direct routes when costs are equal.
    #[must_use]
    pub fn best_route(&self) -> Option<&MeshRoute> {
        self.routes.iter().min_by_key(|r| r.cost)
    }

    /// Inserts or updates a route.  If a route to the same destination
    /// already exists with equal or lower cost, it is left unchanged.
    /// If the new route has a lower cost, it replaces the existing one.
    pub fn update(&mut self, route: MeshRoute) {
        if let Some(pos) = self
            .routes
            .iter()
            .position(|r| r.destination == route.destination)
        {
            if route.cost < self.routes[pos].cost {
                self.routes[pos] = route;
            }
            // Equal or higher cost: keep existing route (fresher entry)
        } else {
            self.routes.push(route);
        }
    }

    /// Removes all routes through a given next-hop node (used when a peer
    /// becomes unreachable).
    pub fn remove_via(&mut self, via: &NodeID) {
        self.routes.retain(|r| r.next_hop.as_ref() != Some(via));
    }

    /// Removes the route to a specific destination.
    pub fn remove_destination(&mut self, destination: &NodeID) {
        self.routes.retain(|r| r.destination != *destination);
    }

    /// Returns all routes in the table.
    #[must_use]
    pub fn iter(&self) -> impl Iterator<Item = &MeshRoute> {
        self.routes.iter()
    }

    /// Returns the number of routes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.routes.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }
}
use crate::prelude::v1::*;
