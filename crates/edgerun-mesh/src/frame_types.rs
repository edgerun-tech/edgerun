use edgerun_hardware_signing::{
    MESH_PUBLIC_KEY_LENGTH, MESH_SIGNATURE_LENGTH, NodeID,
};
use edgerun_crypto::p256::ecdsa::Signature;
use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashVerifier;
use edgerun_crypto::p256::ecdsa::VerifyingKey;

use super::*;

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
