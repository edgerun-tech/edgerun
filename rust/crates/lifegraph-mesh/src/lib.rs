//! Core types for the Lifegraph identity-routed mesh network.
//!
//! Every node's identity IS its address: the raw ECDSA P-256 public key
//! (64 bytes, uncompressed x||y).  All mesh frames are signed by the
//! sender's secure hardware and verified against the sender's NodeID
//! embedded in the frame header.

use lifegraph_hardware_signing::{
    MESH_PUBLIC_KEY_LENGTH, MESH_SIGNATURE_LENGTH, NodeID,
};
use p256::ecdsa::Signature;
use p256::ecdsa::signature::hazmat::{PrehashSigner, PrehashVerifier};
use p256::ecdsa::VerifyingKey;

// ---------------------------------------------------------------------------
// Frame types
// ---------------------------------------------------------------------------

/// The type of a mesh frame, encoded as a single byte on the wire.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FrameType {
    /// Capability invocation, result, or session control payload.
    /// Encrypted through the session layer.
    Data = 0,
    /// Discovery hello — advertises this node's identity and routing table.
    Discovery = 1,
    /// Route advertisement — piggybacked on data frames or sent standalone.
    RouteAdv = 2,
    /// ECDH handshake init — sent by the initiator to start a session.
    /// Signed but NOT encrypted (no session exists yet).
    HandshakeInit = 3,
    /// ECDH handshake accept — sent by the responder to complete a session.
    /// Signed but NOT encrypted (session is being established).
    HandshakeAccept = 4,
    /// Unknown or unsupported frame type.
    Unknown(u8),
}

impl FrameType {
    #[must_use]
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Data,
            1 => Self::Discovery,
            2 => Self::RouteAdv,
            3 => Self::HandshakeInit,
            4 => Self::HandshakeAccept,
            other => Self::Unknown(other),
        }
    }
}

// ---------------------------------------------------------------------------
// Mesh frame header (130 bytes, fixed)
// ---------------------------------------------------------------------------

/// The fixed-size header prefix of every mesh frame.
///
/// Wire layout:
///   [0..64)   dest_node_id  — 64-byte ECDSA P-256 public key of the recipient
///   [64..128) src_node_id   — 64-byte ECDSA P-256 public key of the sender
///   [128]     ttl           — decremented at each hop, dropped at 0
///   [129]     frame_type    — Data / Discovery / RouteAdv
///
/// After the header comes the variable-length payload, followed by
/// a 64-byte ECDSA P-256 signature (r||s) over header+payload.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshFrameHeader {
    pub dest: NodeID,
    pub src: NodeID,
    pub ttl: u8,
    pub frame_type: FrameType,
}

impl MeshFrameHeader {
    pub const SIZE: usize = MESH_PUBLIC_KEY_LENGTH * 2 + 2; // 130

    /// Encode the header into exactly 130 bytes.
    #[must_use]
    pub fn encode(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];
        buf[..64].copy_from_slice(&self.dest.0);
        buf[64..128].copy_from_slice(&self.src.0);
        buf[128] = self.ttl;
        buf[129] = match self.frame_type {
            FrameType::Data => 0,
            FrameType::Discovery => 1,
            FrameType::RouteAdv => 2,
            FrameType::HandshakeInit => 3,
            FrameType::HandshakeAccept => 4,
            FrameType::Unknown(v) => v,
        };
        buf
    }

    /// Decode a header from exactly 130 bytes.
    pub fn decode(buf: &[u8; Self::SIZE]) -> Self {
        let mut dest = [0u8; MESH_PUBLIC_KEY_LENGTH];
        let mut src = [0u8; MESH_PUBLIC_KEY_LENGTH];
        dest.copy_from_slice(&buf[..64]);
        src.copy_from_slice(&buf[64..128]);
        Self {
            dest: NodeID(dest),
            src: NodeID(src),
            ttl: buf[128],
            frame_type: FrameType::from_u8(buf[129]),
        }
    }
}

impl core::fmt::Debug for MeshFrameHeader {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MeshFrameHeader")
            .field("dest", &self.dest)
            .field("src", &self.src)
            .field("ttl", &self.ttl)
            .field("frame_type", &self.frame_type)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Assembled frame (for in-memory use)
// ---------------------------------------------------------------------------

/// An assembled mesh frame ready for signing or verification.
///
/// The wire format is: `header_bytes || payload || signature`
/// The signature covers `header_bytes || payload`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshFrame {
    pub header: MeshFrameHeader,
    pub payload: Vec<u8>,
    pub signature: [u8; MESH_SIGNATURE_LENGTH],
}

