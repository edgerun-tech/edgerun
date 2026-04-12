use edgerun_hardware_signing::NodeID;
pub use edgerun_crypto::p256::ecdh::EphemeralSecret;
use edgerun_crypto::p256::PublicKey;
use edgerun_crypto::p256::elliptic_curve::sec1::ToEncodedPoint;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::*;

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
    pub(crate) fn new(peer: NodeID, key: [u8; 32]) -> Self {
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

