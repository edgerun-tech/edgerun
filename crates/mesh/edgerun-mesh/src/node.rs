use edgerun_hardware_signing::{MESH_PUBLIC_KEY_LENGTH, MESH_SIGNATURE_LENGTH, NodeID};

use super::*;

// ---------------------------------------------------------------------------
// Signing helper
// ---------------------------------------------------------------------------

/// Signs a mesh frame with the given ECDSA P-256 signing key.
///
/// The signature uses domain separation: `SHA-256("edgerun:v0:sig:mesh-frame" || 0x00 || SHA-256(header_bytes || payload))`
/// This should be called before `to_wire()`.
pub fn sign_frame(frame: &mut MeshFrame, signing_key: &edgerun_crypto::P256SigningKey) {
    let preimage = frame.signed_preimage();
    let record_hash = edgerun_protocols::core_protocol::crypto::sha256(&preimage);
    let mut sig_input = Vec::with_capacity(22 + 1 + 32);
    sig_input.extend_from_slice(b"edgerun:v0:sig:mesh-frame");
    sig_input.push(0);
    sig_input.extend_from_slice(&record_hash);
    let full_digest = edgerun_protocols::core_protocol::crypto::sha256(&sig_input);
    let sig = signing_key
        .sign_prehash_fixed(&full_digest)
        .expect("P-256 signing failed");
    frame.signature.copy_from_slice(&sig);
}

// ---------------------------------------------------------------------------
// Peer discovery
// ---------------------------------------------------------------------------

/// A peer discovered via the multicast discovery protocol.
#[derive(Clone, Debug)]
pub struct MeshPeer {
    /// The peer's identity (ECDSA P-256 public key).
    pub node_id: NodeID,
    /// The peer's advertised routing table (learned from its hello packet).
    pub advertised_routes: Vec<MeshRoute>,
    /// Time this peer was last seen (seconds since UNIX epoch).
    pub last_seen_unix: i64,
    /// Number of consecutive missed heartbeats.
    pub missed_heartbeats: u8,
}

impl MeshPeer {
    /// Returns `true` if the peer should be considered dead.
    #[must_use]
    pub fn is_dead(&self) -> bool {
        self.missed_heartbeats >= 3
    }
}

// ---------------------------------------------------------------------------
// Discovery packet payload
// ---------------------------------------------------------------------------

/// The payload inside a `FrameType::Discovery` frame.
///
/// This is what every node multicasts periodically to announce its
/// presence and share its routing table.
#[derive(Clone, Debug)]
pub struct DiscoveryPayload {
    /// Monotonically increasing sequence number for this node's hello packets.
    pub sequence: u32,
    /// This node's current routing table (so peers can learn multi-hop paths).
    pub routes: Vec<MeshRoute>,
}

// ---------------------------------------------------------------------------
// Local identity
// ---------------------------------------------------------------------------

/// A node's local mesh identity, backed by secure hardware.
#[derive(Clone, Debug)]
pub struct LocalNode {
    /// This node's NodeID (ECDSA P-256 public key from hardware).
    pub node_id: NodeID,
    /// Monotonically increasing sequence counter for discovery packets.
    pub discovery_sequence: u32,
}

impl LocalNode {
    pub fn new(node_id: NodeID) -> Self {
        Self {
            node_id,
            discovery_sequence: 0,
        }
    }

    /// Increments and returns the next discovery sequence number.
    pub fn next_sequence(&mut self) -> u32 {
        self.discovery_sequence += 1;
        self.discovery_sequence
    }
}
use crate::prelude::v1::*;
