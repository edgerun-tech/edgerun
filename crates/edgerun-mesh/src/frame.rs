use edgerun_hardware_signing::{
    MESH_PUBLIC_KEY_LENGTH, MESH_SIGNATURE_LENGTH, NodeID,
};
use edgerun_crypto::p256::ecdsa::Signature;
use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashVerifier;
use edgerun_crypto::p256::ecdsa::VerifyingKey;

use super::*;

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
    /// The signature is computed over `SHA-256("edgerun:v0:sig:mesh-frame" || 0x00 || SHA-256(header_bytes || payload))`
    /// using SHA-256 as the digest.  The public key is reconstructed from
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

        // Verify with domain separation: SHA-256("edgerun:v0:sig:mesh-frame" || 0x00 || SHA-256(preimage))
        let preimage = self.signed_preimage();
        let record_hash = edgerun_core::crypto::sha256(&preimage);
        let mut sig_input = Vec::with_capacity(22 + 1 + 32); // domain tag + null + hash
        sig_input.extend_from_slice(b"edgerun:v0:sig:mesh-frame");
        sig_input.push(0);
        sig_input.extend_from_slice(&record_hash);
        let full_digest = edgerun_core::crypto::sha256(&sig_input);

        vk.verify_prehash(&full_digest, &sig).is_ok()
    }
}

// ---------------------------------------------------------------------------
