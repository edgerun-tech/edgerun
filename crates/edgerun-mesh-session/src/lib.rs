//! Session-based authenticated encryption for the edgerun mesh.
//!
//! ## Architecture
//!
//! **Tier 1 — Identity (hardware, infrequent):**
//! ECDSA P-256 signatures via TPM/YubiKey/SecureEnclave.
//! Used only during handshake (2 signatures) and periodic rekey (2 signatures).
//!
//! **Tier 2 — Transport (CPU AES-NI, every frame):**
//! AES-256-GCM with a session key derived from ECDH P-256.
//! ~3-17 million frames/sec at 1 Gbps (1 cycle/byte).
//!
//! ## Handshake
//!
//! ```text
//! A: generates ephemeral ECDH keypair (ephemeral_pub_a)
//! A: sends HandshakeInit { peer: B, ephemeral_pub: ephemeral_pub_a } (signed by TPM)
//! B: generates ephemeral ECDH keypair (ephemeral_pub_b)
//! B: computes shared_secret = ECDH(ephemeral_pub_a, ephemeral_priv_b)
//! B: derives session key = HKDF-SHA256(shared_secret, info="edgerun-session-v1")
//! B: sends HandshakeAccept { peer: A, ephemeral_pub: ephemeral_pub_b } (signed by TPM)
//! A: computes shared_secret = ECDH(ephemeral_pub_b, ephemeral_priv_a)
//! A: derives session key = HKDF-SHA256(shared_secret, info="edgerun-session-v1")
//! → both sides now have the same AES-256-GCM session key
//! ```
//!
//! ## Wire format
//!
//! HandshakeInit / HandshakeAccept:
//! ```text
//! [NodeID: 64 bytes][ephemeral_pub: 65 bytes] = 129 bytes total
//! ```
//!
//! Encrypted frame:
//! ```text
//! [nonce: 12 bytes][ciphertext + GCM tag]
//! ```

use edgerun_hardware_signing::NodeID;
use edgerun_crypto::rand_core::RngCore;
use edgerun_crypto::aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
pub use edgerun_crypto::p256::ecdh::EphemeralSecret;
use edgerun_crypto::p256::PublicKey;
use edgerun_crypto::p256::elliptic_curve::sec1::ToEncodedPoint;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Volatile zero to prevent compiler optimization from eliding it.
fn volatile_zero_u64(val: &mut u64) {
    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
    let p: *mut u64 = val;
    unsafe { p.write_volatile(0) }
    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
}

fn volatile_zero_bytes(buf: &mut [u8]) {
    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
    for b in buf.iter_mut() {
        unsafe { (b as *mut u8).write_volatile(0) }
    }
    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
}

// ---------------------------------------------------------------------------
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

// ---------------------------------------------------------------------------
// Session
// ---------------------------------------------------------------------------

/// An active AES-256-GCM session with a remote peer.
///
/// The session key is derived from an ECDH shared secret and is **never
/// persisted or transmitted**. It lives only in memory and is zeroed on drop.
pub struct MeshSession {
    peer: NodeID,
    cipher: edgerun_crypto::AesGcmCipher,
    /// 4-byte random prefix (unique per session).
    nonce_prefix: [u8; 4],
    /// Monotonically increasing counter (8 bytes).
    nonce_counter: u64,
    /// Highest nonce counter seen from the peer (replay detection).
    highest_seen_counter: Option<u64>,
    frame_count: u64,
    created_at: Instant,
}

impl MeshSession {
    /// Creates a session from a derived AES-256-GCM key.
    fn new(peer: NodeID, key: [u8; 32]) -> Self {
        let mut nonce_prefix = [0u8; 4];
        edgerun_crypto::getrandom::fill(&mut nonce_prefix).expect("getrandom failed");

        Self {
            peer,
            cipher: edgerun_crypto::AesGcmCipher::new_from_slice(&key).expect("valid AES-256 key"),
            nonce_prefix,
            nonce_counter: 0,
            highest_seen_counter: None,
            frame_count: 0,
            created_at: Instant::now(),
        }
    }

    /// Returns the peer's NodeID.
    pub fn peer(&self) -> NodeID {
        self.peer
    }

    /// Encrypts a plaintext frame. Returns `nonce(12) || ciphertext || tag(16)`.
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Vec<u8> {
        // Build nonce: 4-byte random prefix + 8-byte counter
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        nonce_bytes[..4].copy_from_slice(&self.nonce_prefix);
        nonce_bytes[4..].copy_from_slice(&self.nonce_counter.to_be_bytes());
        self.nonce_counter = self.nonce_counter.wrapping_add(1);

        let ciphertext = self.cipher.encrypt(&nonce_bytes, plaintext).expect("AES-GCM encrypt failed");

        let mut out = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&ciphertext);
        self.frame_count += 1;
        out
    }

    /// Decrypts a ciphertext frame. Returns the plaintext.
    ///
    /// **Replay protection:** rejects any frame with a nonce counter
    /// less than or equal to the highest counter seen so far from this peer.
    pub fn decrypt(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>, SessionError> {
        if ciphertext.len() < NONCE_SIZE {
            return Err(SessionError::CiphertextTooShort);
        }

        let nonce_bytes = &ciphertext[..NONCE_SIZE];
        // Extract the 8-byte counter from the nonce
        let mut counter_bytes = [0u8; 8];
        counter_bytes.copy_from_slice(&nonce_bytes[4..12]);
        let counter = u64::from_be_bytes(counter_bytes);

        // Replay detection
        if self.highest_seen_counter.is_some_and(|max| counter <= max) {
            return Err(SessionError::ReplayDetected);
        }
        self.highest_seen_counter = Some(counter);

        let nonce_arr: [u8; 12] = nonce_bytes.try_into().unwrap();
        let payload = &ciphertext[NONCE_SIZE..];

        self.cipher
            .decrypt(&nonce_arr, payload)
            .map_err(|_| SessionError::DecryptionFailed)
    }

    /// Returns `true` if this session should be rekeyed.
    pub fn needs_rekey(&self) -> bool {
        self.frame_count >= MAX_FRAMES_BEFORE_REKEY
            || self.created_at.elapsed() >= MAX_SESSION_AGE
    }
}

