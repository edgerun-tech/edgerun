//! Mesh session management — ECDH handshakes, session encryption, and replay protection.
use std::time::Duration;

mod session;
mod handshake;
mod manager;
mod error;

pub use session::MeshSession;
pub use handshake::{HandshakeInit, HandshakeAccept};
pub use manager::SessionManager;
pub use error::SessionError;

// Constants
// ---------------------------------------------------------------------------

/// Size of the ECDH public key (uncompressed SEC1 point, 65 bytes).
pub const ECDH_PUBLIC_KEY_SIZE: usize = 65;

/// Size of a serialized handshake message (NodeID + ECDH pubkey).
pub const HANDSHAKE_MSG_SIZE: usize = 129; // 64 + 65

/// Nonce size for AES-GCM.
const NONCE_SIZE: usize = 12;

/// HKDF info string for session key derivation.
const HKDF_INFO: &[u8] = b"edgerun-session-v1";

/// Maximum frames before forced rekey.
pub const MAX_FRAMES_BEFORE_REKEY: u64 = 1_000_000;

/// Maximum age before forced rekey.
pub const MAX_SESSION_AGE: Duration = Duration::from_secs(300); // 5 minutes


#[cfg(test)]
mod tests;