impl MeshFrame {
    /// Returns the bytes over which the signature was computed
    /// (header encoding + payload).
    #[must_use]
    pub fn signed_preimage(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(MeshFrameHeader::SIZE + self.payload.len());
        out.extend_from_slice(&self.header.encode());
        out.extend_from_slice(&self.payload);
        out
    }

    /// Returns the full wire format: header || payload || signature.
    #[must_use]
    pub fn to_wire(&self) -> Vec<u8> {
        let mut out = self.signed_preimage();
        out.extend_from_slice(&self.signature);
        out
    }

    /// Parse from wire format. Returns `None` if the buffer is too short.
    pub fn from_wire(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < MeshFrameHeader::SIZE + MESH_SIGNATURE_LENGTH {
            return None;
        }
        let sig_start = bytes.len() - MESH_SIGNATURE_LENGTH;
        let header_bytes: [u8; MeshFrameHeader::SIZE] =
            bytes[..MeshFrameHeader::SIZE].try_into().ok()?;
        let mut signature = [0u8; MESH_SIGNATURE_LENGTH];
        signature.copy_from_slice(&bytes[sig_start..]);
        let payload = bytes[MeshFrameHeader::SIZE..sig_start].to_vec();
        Some(Self {
            header: MeshFrameHeader::decode(&header_bytes),
            payload,
            signature,
        })
    }

    /// Creates an unsigned mesh frame with the given destination and payload.
    /// The src, ttl, and frame_type are set to defaults.
    /// The frame must be signed before sending.
    #[must_use]
    pub fn from_payload(dest: NodeID, payload: Vec<u8>) -> Self {
        Self {
            header: MeshFrameHeader {
                dest,
                src: NodeID([0u8; 64]),
                ttl: 16,
                frame_type: FrameType::Data,
            },
            payload,
            signature: [0u8; MESH_SIGNATURE_LENGTH],
        }
    }

    /// Verifies the ECDSA P-256 signature against the sender's NodeID.
    ///
    /// The signature is computed over `header_bytes || payload` using
    /// SHA-256 as the digest.  The public key is reconstructed from
    /// the `src` field of the header (64 bytes, uncompressed x||y).
    ///
    /// Returns `true` if the signature is valid, `false` otherwise.
    #[must_use]
    pub fn verify_signature(&self) -> bool {
        // Build SEC1 encoded public key: 0x04 || x(32) || y(32)
        let mut sec1_key = [0u8; 65];
        sec1_key[0] = 0x04;
        sec1_key[1..].copy_from_slice(&self.header.src.0);

        // Parse the verifying key
        let vk = match VerifyingKey::from_sec1_bytes(&sec1_key) {
            Ok(v) => v,
            Err(_) => return false,
        };

        // Parse the signature (r||s, 64 bytes)
        let sig = match Signature::from_slice(&self.signature) {
            Ok(s) => s,
            Err(_) => return false,
        };

        // Hash the preimage and verify
        let preimage = self.signed_preimage();
        let digest = lifegraph_core::crypto::sha256(&preimage);
        vk.verify_prehash(&digest, &sig).is_ok()
    }
}