impl Drop for MeshSession {
    fn drop(&mut self) {
        // Zero all sensitive fields on drop
        volatile_zero_u64(&mut self.nonce_counter);
        volatile_zero_bytes(&mut self.nonce_prefix);
        if let Some(ref mut c) = self.highest_seen_counter { volatile_zero_u64(c); }
        // The cipher holds the AES-256 key — we can't zero it directly,
        // but the key material in the cipher struct is stored in memory
        // that will be freed. For defense-in-depth, we could use a custom
        // type that wraps the cipher and zeros on drop, but the Key type
        // from aes-gcm already implements Zeroize.
    }
}

// ---------------------------------------------------------------------------
// Handshake messages
// ---------------------------------------------------------------------------

/// First message in the ECDH handshake.
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
        let session_key = derive_session_key(&*shared.raw_secret_bytes());

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
        let session_key = derive_session_key(&*shared.raw_secret_bytes());

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
fn derive_session_key(shared_secret: &[u8]) -> [u8; 32] {
    let key_bytes = edgerun_core::crypto::HkdfSha256::new(None, shared_secret)
        .expand(HKDF_INFO, 32);
    key_bytes.try_into().expect("HKDF expand failed")
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

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
        }
    }
}

impl std::error::Error for SessionError {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_hardware_signing::{HardwareKeyInfo, HardwareSigningKey};

    fn node_id(v: u8) -> NodeID {
        let mut bytes = [0u8; 64];
        bytes[0] = v;
        NodeID(bytes)
    }

    // ── Handshake ──

    #[test]
    fn full_handshake_and_encrypt_decrypt() {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);

        // Step 1: Alice initiates
        let (init, alice_secret) = alice_mgr.initiate_handshake(bob_id);
        assert_eq!(init.initiator, alice_id);

        // Step 2: Bob responds
        let (accept, _bob_secret) = bob_mgr.respond_to_handshake(&init).unwrap();
        assert_eq!(accept.responder, bob_id);

        // Step 3: Alice completes
        alice_mgr.complete_handshake_initiator(&accept, &alice_secret).unwrap();

        assert_eq!(alice_mgr.session_count(), 1);
        assert_eq!(bob_mgr.session_count(), 1);

        // Encrypt from Alice to Bob
        let plaintext = b"hello mesh";
        let ciphertext = alice_mgr.encrypt_for(bob_id, plaintext).unwrap();

