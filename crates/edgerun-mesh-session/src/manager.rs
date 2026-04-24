use edgerun_hardware_signing::NodeID;
pub use edgerun_crypto::p256::ecdh::EphemeralSecret;
use edgerun_crypto::p256::PublicKey;
use edgerun_crypto::p256::elliptic_curve::sec1::ToEncodedPoint;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::*;

// ---------------------------------------------------------------------------
// Session manager
// ---------------------------------------------------------------------------

/// Manages active sessions with mesh peers.
///
/// Handles:
/// - ECDH handshake completion (both initiator and responder roles)
/// - Frame encryption/decryption per session
/// - Automatic rekey when sessions age or exceed frame count
/// - Replay attack detection (per-session nonce counter tracking)
pub struct SessionManager {
    sessions: HashMap<NodeID, MeshSession>,
    /// Our NodeID (from hardware).
    our_node_id: NodeID,
}

impl SessionManager {
    pub fn new(our_node_id: NodeID) -> Self {
        Self {
            sessions: HashMap::new(),
            our_node_id,
        }
    }

    /// Returns our NodeID.
    pub fn our_node_id(&self) -> NodeID {
        self.our_node_id
    }

    /// Generate a random EphemeralSecret using OS randomness.
    pub fn random_ephemeral_secret() -> EphemeralSecret {
        EphemeralSecret::random(&mut edgerun_crypto::OsRng)
    }

    // -----------------------------------------------------------------------
    // Handshake: initiator side
    // -----------------------------------------------------------------------

    /// Starts a handshake as the initiator.
    ///
    /// Generates an ephemeral ECDH keypair and returns the `HandshakeInit`
    /// message to send to the responder. The private ephemeral key is kept
    /// internally until the handshake completes.
    pub fn initiate_handshake(&self, _peer: NodeID) -> (HandshakeInit, EphemeralSecret) {
        let secret = Self::random_ephemeral_secret();
        let pub_point = secret.public_key().to_encoded_point(false);
        let pub_bytes = pub_point.as_bytes();
        let mut ephemeral_pub = [0u8; ECDH_PUBLIC_KEY_SIZE];
        ephemeral_pub.copy_from_slice(pub_bytes);

        let msg = HandshakeInit {
            initiator: self.our_node_id,
            ephemeral_pub,
        };
        (msg, secret)
    }

    /// Completes the handshake after receiving `HandshakeAccept` from the peer.
    ///
    /// Derives the AES-256-GCM session key from the ECDH shared secret.
    pub fn complete_handshake_initiator(
        &mut self,
        accept: &HandshakeAccept,
        our_secret: &EphemeralSecret,
    ) -> Result<(), SessionError> {
        let their_pub = PublicKey::from_sec1_bytes(&accept.ephemeral_pub)
            .map_err(|_| SessionError::InvalidEcdhPublicKey)?;
        let shared = our_secret.diffie_hellman(&their_pub);
        let session_key = derive_session_key(shared.raw_secret_bytes());

        let session = MeshSession::new(accept.responder, session_key);
        self.sessions.insert(accept.responder, session);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Handshake: responder side
    // -----------------------------------------------------------------------

    /// Responds to a `HandshakeInit` from a peer.
    ///
    /// Generates an ephemeral keypair, derives the session key, and returns
    /// the `HandshakeAccept` message to send back.
    pub fn respond_to_handshake(
        &mut self,
        init: &HandshakeInit,
    ) -> Result<(HandshakeAccept, EphemeralSecret), SessionError> {
        let their_pub = PublicKey::from_sec1_bytes(&init.ephemeral_pub)
            .map_err(|_| SessionError::InvalidEcdhPublicKey)?;

        let our_secret = Self::random_ephemeral_secret();
        let our_pub_point = our_secret.public_key().to_encoded_point(false);
        let our_pub_bytes = our_pub_point.as_bytes();
        let mut ephemeral_pub = [0u8; ECDH_PUBLIC_KEY_SIZE];
        ephemeral_pub.copy_from_slice(our_pub_bytes);

        let shared = our_secret.diffie_hellman(&their_pub);
        let session_key = derive_session_key(shared.raw_secret_bytes());

        let session = MeshSession::new(init.initiator, session_key);
        self.sessions.insert(init.initiator, session);

        let accept = HandshakeAccept {
            responder: self.our_node_id,
            ephemeral_pub,
        };
        Ok((accept, our_secret))
    }

    // -----------------------------------------------------------------------
    // Data transfer
    // -----------------------------------------------------------------------

    /// Encrypts a plaintext for the given peer.
    /// Returns an error if no active session exists (handshake required).
    pub fn encrypt_for(
        &mut self,
        peer: NodeID,
        plaintext: &[u8],
    ) -> Result<Vec<u8>, SessionError> {
        let session = self
            .sessions
            .get_mut(&peer)
            .ok_or(SessionError::NoActiveSession)?;

        if session.needs_rekey() {
            return Err(SessionError::SessionExpired);
        }

        Ok(session.encrypt(plaintext))
    }

    /// Decrypts a ciphertext from the given peer.
    /// Returns an error if no active session exists or decryption fails.
    pub fn decrypt_from(
        &mut self,
        peer: NodeID,
        ciphertext: &[u8],
    ) -> Result<Vec<u8>, SessionError> {
        let session = self
            .sessions
            .get_mut(&peer)
            .ok_or(SessionError::NoActiveSession)?;

        session.decrypt(ciphertext)
    }

    /// Removes all sessions for the given peer.
    pub fn remove_peer(&mut self, peer: &NodeID) {
        self.sessions.remove(peer);
    }

    /// Returns the number of active sessions.
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }
}

// ---------------------------------------------------------------------------
// Key derivation
// ---------------------------------------------------------------------------

/// Derives a 32-byte AES-256 session key from an ECDH shared secret
/// using HKDF-SHA256.
pub(crate) fn derive_session_key(shared_secret: &[u8]) -> [u8; 32] {
    let key_bytes = edgerun_core::crypto::HkdfSha256::new(None, shared_secret)
        .expand(HKDF_INFO, 32);
    key_bytes.try_into().expect("HKDF expand failed")
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------
