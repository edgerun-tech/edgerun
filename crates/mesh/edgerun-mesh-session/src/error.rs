use crate::prelude::v1::*;
pub use edgerun_crypto::p256::ecdh::EphemeralSecret;
use edgerun_crypto::p256::elliptic_curve::sec1::ToEncodedPoint;
use edgerun_crypto::p256::PublicKey;
use edgerun_hardware_signing::NodeID;

use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionError {
    /// No active session exists with the given peer (handshake required).
    NoActiveSession,
    /// Session has expired (too many frames or too old).
    SessionExpired,
    /// The ECDH public key in the handshake is invalid.
    InvalidEcdhPublicKey,
    /// AES-GCM decryption failed (wrong key, corrupted data, or replay).
    DecryptionFailed,
    /// Ciphertext is too short to contain a nonce.
    CiphertextTooShort,
    /// Replay attack detected: nonce counter went backwards or repeated.
    ReplayDetected,
    /// Replay-only inputs were supplied to a host-clock session manager.
    ReplayOnlyInput,
}

impl core::fmt::Display for SessionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoActiveSession => write!(f, "no active session with peer"),
            Self::SessionExpired => write!(f, "session expired, rekey required"),
            Self::InvalidEcdhPublicKey => write!(f, "invalid ECDH public key in handshake"),
            Self::DecryptionFailed => write!(f, "AES-GCM decryption failed"),
            Self::CiphertextTooShort => write!(f, "ciphertext too short"),
            Self::ReplayDetected => write!(f, "replay attack detected"),
            Self::ReplayOnlyInput => {
                write!(f, "replay-only input requires replayable session manager")
            }
        }
    }
}

impl core::error::Error for SessionError {}