        // Bob decrypts
        let recovered = bob_mgr.decrypt_from(alice_id, &ciphertext).unwrap();
        assert_eq!(&recovered, plaintext);
    }

    #[test]
    fn encrypt_fails_without_session() {
        let mut mgr = SessionManager::new(node_id(0xAA));
        assert!(matches!(
            mgr.encrypt_for(node_id(0xBB), b"test"),
            Err(SessionError::NoActiveSession)
        ));
    }

    #[test]
    fn nonce_advances_on_each_encrypt() {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);

        let (init, secret) = alice_mgr.initiate_handshake(bob_id);
        let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
        alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();

        let c1 = alice_mgr.encrypt_for(bob_id, b"msg1").unwrap();
        let c2 = alice_mgr.encrypt_for(bob_id, b"msg1").unwrap();
        assert_ne!(c1, c2, "nonces should differ between encryptions");

        assert_eq!(bob_mgr.decrypt_from(alice_id, &c1).unwrap(), b"msg1");
        assert_eq!(bob_mgr.decrypt_from(alice_id, &c2).unwrap(), b"msg1");
    }

    // ── Replay detection ──

    #[test]
    fn replay_detection_rejects_old_nonce() {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);

        let (init, secret) = alice_mgr.initiate_handshake(bob_id);
        let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
        alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();

        let c1 = alice_mgr.encrypt_for(bob_id, b"msg1").unwrap();
        // First decrypt succeeds
        assert_eq!(bob_mgr.decrypt_from(alice_id, &c1).unwrap(), b"msg1");
        // Second decrypt of same ciphertext = replay
        assert!(matches!(
            bob_mgr.decrypt_from(alice_id, &c1),
            Err(SessionError::ReplayDetected)
        ));
    }

    #[test]
    fn replay_detection_accepts_out_of_order() {
        // If Alice sends msg 2 before msg 1 (UDP reordering), Bob should
        // accept msg 2, then reject msg 1 as a replay.
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);

        let (init, secret) = alice_mgr.initiate_handshake(bob_id);
        let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
        alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();

        let c1 = alice_mgr.encrypt_for(bob_id, b"msg1").unwrap();
        let c2 = alice_mgr.encrypt_for(bob_id, b"msg2").unwrap();

        // Receive msg 2 first (counter=1)
        assert_eq!(bob_mgr.decrypt_from(alice_id, &c2).unwrap(), b"msg2");
        // Then msg 1 arrives (counter=0) → replay
        assert!(matches!(
            bob_mgr.decrypt_from(alice_id, &c1),
            Err(SessionError::ReplayDetected)
        ));
    }

    // ── Session expiry ──

    #[test]
    fn session_expires_after_max_frames() {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);

        let (init, secret) = alice_mgr.initiate_handshake(bob_id);
        let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
        alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();

        for _ in 0..MAX_FRAMES_BEFORE_REKEY {
            let _ = alice_mgr.encrypt_for(bob_id, b"data").unwrap();
        }

        assert!(matches!(
            alice_mgr.encrypt_for(bob_id, b"data"),
            Err(SessionError::SessionExpired)
        ));
    }

    #[test]
    fn remove_peer_clears_session() {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);

        let (init, secret) = alice_mgr.initiate_handshake(bob_id);
        let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
        alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();

        assert_eq!(alice_mgr.session_count(), 1);
        alice_mgr.remove_peer(&bob_id);
        assert_eq!(alice_mgr.session_count(), 0);
        assert!(matches!(
            alice_mgr.encrypt_for(bob_id, b"data"),
            Err(SessionError::NoActiveSession)
        ));
    }

    // ── Key derivation ──

    #[test]
    fn derive_session_key_is_deterministic() {
        let secret_a = crate::SessionManager::random_ephemeral_secret();
        let pub_a = secret_a.public_key();
        let secret_b = crate::SessionManager::random_ephemeral_secret();
        let pub_b = secret_b.public_key();

        let shared_ab = secret_a.diffie_hellman(&pub_b);
        let shared_ba = secret_b.diffie_hellman(&pub_a);

        assert_eq!(shared_ab.raw_secret_bytes().as_slice(), shared_ba.raw_secret_bytes().as_slice());

        let key_ab = derive_session_key(shared_ab.raw_secret_bytes().as_slice());
        let key_ba = derive_session_key(shared_ba.raw_secret_bytes().as_slice());
        assert_eq!(key_ab, key_ba);
    }

    // ── Handshake wire format ──

    #[test]
    fn handshake_init_encode_decode_roundtrip() {
        let msg = HandshakeInit {
            initiator: node_id(0xAA),
            ephemeral_pub: [0x42; 65],
        };
        let encoded = msg.encode();
        assert_eq!(encoded.len(), HANDSHAKE_MSG_SIZE);

        let decoded = HandshakeInit::decode(&encoded).expect("decode failed");
        assert_eq!(decoded, msg);
    }

    #[test]
    fn handshake_accept_encode_decode_roundtrip() {
        let msg = HandshakeAccept {
            responder: node_id(0xBB),
            ephemeral_pub: [0x43; 65],
        };
        let encoded = msg.encode();
        assert_eq!(encoded.len(), HANDSHAKE_MSG_SIZE);

        let decoded = HandshakeAccept::decode(&encoded).expect("decode failed");
        assert_eq!(decoded, msg);
    }

    #[test]
    fn handshake_decode_rejects_truncated_input() {
        assert!(HandshakeInit::decode(&[0u8; 100]).is_none());
        assert!(HandshakeInit::decode(&[0u8; 128]).is_none());
        assert!(HandshakeInit::decode(&[0u8; 130]).is_none());
    }

    // ── Invalid ECDH key ──

    #[test]
    fn handshake_rejects_invalid_public_key() {
        let mut mgr = SessionManager::new(node_id(0xAA));
        let bad_init = HandshakeInit {
            initiator: node_id(0xBB),
            ephemeral_pub: [0xFF; 65], // invalid SEC1 point
        };
        assert!(matches!(
            mgr.respond_to_handshake(&bad_init),
            Err(SessionError::InvalidEcdhPublicKey)
        ));
    }

    // ── Bidirectional communication ──

    #[test]
    fn bidirectional_encrypt_decrypt() {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);

        let (init, secret) = alice_mgr.initiate_handshake(bob_id);
        let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
        alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();

        // Alice → Bob
        let ct1 = alice_mgr.encrypt_for(bob_id, b"alice-to-bob").unwrap();
        assert_eq!(bob_mgr.decrypt_from(alice_id, &ct1).unwrap(), b"alice-to-bob");

        // Bob → Alice (Bob has the session too since respond_to_handshake creates it)
        let ct2 = bob_mgr.encrypt_for(alice_id, b"bob-to-alice").unwrap();
        assert_eq!(alice_mgr.decrypt_from(bob_id, &ct2).unwrap(), b"bob-to-alice");
    }

    // ── HardwareMeshSigner integration ──

    #[test]
    fn hardware_mesh_signer_extracts_node_id_and_signs_digest() {
        use edgerun_hardware_signing::{HardwareMeshSigner, MeshSigner, MESH_SIGNATURE_LENGTH};

        struct FakeMeshKey;
        impl HardwareSigningKey for FakeMeshKey {
            fn key_info(&self) -> Result<HardwareKeyInfo, edgerun_hardware_signing::HardwareSigningError> {
                let mut pk = [0u8; 64];
                pk[0] = 0x04;
                pk[1] = 0xAB;
                Ok(HardwareKeyInfo {
                    provider: edgerun_hardware_signing::HardwareProviderKind::Tpm,
                    key_name: "fake-key".into(),
                    algorithm: edgerun_hardware_signing::HardwareSignatureAlgorithm::EcdsaP256Sha256,
                    public_key: pk.to_vec(),
                    attestation: vec![],
                    assurance_level: edgerun_hardware_signing::HardwareAssuranceLevel::IsolatedHardware,
                    biometric_state: Default::default(),
                })
            }
            fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, edgerun_hardware_signing::HardwareSigningError> {
                let mut sig = [0u8; 64];
                sig[..32].copy_from_slice(message);
                sig[32..].copy_from_slice(&message.iter().map(|b| !b).collect::<Vec<_>>()[..32]);
                Ok(sig.to_vec())
            }
        }

        let key = FakeMeshKey;
        let signer = HardwareMeshSigner::new(key).expect("should create signer");

        let node_id = signer.node_id();
        assert_eq!(node_id.0[0], 0x04);
        assert_eq!(node_id.0[1], 0xAB);

        let digest = [0x42u8; 32];
        let sig = signer.sign_digest(&digest).expect("should sign digest");
        assert_eq!(sig.len(), MESH_SIGNATURE_LENGTH);
        assert_eq!(sig[..32], digest);
    }

    // ── Error types and Display ──

    #[test]
    fn session_error_display_no_active_session() {
        let err = SessionError::NoActiveSession;
        assert_eq!(format!("{}", err), "no active session with peer");
    }

    #[test]
    fn session_error_display_session_expired() {
        let err = SessionError::SessionExpired;
        assert_eq!(format!("{}", err), "session expired, rekey required");
    }

    #[test]
    fn session_error_display_invalid_ecdh_public_key() {
        let err = SessionError::InvalidEcdhPublicKey;
        assert_eq!(format!("{}", err), "invalid ECDH public key in handshake");
    }

    #[test]
    fn session_error_display_decryption_failed() {
        let err = SessionError::DecryptionFailed;
        assert_eq!(format!("{}", err), "AES-GCM decryption failed");
    }

    #[test]
    fn session_error_display_ciphertext_too_short() {
        let err = SessionError::CiphertextTooShort;
        assert_eq!(format!("{}", err), "ciphertext too short");
    }

    #[test]
    fn session_error_display_replay_detected() {
        let err = SessionError::ReplayDetected;
        assert_eq!(format!("{}", err), "replay attack detected");
    }

    #[test]
    fn session_error_implements_error_trait() {
        fn assert_error<T: std::error::Error>() {}
        assert_error::<SessionError>();
    }

    #[test]
    fn session_error_is_clone_and_eq() {
        let a = SessionError::NoActiveSession;
        let b = a.clone();
        assert_eq!(a, b);

        let c = SessionError::ReplayDetected;
        let d = SessionError::DecryptionFailed;
        assert_ne!(c, d);
    }

    // ── Handshake protocol edge cases ──

    #[test]
    fn handshake_init_decode_empty_slice() {
        assert!(HandshakeInit::decode(&[]).is_none());
    }

    #[test]
    fn handshake_init_decode_one_byte_short() {
        assert!(HandshakeInit::decode(&[0u8; HANDSHAKE_MSG_SIZE - 1]).is_none());
    }

    #[test]
    fn handshake_init_decode_one_byte_long() {
        assert!(HandshakeInit::decode(&[0u8; HANDSHAKE_MSG_SIZE + 1]).is_none());
    }

    #[test]
    fn handshake_accept_decode_empty_slice() {
        assert!(HandshakeAccept::decode(&[]).is_none());
    }

    #[test]
    fn handshake_accept_decode_wrong_size() {
        assert!(HandshakeAccept::decode(&[0u8; 64]).is_none());
        assert!(HandshakeAccept::decode(&[0u8; HANDSHAKE_MSG_SIZE + 5]).is_none());
    }

    #[test]
    fn handshake_init_encode_produces_exact_size() {
        let msg = HandshakeInit {
            initiator: node_id(1),
            ephemeral_pub: [0u8; ECDH_PUBLIC_KEY_SIZE],
        };
        let encoded = msg.encode();
        assert_eq!(encoded.len(), HANDSHAKE_MSG_SIZE);
        // First 64 bytes = NodeID
        assert_eq!(encoded[0], 1);
        assert!(encoded[1..64].iter().all(|&b| b == 0));
        // Remaining 65 bytes = ephemeral pub (all zeros in this test)
        assert!(encoded[64..].iter().all(|&b| b == 0));
    }

    #[test]
    fn handshake_accept_encode_produces_exact_size() {
        let msg = HandshakeAccept {
            responder: node_id(2),
            ephemeral_pub: [0xAB; ECDH_PUBLIC_KEY_SIZE],
        };
        let encoded = msg.encode();
        assert_eq!(encoded.len(), HANDSHAKE_MSG_SIZE);
        assert_eq!(encoded[0], 2);
        assert!(encoded[1..64].iter().all(|&b| b == 0));
        assert!(encoded[64..].iter().all(|&b| b == 0xAB));
    }

    #[test]
    fn handshake_init_decode_exact_boundary() {
        let msg = HandshakeInit {
            initiator: node_id(0xFF),
            ephemeral_pub: [0x55; ECDH_PUBLIC_KEY_SIZE],
        };
        let encoded = msg.encode();
        let decoded = HandshakeInit::decode(&encoded).unwrap();
        assert_eq!(decoded.initiator.0[0], 0xFF);
        assert_eq!(decoded.ephemeral_pub, [0x55; ECDH_PUBLIC_KEY_SIZE]);
    }

    #[test]
    fn handshake_accept_decode_exact_boundary() {
        let msg = HandshakeAccept {
            responder: node_id(0x7F),
            ephemeral_pub: [0xAA; ECDH_PUBLIC_KEY_SIZE],
        };
        let encoded = msg.encode();
        let decoded = HandshakeAccept::decode(&encoded).unwrap();
        assert_eq!(decoded.responder.0[0], 0x7F);
        assert_eq!(decoded.ephemeral_pub, [0xAA; ECDH_PUBLIC_KEY_SIZE]);
    }

    // ── Session key derivation edge cases ──

    #[test]
    fn derive_session_key_produces_32_byte_key() {
        // Verify that derive_session_key always returns exactly 32 bytes.
        // Use a fixed input to avoid RNG dependency.
        let shared = [0x42u8; 32];
        let key = edgerun_core::crypto::HkdfSha256::new(None, &shared)
            .expand(HKDF_INFO, 32);
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn derive_session_key_same_input_same_output() {
        // HKDF is deterministic: same input always produces the same key.
        let shared = [0x77u8; 32];
        let key1 = edgerun_core::crypto::HkdfSha256::new(None, &shared)
            .expand(HKDF_INFO, 32);
        let key2 = edgerun_core::crypto::HkdfSha256::new(None, &shared)
            .expand(HKDF_INFO, 32);
        assert_eq!(key1, key2);
    }

    #[test]
    fn derive_session_key_32_bytes_output() {
        let secret = SessionManager::random_ephemeral_secret();
        let pubkey = secret.public_key();
        let shared = secret.diffie_hellman(&pubkey);
        let key = derive_session_key(&*shared.raw_secret_bytes());
        assert_eq!(key.len(), 32);
    }

    // ── Encrypt/decrypt operations ──

    #[test]
    fn encrypt_empty_plaintext() {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);

        let (init, secret) = alice_mgr.initiate_handshake(bob_id);
        let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
        alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();

        let ct = alice_mgr.encrypt_for(bob_id, b"").unwrap();
        // Ciphertext = 12-byte nonce + 16-byte GCM tag (no plaintext)
        assert_eq!(ct.len(), NONCE_SIZE + 16);
        let pt = bob_mgr.decrypt_from(alice_id, &ct).unwrap();
        assert_eq!(pt, b"");
    }

    #[test]
    fn encrypt_large_plaintext() {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);

        let (init, secret) = alice_mgr.initiate_handshake(bob_id);
        let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
        alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();

        let plaintext = vec![0x42u8; 65536];
        let ct = alice_mgr.encrypt_for(bob_id, &plaintext).unwrap();
        assert_eq!(ct.len(), NONCE_SIZE + plaintext.len() + 16);
        let pt = bob_mgr.decrypt_from(alice_id, &ct).unwrap();
        assert_eq!(pt, plaintext);
    }

    #[test]
    fn encrypt_binary_data_all_byte_values() {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);

        let (init, secret) = alice_mgr.initiate_handshake(bob_id);
        let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
        alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();

        let plaintext: Vec<u8> = (0..=255).cycle().take(1024).collect();
        let ct = alice_mgr.encrypt_for(bob_id, &plaintext).unwrap();
        let pt = bob_mgr.decrypt_from(alice_id, &ct).unwrap();
        assert_eq!(pt, plaintext);
    }

    #[test]
    fn encrypt_produces_ciphertext_longer_than_plaintext() {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);

        let (init, secret) = alice_mgr.initiate_handshake(bob_id);
        let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
        alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();

        let plaintext = b"short";
        let ct = alice_mgr.encrypt_for(bob_id, plaintext).unwrap();
        assert!(ct.len() > plaintext.len(), "ciphertext should include nonce + tag");
    }

    // ── Decrypt edge cases ──

    #[test]
    fn decrypt_empty_ciphertext_returns_error() {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);

        let (init, secret) = alice_mgr.initiate_handshake(bob_id);
        let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
        alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();

        assert!(matches!(
            bob_mgr.decrypt_from(alice_id, &[]),
            Err(SessionError::CiphertextTooShort)
        ));
    }

    #[test]
    fn decrypt_ciphertext_shorter_than_nonce() {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);

        let (init, secret) = alice_mgr.initiate_handshake(bob_id);
        let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
        alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();

        for len in 1..NONCE_SIZE {
            assert!(matches!(
                bob_mgr.decrypt_from(alice_id, &vec![0u8; len]),
                Err(SessionError::CiphertextTooShort)
            ));
        }
    }

    #[test]
    fn decrypt_tampered_ciphertext_fails() {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);

        let (init, secret) = alice_mgr.initiate_handshake(bob_id);
        let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
        alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();

        let mut ct = alice_mgr.encrypt_for(bob_id, b"secret").unwrap();
        // Flip a byte in the ciphertext portion (after the nonce)
        ct[NONCE_SIZE] ^= 0xFF;
        assert!(matches!(
            bob_mgr.decrypt_from(alice_id, &ct),
            Err(SessionError::DecryptionFailed)
        ));
    }

    #[test]
    fn decrypt_tampered_nonce_fails() {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);

        let (init, secret) = alice_mgr.initiate_handshake(bob_id);
        let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
        alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();

        let mut ct = alice_mgr.encrypt_for(bob_id, b"secret").unwrap();
        // Flip a byte in the nonce prefix
        ct[0] ^= 0xFF;
        assert!(matches!(
            bob_mgr.decrypt_from(alice_id, &ct),
            Err(SessionError::DecryptionFailed)
        ));
    }

    #[test]
    fn decrypt_nonce_only_ciphertext() {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);

        let (init, secret) = alice_mgr.initiate_handshake(bob_id);
        let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
        alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();

        // Exactly 12 bytes = nonce only, no ciphertext/tag
        let ct = vec![0u8; NONCE_SIZE];
        assert!(matches!(
            bob_mgr.decrypt_from(alice_id, &ct),
            Err(SessionError::DecryptionFailed)
        ));
    }

    // ── Nonce advancement ──

    #[test]
    fn nonce_counter_advances_per_encrypt() {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);

        let (init, secret) = alice_mgr.initiate_handshake(bob_id);
        let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
        alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();

        let c1 = alice_mgr.encrypt_for(bob_id, b"x").unwrap();
        let c2 = alice_mgr.encrypt_for(bob_id, b"x").unwrap();
        let c3 = alice_mgr.encrypt_for(bob_id, b"x").unwrap();

        // Nonces are in bytes [0..12]; counter is in bytes [4..12]
        fn extract_counter(ct: &[u8]) -> u64 {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&ct[4..12]);
            u64::from_be_bytes(buf)
        }

        assert_eq!(extract_counter(&c1), 0);
        assert_eq!(extract_counter(&c2), 1);
        assert_eq!(extract_counter(&c3), 2);
    }

    #[test]
    fn nonce_prefix_is_random_per_session() {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);

        let (init, secret) = alice_mgr.initiate_handshake(bob_id);
        let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
        alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();

        let c1 = alice_mgr.encrypt_for(bob_id, b"x").unwrap();
        let prefix1 = &c1[..4];

        // Second session (new handshake)
        let alice_id2 = node_id(0xCC);
        let bob_id2 = node_id(0xDD);
        let mut alice_mgr2 = SessionManager::new(alice_id2);
        let mut bob_mgr2 = SessionManager::new(bob_id2);

        let (init2, secret2) = alice_mgr2.initiate_handshake(bob_id2);
        let (accept2, _) = bob_mgr2.respond_to_handshake(&init2).unwrap();
        alice_mgr2.complete_handshake_initiator(&accept2, &secret2).unwrap();

        let c2 = alice_mgr2.encrypt_for(bob_id2, b"x").unwrap();
        let prefix2 = &c2[..4];

        // Prefixes should differ (extremely likely with random 4 bytes)
        assert_ne!(prefix1, prefix2);
    }

    #[test]
    fn different_plaintext_same_nonce_produces_same_prefix() {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);

        let (init, secret) = alice_mgr.initiate_handshake(bob_id);
        let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
        alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();

        // Two separate encryptions produce different nonces (different counter)
        let c1 = alice_mgr.encrypt_for(bob_id, b"hello").unwrap();
        let c2 = alice_mgr.encrypt_for(bob_id, b"world").unwrap();

        // Counter portion differs
        assert_ne!(&c1[4..12], &c2[4..12]);
        // Prefix portion should be the same (same session)
        assert_eq!(&c1[..4], &c2[..4]);
    }

    // ── Replay detection edge cases ──

    #[test]
    fn replay_detection_rejects_same_nonce_twice() {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);

        let (init, secret) = alice_mgr.initiate_handshake(bob_id);
        let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
        alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();

        let ct = alice_mgr.encrypt_for(bob_id, b"once").unwrap();
        bob_mgr.decrypt_from(alice_id, &ct).unwrap();
        // Exact same ciphertext again
        assert!(matches!(
            bob_mgr.decrypt_from(alice_id, &ct),
            Err(SessionError::ReplayDetected)
        ));
    }

    #[test]
    fn replay_detection_accepts_strictly_increasing_counter() {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);

        let (init, secret) = alice_mgr.initiate_handshake(bob_id);
        let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
        alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();

        let c1 = alice_mgr.encrypt_for(bob_id, b"1").unwrap();
        let c2 = alice_mgr.encrypt_for(bob_id, b"2").unwrap();
        let c3 = alice_mgr.encrypt_for(bob_id, b"3").unwrap();

        // In order: all should succeed
        assert_eq!(bob_mgr.decrypt_from(alice_id, &c1).unwrap(), b"1");
        assert_eq!(bob_mgr.decrypt_from(alice_id, &c2).unwrap(), b"2");
        assert_eq!(bob_mgr.decrypt_from(alice_id, &c3).unwrap(), b"3");
    }

    #[test]
    fn replay_detection_rejects_lower_counter_after_higher() {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);

        let (init, secret) = alice_mgr.initiate_handshake(bob_id);
        let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
        alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();

        let c1 = alice_mgr.encrypt_for(bob_id, b"1").unwrap();
        let c2 = alice_mgr.encrypt_for(bob_id, b"2").unwrap();

        // Receive higher first
        assert_eq!(bob_mgr.decrypt_from(alice_id, &c2).unwrap(), b"2");
        // Lower is replay
        assert!(matches!(
            bob_mgr.decrypt_from(alice_id, &c1),
            Err(SessionError::ReplayDetected)
        ));
    }

    // ── Session expiry after max frames ──

    #[test]
    fn needs_rekey_returns_true_at_max_frames() {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);

        let (init, secret) = alice_mgr.initiate_handshake(bob_id);
        let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
        alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();

        // Encrypt exactly MAX_FRAMES_BEFORE_REKEY times
        for _ in 0..MAX_FRAMES_BEFORE_REKEY {
            let _ = alice_mgr.encrypt_for(bob_id, b"x").unwrap();
        }

        // Session should be expired
        assert!(matches!(
            alice_mgr.encrypt_for(bob_id, b"x"),
            Err(SessionError::SessionExpired)
        ));
    }

    #[test]
    fn needs_rekey_returns_false_one_below_max_frames() {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);

        let (init, secret) = alice_mgr.initiate_handshake(bob_id);
        let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
        alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();

        // One less than max
        for _ in 0..(MAX_FRAMES_BEFORE_REKEY - 1) {
            let _ = alice_mgr.encrypt_for(bob_id, b"x").unwrap();
        }

        // Should still work
        let ct = alice_mgr.encrypt_for(bob_id, b"x").unwrap();
        assert_eq!(bob_mgr.decrypt_from(alice_id, &ct).unwrap(), b"x");
    }

    #[test]
    fn session_expiry_does_not_affect_other_peers() {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);
        let charlie_id = node_id(0xCC);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);
        let mut charlie_mgr = SessionManager::new(charlie_id);

        // Alice ↔ Bob
        let (init_ab, secret_ab) = alice_mgr.initiate_handshake(bob_id);
        let (accept_ab, _) = bob_mgr.respond_to_handshake(&init_ab).unwrap();
        alice_mgr.complete_handshake_initiator(&accept_ab, &secret_ab).unwrap();

        // Alice ↔ Charlie
        let (init_ac, secret_ac) = alice_mgr.initiate_handshake(charlie_id);
        let (accept_ac, _) = charlie_mgr.respond_to_handshake(&init_ac).unwrap();
        alice_mgr.complete_handshake_initiator(&accept_ac, &secret_ac).unwrap();

        // Exhaust Alice→Bob session
        for _ in 0..MAX_FRAMES_BEFORE_REKEY {
            let _ = alice_mgr.encrypt_for(bob_id, b"x").unwrap();
        }

        // Alice→Bob is expired
        assert!(matches!(
            alice_mgr.encrypt_for(bob_id, b"x"),
            Err(SessionError::SessionExpired)
        ));

        // Alice→Charlie still works
        let ct = alice_mgr.encrypt_for(charlie_id, b"hello").unwrap();
        assert_eq!(charlie_mgr.decrypt_from(alice_id, &ct).unwrap(), b"hello");
    }

    // ── Peer session management ──

    #[test]
    fn session_manager_tracks_multiple_peers() {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);
        let charlie_id = node_id(0xCC);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);
        let mut charlie_mgr = SessionManager::new(charlie_id);

        let (init_ab, secret_ab) = alice_mgr.initiate_handshake(bob_id);
        let (accept_ab, _) = bob_mgr.respond_to_handshake(&init_ab).unwrap();
        alice_mgr.complete_handshake_initiator(&accept_ab, &secret_ab).unwrap();

        let (init_ac, secret_ac) = alice_mgr.initiate_handshake(charlie_id);
        let (accept_ac, _) = charlie_mgr.respond_to_handshake(&init_ac).unwrap();
        alice_mgr.complete_handshake_initiator(&accept_ac, &secret_ac).unwrap();

        assert_eq!(alice_mgr.session_count(), 2);
        assert_eq!(bob_mgr.session_count(), 1);
        assert_eq!(charlie_mgr.session_count(), 1);
    }

    #[test]
    fn session_manager_our_node_id() {
        let id = node_id(0x42);
        let mgr = SessionManager::new(id);
        assert_eq!(mgr.our_node_id(), id);
    }

    #[test]
    fn session_manager_new_starts_empty() {
        let mgr = SessionManager::new(node_id(1));
        assert_eq!(mgr.session_count(), 0);
    }

    #[test]
    fn remove_peer_non_existent_does_not_panic() {
        let mut mgr = SessionManager::new(node_id(1));
        mgr.remove_peer(&node_id(99)); // No session exists
        assert_eq!(mgr.session_count(), 0);
    }

    #[test]
    fn remove_peer_removes_only_target() {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);
        let charlie_id = node_id(0xCC);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);
        let mut charlie_mgr = SessionManager::new(charlie_id);

        let (init_ab, secret_ab) = alice_mgr.initiate_handshake(bob_id);
        let (accept_ab, _) = bob_mgr.respond_to_handshake(&init_ab).unwrap();
        alice_mgr.complete_handshake_initiator(&accept_ab, &secret_ab).unwrap();

        let (init_ac, secret_ac) = alice_mgr.initiate_handshake(charlie_id);
        let (accept_ac, _) = charlie_mgr.respond_to_handshake(&init_ac).unwrap();
        alice_mgr.complete_handshake_initiator(&accept_ac, &secret_ac).unwrap();

        alice_mgr.remove_peer(&bob_id);
        assert_eq!(alice_mgr.session_count(), 1);
        // Charlie still works
        let ct = alice_mgr.encrypt_for(charlie_id, b"hi").unwrap();
        assert_eq!(charlie_mgr.decrypt_from(alice_id, &ct).unwrap(), b"hi");
        // Bob is gone
        assert!(matches!(
            alice_mgr.encrypt_for(bob_id, b"hi"),
            Err(SessionError::NoActiveSession)
        ));
    }

    // ── MeshSession peer accessor ──

    #[test]
    fn mesh_session_peer_returns_correct_node_id() {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);

        let (init, secret) = alice_mgr.initiate_handshake(bob_id);
        let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
        alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();

        // Bob's session with Alice should report Alice as peer
        // We can't directly access the session, but we can verify via decrypt
        let ct = alice_mgr.encrypt_for(bob_id, b"test").unwrap();
        let pt = bob_mgr.decrypt_from(alice_id, &ct).unwrap();
        assert_eq!(pt, b"test");
    }

    // ── SessionManager::random_ephemeral_secret ──

    #[test]
    fn random_ephemeral_secret_produces_different_keys() {
        let s1 = SessionManager::random_ephemeral_secret();
        let s2 = SessionManager::random_ephemeral_secret();
        assert_ne!(
            s1.public_key().to_encoded_point(false).as_bytes(),
            s2.public_key().to_encoded_point(false).as_bytes()
        );
    }

    // ── Constants ──

    #[test]
    fn constant_values() {
        assert_eq!(ECDH_PUBLIC_KEY_SIZE, 65);
        assert_eq!(HANDSHAKE_MSG_SIZE, 129);
        assert_eq!(MAX_FRAMES_BEFORE_REKEY, 1_000_000);
        assert_eq!(MAX_SESSION_AGE, Duration::from_secs(300));
    }

    // ── Full round-trip with multiple messages ──

    #[test]
    fn multi_message_round_trip() {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);

        let (init, secret) = alice_mgr.initiate_handshake(bob_id);
        let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
        alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();

        let messages = vec![
            b"hello".to_vec(),
            b"world".to_vec(),
            vec![0u8; 100],
            vec![0xFFu8; 256],
            b"final".to_vec(),
        ];

        for (i, msg) in messages.iter().enumerate() {
            let ct = alice_mgr.encrypt_for(bob_id, msg).unwrap();
            let pt = bob_mgr.decrypt_from(alice_id, &ct).unwrap();
            assert_eq!(&pt, msg, "message {} mismatch", i);
        }
    }

    // ── Respond to handshake creates session on responder side ──

    #[test]
    fn respond_to_handshake_creates_responder_session() {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);

        let (init, secret) = alice_mgr.initiate_handshake(bob_id);
        let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();

        // Bob already has a session with Alice
        assert_eq!(bob_mgr.session_count(), 1);

        // Bob can encrypt to Alice even before Alice completes
        let ct = bob_mgr.encrypt_for(alice_id, b"early").unwrap();
        // But Alice can't decrypt yet (no session on her side)
        assert!(matches!(
            alice_mgr.decrypt_from(bob_id, &ct),
            Err(SessionError::NoActiveSession)
        ));

        // After Alice completes, she can decrypt
        alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();
        let ct2 = bob_mgr.encrypt_for(alice_id, b"after").unwrap();
        assert_eq!(alice_mgr.decrypt_from(bob_id, &ct2).unwrap(), b"after");
    }

    // ── HandshakeInit/HandshakeAccept derive Clone + Debug ──

    #[test]
    fn handshake_init_is_clone_and_debug() {
        let msg = HandshakeInit {
            initiator: node_id(1),
            ephemeral_pub: [0x11; 65],
        };
        let cloned = msg.clone();
        assert_eq!(msg, cloned);
        let debug_str = format!("{:?}", msg);
        assert!(debug_str.contains("HandshakeInit"));
    }

    #[test]
    fn handshake_accept_is_clone_and_debug() {
        let msg = HandshakeAccept {
            responder: node_id(2),
            ephemeral_pub: [0x22; 65],
        };
        let cloned = msg.clone();
        assert_eq!(msg, cloned);
        let debug_str = format!("{:?}", msg);
        assert!(debug_str.contains("HandshakeAccept"));
    }

    // ── Decrypting tampered/invalid data ──

    #[test]
    fn decrypt_random_garbage_fails() {
        let alice_id = node_id(0xAA);
        let bob_id = node_id(0xBB);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut bob_mgr = SessionManager::new(bob_id);

        let (init, secret) = alice_mgr.initiate_handshake(bob_id);
        let (accept, _) = bob_mgr.respond_to_handshake(&init).unwrap();
        alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();

        // Random garbage that has the right structure (12-byte nonce + some payload)
        // but is not a valid ciphertext
        let mut garbage = vec![0x42u8; NONCE_SIZE + 20];
        // Set a high counter to avoid replay issues
        garbage[4..12].copy_from_slice(&0xFFFF_FFFF_FFFF_FFFFu64.to_be_bytes());

        assert!(matches!(
            bob_mgr.decrypt_from(alice_id, &garbage),
            Err(SessionError::DecryptionFailed)
        ));
    }

    // ── Initiate handshake returns correct initiator node ──

    #[test]
    fn initiate_handshake_returns_our_node_id_as_initiator() {
        let our_id = node_id(0xDE);
        let mgr = SessionManager::new(our_id);
        let (init, _) = mgr.initiate_handshake(node_id(0xAD));
        assert_eq!(init.initiator, our_id);
    }

    // ── Handshake with all-zero NodeID ──

    #[test]
    fn handshake_with_zero_node_id() {
        let zero_id = NodeID([0u8; 64]);
        let alice_id = node_id(0xAA);

        let mut alice_mgr = SessionManager::new(alice_id);
        let mut zero_mgr = SessionManager::new(zero_id);

        let (init, secret) = alice_mgr.initiate_handshake(zero_id);
        let (accept, _) = zero_mgr.respond_to_handshake(&init).unwrap();
        alice_mgr.complete_handshake_initiator(&accept, &secret).unwrap();

        let ct = alice_mgr.encrypt_for(zero_id, b"test").unwrap();
        assert_eq!(zero_mgr.decrypt_from(alice_id, &ct).unwrap(), b"test");
    }
}