// ---------------------------------------------------------------------------
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
        if let Some(pos) = self.routes.iter().position(|r| r.destination == route.destination) {
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

// ---------------------------------------------------------------------------
// Signing helper
// ---------------------------------------------------------------------------

/// Signs a mesh frame with the given ECDSA P-256 signing key.
///
/// The signature covers the header bytes and payload (not the signature itself).
/// This should be called before `to_wire()`.
pub fn sign_frame(frame: &mut MeshFrame, signing_key: &p256::ecdsa::SigningKey) {
    let preimage = frame.signed_preimage();
    let digest = lifegraph_core::crypto::sha256(&preimage);
    let sig: Signature = signing_key.sign_prehash(&digest).expect("P-256 signing failed");
    frame.signature.copy_from_slice(&sig.to_bytes());
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

#[cfg(test)]
mod tests {
    use super::*;
    use p256::ecdsa::SigningKey;

    fn test_node_id(v: u8) -> NodeID {
        let mut bytes = [0u8; 64];
        bytes[0] = v;
        NodeID(bytes)
    }

    /// Creates a real P-256 keypair and returns (NodeID, signing_key).
    fn make_real_keypair() -> (NodeID, SigningKey) {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).unwrap();
        let signing_key = SigningKey::from_bytes(&bytes.into()).unwrap();
        let encoded = signing_key.verifying_key().to_encoded_point(false);
        let bytes = encoded.as_bytes();
        // Skip the 0x04 prefix, take x||y (64 bytes)
        let mut node_bytes = [0u8; 64];
        node_bytes.copy_from_slice(&bytes[1..65]);
        (NodeID(node_bytes), signing_key)
    }

    #[test]
    fn header_encode_decode_roundtrip() {
        let header = MeshFrameHeader {
            dest: test_node_id(0xAA),
            src: test_node_id(0xBB),
            ttl: 5,
            frame_type: FrameType::Discovery,
        };
        let encoded = header.encode();
        assert_eq!(encoded.len(), 130);
        let decoded = MeshFrameHeader::decode(&encoded);
        assert_eq!(decoded.dest, header.dest);
        assert_eq!(decoded.src, header.src);
        assert_eq!(decoded.ttl, 5);
        assert_eq!(decoded.frame_type, FrameType::Discovery);
    }

    #[test]
    fn frame_wire_roundtrip() {
        let header = MeshFrameHeader {
            dest: test_node_id(0x01),
            src: test_node_id(0x02),
            ttl: 10,
            frame_type: FrameType::Data,
        };
        let payload = b"hello mesh".to_vec();
        let sig = [0x42u8; MESH_SIGNATURE_LENGTH];
        let frame = MeshFrame {
            header,
            payload: payload.clone(),
            signature: sig,
        };
        let wire = frame.to_wire();
        let recovered = MeshFrame::from_wire(&wire).expect("parse should succeed");
        assert_eq!(recovered.header.dest, frame.header.dest);
        assert_eq!(recovered.header.src, frame.header.src);
        assert_eq!(recovered.payload, payload);
        assert_eq!(recovered.signature, sig);
    }

    #[test]
    fn frame_from_wire_rejects_too_short() {
        assert!(MeshFrame::from_wire(&[0u8; 50]).is_none());
    }

    #[test]
    fn routing_table_best_path_selection() {
        let a = test_node_id(0xAA);
        let b = test_node_id(0xBB);
        let c = test_node_id(0xCC);

        let mut table = MeshRoutingTable::default();
        table.update(MeshRoute {
            destination: b,
            next_hop: None,
            cost: 1,
        });
        table.update(MeshRoute {
            destination: c,
            next_hop: Some(b),
            cost: 2,
        });

        // Direct route to B
        let route_to_b = table.lookup(&b).expect("should find B");
        assert_eq!(route_to_b.cost, 1);
        assert!(route_to_b.next_hop.is_none());

        // C via B at cost 2
        let route_to_c = table.lookup(&c).expect("should find C");
        assert_eq!(route_to_c.cost, 2);
        assert_eq!(route_to_c.next_hop, Some(b));

        // Update with a worse route to C — should be rejected
        table.update(MeshRoute {
            destination: c,
            next_hop: Some(b),
            cost: 5,
        });
        let route_to_c = table.lookup(&c).unwrap();
        assert_eq!(route_to_c.cost, 2); // unchanged

        // Update with a better route to C — should replace
        table.update(MeshRoute {
            destination: c,
            next_hop: Some(a),
            cost: 1,
        });
        let route_to_c = table.lookup(&c).unwrap();
        assert_eq!(route_to_c.cost, 1);
        assert_eq!(route_to_c.next_hop, Some(a));
    }

    #[test]
    fn remove_via_clears_indirect_routes() {
        let b = test_node_id(0xBB);
        let c = test_node_id(0xCC);

        let mut table = MeshRoutingTable::default();
        table.update(MeshRoute {
            destination: b,
            next_hop: None,
            cost: 1,
        });
        table.update(MeshRoute {
            destination: c,
            next_hop: Some(b),
            cost: 2,
        });

        table.remove_via(&b);

        // Direct route to B is NOT via B (next_hop is None), so it stays
        assert!(table.lookup(&b).is_some());
        // Route to C via B is removed
        assert!(table.lookup(&c).is_none());
    }

    #[test]
    fn peer_dead_after_three_misses() {
        let peer = MeshPeer {
            node_id: test_node_id(0xDD),
            advertised_routes: Vec::new(),
            last_seen_unix: 1000,
            missed_heartbeats: 2,
        };
        assert!(!peer.is_dead());
        let mut dead = peer.clone();
        dead.missed_heartbeats = 3;
        assert!(dead.is_dead());
    }

    #[test]
    fn local_node_sequence_monotonically_increases() {
        let mut node = LocalNode::new(test_node_id(0xEE));
        assert_eq!(node.next_sequence(), 1);
        assert_eq!(node.next_sequence(), 2);
        assert_eq!(node.next_sequence(), 3);
    }

    #[test]
    fn frame_type_from_u8() {
        assert_eq!(FrameType::from_u8(0), FrameType::Data);
        assert_eq!(FrameType::from_u8(1), FrameType::Discovery);
        assert_eq!(FrameType::from_u8(2), FrameType::RouteAdv);
        assert_eq!(FrameType::from_u8(3), FrameType::HandshakeInit);
        assert_eq!(FrameType::from_u8(4), FrameType::HandshakeAccept);
        assert_eq!(FrameType::from_u8(99), FrameType::Unknown(99));
    }

    #[test]
    fn frame_sign_and_verify_with_real_p256_key() {
        let (node_id, signing_key) = make_real_keypair();
        let dest = test_node_id(0xFF);

        let header = MeshFrameHeader {
            dest,
            src: node_id,
            ttl: 10,
            frame_type: FrameType::Data,
        };
        let payload = b"hello mesh".to_vec();
        let mut frame = MeshFrame {
            header,
            payload: payload.clone(),
            signature: [0u8; MESH_SIGNATURE_LENGTH],
        };

        // Sign the frame
        sign_frame(&mut frame, &signing_key);

        // Verify on the wire format
        let wire = frame.to_wire();
        let recovered = MeshFrame::from_wire(&wire).expect("wire parse should succeed");
        assert!(recovered.verify_signature(), "signature should be valid");
        assert_eq!(recovered.payload, payload);
        assert_eq!(recovered.header.src, node_id);
        assert_eq!(recovered.header.dest, dest);
    }

    #[test]
    fn frame_verify_rejects_tampered_payload() {
        let (node_id, signing_key) = make_real_keypair();

        let header = MeshFrameHeader {
            dest: test_node_id(0xFF),
            src: node_id,
            ttl: 10,
            frame_type: FrameType::Data,
        };
        let mut frame = MeshFrame {
            header,
            payload: b"original".to_vec(),
            signature: [0u8; MESH_SIGNATURE_LENGTH],
        };
        sign_frame(&mut frame, &signing_key);

        // Tamper with the payload after signing
        frame.payload = b"tampered".to_vec();

        assert!(
            !frame.verify_signature(),
            "tampered payload should fail verification"
        );
    }

    #[test]
    fn frame_verify_rejects_wrong_sender_key() {
        let (real_node, real_key) = make_real_keypair();
        let (fake_node, fake_key) = make_real_keypair();

        // Sign with real key
        let header = MeshFrameHeader {
            dest: test_node_id(0xFF),
            src: fake_node, // But claim to be the fake node
            ttl: 10,
            frame_type: FrameType::Data,
        };
        let mut frame = MeshFrame {
            header,
            payload: b"data".to_vec(),
            signature: [0u8; MESH_SIGNATURE_LENGTH],
        };
        sign_frame(&mut frame, &fake_key);

        // This should verify because the src matches the key used
        assert!(frame.verify_signature());

        // Now forge a frame with real_node's ID but signed by fake_key
        let header2 = MeshFrameHeader {
            dest: test_node_id(0xFF),
            src: real_node, // Claim to be real_node
            ttl: 10,
            frame_type: FrameType::Data,
        };
        let mut fake_frame = MeshFrame {
            header: header2,
            payload: b"data".to_vec(),
            signature: [0u8; MESH_SIGNATURE_LENGTH],
        };
        sign_frame(&mut fake_frame, &fake_key);

        // This should fail because src is real_node but key is fake_key
        assert!(
            !fake_frame.verify_signature(),
            "spoofed sender should fail verification"
        );

        // Suppress unused variable warnings
        let _ = real_key;
    }
}
