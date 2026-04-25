use crate::{
    FrameType, LocalNode, MeshFrame, MeshFrameHeader, MeshPeer, MeshRoute, MeshRoutingTable,
};
use edgerun_hardware_signing::{NodeID, MESH_SIGNATURE_LENGTH};

// ---------------------------------------------------------------------------
// Discovery packet (serialized payload)
// ---------------------------------------------------------------------------
use super::*;

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
