use super::prelude::v1::*;
use edgerun_crypto::p256::PublicKey;
pub use edgerun_crypto::p256::ecdh::EphemeralSecret;
use edgerun_crypto::p256::elliptic_curve::sec1::ToEncodedPoint;
use edgerun_hardware_signing::NodeID;

use super::*;

/// Sent by the initiator to the responder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandshakeInit {
    /// Initiator's NodeID (public key, 64 bytes).
    pub initiator: NodeID,
    /// Initiator's ephemeral ECDH public key (65 bytes, SEC1 uncompressed).
    pub ephemeral_pub: [u8; ECDH_PUBLIC_KEY_SIZE],
}

/// Second message in the ECDH handshake.
/// Sent by the responder back to the initiator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandshakeAccept {
    /// Responder's NodeID (public key, 64 bytes).
    pub responder: NodeID,
    /// Responder's ephemeral ECDH public key (65 bytes, SEC1 uncompressed).
    pub ephemeral_pub: [u8; ECDH_PUBLIC_KEY_SIZE],
}

impl HandshakeInit {
    /// Serialize to exactly 129 bytes.
    #[must_use]
    pub fn encode(&self) -> [u8; HANDSHAKE_MSG_SIZE] {
        let mut buf = [0u8; HANDSHAKE_MSG_SIZE];
        buf[..64].copy_from_slice(&self.initiator.0);
        buf[64..].copy_from_slice(&self.ephemeral_pub);
        buf
    }

    /// Parse from exactly 129 bytes.
    pub fn decode(buf: &[u8]) -> Option<Self> {
        if buf.len() != HANDSHAKE_MSG_SIZE {
            return None;
        }
        let mut initiator = [0u8; 64];
        let mut ephemeral_pub = [0u8; ECDH_PUBLIC_KEY_SIZE];
        initiator.copy_from_slice(&buf[..64]);
        ephemeral_pub.copy_from_slice(&buf[64..]);
        Some(Self {
            initiator: NodeID(initiator),
            ephemeral_pub,
        })
    }
}

impl HandshakeAccept {
    /// Serialize to exactly 129 bytes.
    #[must_use]
    pub fn encode(&self) -> [u8; HANDSHAKE_MSG_SIZE] {
        let mut buf = [0u8; HANDSHAKE_MSG_SIZE];
        buf[..64].copy_from_slice(&self.responder.0);
        buf[64..].copy_from_slice(&self.ephemeral_pub);
        buf
    }

    /// Parse from exactly 129 bytes.
    pub fn decode(buf: &[u8]) -> Option<Self> {
        if buf.len() != HANDSHAKE_MSG_SIZE {
            return None;
        }
        let mut responder = [0u8; 64];
        let mut ephemeral_pub = [0u8; ECDH_PUBLIC_KEY_SIZE];
        responder.copy_from_slice(&buf[..64]);
        ephemeral_pub.copy_from_slice(&buf[64..]);
        Some(Self {
            responder: NodeID(responder),
            ephemeral_pub,
        })
    }
}
