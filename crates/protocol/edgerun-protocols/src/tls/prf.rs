//! TLS 1.3 key derivation using HKDF (RFC 8446 §7)
//!
//! Uses the workspace `hkdf` and `hmac` crates backed by SHA-256/384.

use alloc::{format, vec, vec::Vec};
use edgerun_crypto::sha::{Digest, Sha256, Sha384};
use edgerun_encoding::byteorder::push_u16_be;

/// Hash abstraction for TLS 1.3 key derivation.
#[derive(Clone)]
pub enum Hasher {
    /// SHA-256 (used for TLS_AES_128_GCM_SHA256).
    Sha256,
    /// SHA-384 (used for TLS_AES_256_GCM_SHA384).
    Sha384,
}

impl Hasher {
    /// Output length of the hash function in bytes.
    pub fn len(&self) -> usize {
        match self {
            Hasher::Sha256 => 32,
            Hasher::Sha384 => 48,
        }
    }

    /// HKDF-Extract(salt, ikm) — derive a pseudorandom key from input keying material.
    pub fn extract(&self, salt: &[u8], ikm: &[u8]) -> Vec<u8> {
        match self {
            Self::Sha256 => hmac_sha256(salt, ikm),
            Self::Sha384 => hmac_sha384(salt, ikm),
        }
    }

    /// HKDF-Expand-Label(secret, label, context, length) per RFC 8446 §7.1
    pub fn expand_label(
        &self,
        secret: &[u8],
        label: &str,
        context: &[u8],
        length: usize,
    ) -> Vec<u8> {
        let hkdf_label = build_hkdf_label(label, context, length);
        self.expand(secret, &hkdf_label, length)
    }

    /// Derive-Secret(secret, label, messages) per RFC 8446 §7.1
    ///
    /// This is HKDF-Expand-Label(secret, label, Hash(messages), Hash.length)
    /// where Hash(messages) is the transcript hash of the given messages.
    pub fn derive_secret(&self, secret: &[u8], label: &str, messages: &[u8]) -> Vec<u8> {
        let hash = self.hash(messages);
        self.expand_label(secret, label, &hash, self.len())
    }

    /// HKDF-Expand(prk, info, length) using the hkdf crate directly.
    fn expand(&self, prk: &[u8], info: &[u8], length: usize) -> Vec<u8> {
        let mut okm = vec![0u8; length];
        match self {
            Hasher::Sha256 => okm = edgerun_crypto::hkdf_sha256(None, prk, info, length),
            Hasher::Sha384 => okm = edgerun_crypto::hkdf_sha384(None, prk, info, length),
        }
        okm
    }

    /// SHA-256 hash
    pub fn hash(&self, data: &[u8]) -> Vec<u8> {
        match self {
            Hasher::Sha256 => Sha256::digest(data).to_vec(),
            Hasher::Sha384 => Sha384::digest(data).to_vec(),
        }
    }
}

/// TLS 1.3 key schedule (RFC 8446 §7.1)
pub struct Tls13KeySchedule {
    hash: Hasher,
    /// Current secret (progresses through the schedule)
    secret: Vec<u8>,
}

impl Tls13KeySchedule {
    /// Get current secret for debugging
    pub fn debug_secret(&self) -> &[u8] {
        &self.secret
    }

    /// Start from a zero salt (early secret derivation)
    pub fn new(hash: Hasher) -> Self {
        let zero = vec![0u8; hash.len()];
        let secret = hash.extract(&zero, &zero); // early_secret
        Tls13KeySchedule { hash, secret }
    }

    /// Advance to handshake secret using the ECDH shared secret
    pub fn advance_to_handshake(&mut self, shared_secret: &[u8], _ch_hash: &[u8], _sh_hash: &[u8]) {
        // derive_secret("derived") with empty messages = Hash("")
        let derived = self.hash.derive_secret(&self.secret, "derived", &[]);

        // handshake_secret = HKDF-Extract(derived_secret, shared_secret)
        // Per RFC 8446 §7.1: salt=derived, ikm=shared_secret
        let handshake_secret = self.hash.extract(&derived, shared_secret);
        self.secret = handshake_secret;
    }

    /// Derive client early traffic secret (0-RTT, RFC 8446 §7.1).
    ///
    /// Must be called BEFORE `advance_to_handshake()` — while the secret
    /// is still the early secret. Derives using label "c e traffic"
    /// with the ClientHello hash as context.
    pub fn client_early_traffic_secret(&self, ch_hash: &[u8]) -> Vec<u8> {
        self.hash
            .expand_label(&self.secret, "c e traffic", ch_hash, self.hash.len())
    }

    /// Derive client handshake traffic secret
    pub fn client_handshake_traffic_secret(&self, ch_hash: &[u8]) -> Vec<u8> {
        self.hash
            .expand_label(&self.secret, "c hs traffic", ch_hash, self.hash.len())
    }

    /// Derive server handshake traffic secret
    pub fn server_handshake_traffic_secret(&self, ch_hash: &[u8]) -> Vec<u8> {
        self.hash
            .expand_label(&self.secret, "s hs traffic", ch_hash, self.hash.len())
    }

    /// Advance to master secret
    pub fn advance_to_master(&mut self) {
        let derived = self.hash.derive_secret(&self.secret, "derived", &[]);
        let zero = vec![0u8; self.hash.len()];
        self.secret = self.hash.extract(&derived, &zero); // master_secret
    }

    /// Derive client application traffic secret
    /// Per RFC 8446 §7.1: HKDF-Expand-Label(master_secret, "c ap traffic", Hash(CH1..SH-Finished), 32)
    pub fn client_app_traffic_secret(&self, transcript_hash: &[u8]) -> Vec<u8> {
        self.hash.expand_label(
            &self.secret,
            "c ap traffic",
            transcript_hash,
            self.hash.len(),
        )
    }

    /// Derive server application traffic secret
    /// Per RFC 8446 §7.1: HKDF-Expand-Label(master_secret, "s ap traffic", Hash(CH1..SH-Finished), 32)
    pub fn server_app_traffic_secret(&self, transcript_hash: &[u8]) -> Vec<u8> {
        self.hash.expand_label(
            &self.secret,
            "s ap traffic",
            transcript_hash,
            self.hash.len(),
        )
    }

    /// Derive resumption master secret
    pub fn resumption_master_secret(&self) -> Vec<u8> {
        self.hash
            .expand_label(&self.secret, "res master", &[], self.hash.len())
    }
}

/// Derived record-layer AEAD keys and IV.
pub struct TrafficKeys {
    /// AEAD encryption key.
    pub write_key: Vec<u8>,
    /// Initialization vector (nonce) for AEAD.
    pub write_iv: Vec<u8>,
}

/// Derive AEAD keys for client → server (handshake)
/// Labels per RFC 8446 §7.3: "key", "iv"
pub fn client_write_keys(
    secret: &[u8],
    cipher_key_len: usize,
    iv_len: usize,
    hash: &Hasher,
) -> TrafficKeys {
    TrafficKeys {
        write_key: hash.expand_label(secret, "key", &[], cipher_key_len),
        write_iv: hash.expand_label(secret, "iv", &[], iv_len),
    }
}

/// Derive AEAD keys for server → client (handshake)
/// Labels per RFC 8446 §7.3: "key", "iv"
pub fn server_write_keys(
    secret: &[u8],
    cipher_key_len: usize,
    iv_len: usize,
    hash: &Hasher,
) -> TrafficKeys {
    TrafficKeys {
        write_key: hash.expand_label(secret, "key", &[], cipher_key_len),
        write_iv: hash.expand_label(secret, "iv", &[], iv_len),
    }
}

/// Derive AEAD keys for client → server (application data)
/// Labels per RFC 8446 §7.3: "key", "iv"
pub fn client_app_write_keys(
    secret: &[u8],
    cipher_key_len: usize,
    iv_len: usize,
    hash: &Hasher,
) -> TrafficKeys {
    TrafficKeys {
        write_key: hash.expand_label(secret, "key", &[], cipher_key_len),
        write_iv: hash.expand_label(secret, "iv", &[], iv_len),
    }
}

/// Derive AEAD keys for server → client (application data)
/// Labels per RFC 8446 §7.3: "key", "iv"
pub fn server_app_write_keys(
    secret: &[u8],
    cipher_key_len: usize,
    iv_len: usize,
    hash: &Hasher,
) -> TrafficKeys {
    TrafficKeys {
        write_key: hash.expand_label(secret, "key", &[], cipher_key_len),
        write_iv: hash.expand_label(secret, "iv", &[], iv_len),
    }
}

/// HKDF-Label encoding (RFC 8446 §7.1)
fn build_hkdf_label(label: &str, context: &[u8], length: usize) -> Vec<u8> {
    let full_label = format!("tls13 {}", label);
    let mut out = Vec::with_capacity(2 + 1 + full_label.len() + 1 + context.len());
    push_u16_be(&mut out, length as u16);
    out.push(full_label.len() as u8);
    out.extend_from_slice(full_label.as_bytes());
    out.push(context.len() as u8);
    out.extend_from_slice(context);
    out
}

// ---------------------------------------------------------------------------
// QUIC key derivation (RFC 9001 §5)
// ---------------------------------------------------------------------------

/// QUIC-specific HKDF-Expand-Label using `"quic {label}"` prefix.
///
/// RFC 9001 §5.1 uses the same HKDF-Expand-Label structure as TLS 1.3
/// (RFC 8446 §7.1) but with the prefix `"quic"` instead of `"tls13"`.
/// Labels: `"quic key"`, `"quic iv"`, `"quic hp"`, `"quic ku"`.
fn build_quic_hkdf_label(label: &str, context: &[u8], length: usize) -> Vec<u8> {
    let full_label = format!("quic {}", label);
    let mut out = Vec::with_capacity(2 + 1 + full_label.len() + 1 + context.len());
    push_u16_be(&mut out, length as u16);
    out.push(full_label.len() as u8);
    out.extend_from_slice(full_label.as_bytes());
    out.push(context.len() as u8);
    out.extend_from_slice(context);
    out
}

impl Hasher {
    /// HKDF-Expand-Label using QUIC labels (`"quic {label}"`).
    ///
    /// Used for deriving AEAD keys, IVs, and header protection keys from
    /// QUIC traffic secrets (RFC 9001 §5.1).
    pub fn quic_expand_label(
        &self,
        secret: &[u8],
        label: &str,
        context: &[u8],
        length: usize,
    ) -> Vec<u8> {
        let hkdf_label = build_quic_hkdf_label(label, context, length);
        self.expand(secret, &hkdf_label, length)
    }
}

/// Well-known initial salts for QUIC version 1 (RFC 9001 §5.2).
pub const INITIAL_SALT_V1: &[u8] = &[
    0x38, 0x76, 0xcf, 0x71, 0xba, 0x52, 0x1f, 0x3d, 0x62, 0xd5, 0x1f, 0xa5, 0x78, 0x3d, 0x78, 0x39,
    0x87, 0x6d, 0xc0, 0x78,
];

/// Initial traffic keys for a QUIC client (RFC 9001 §5.2).
///
/// Derives the AEAD keys and IVs for Initial packets from the server's
/// destination connection ID.
///
/// Returns `(write_keys, read_keys)` where:
/// - `write_keys` — keys for encrypting client → server Initial packets
/// - `read_keys` — keys for decrypting server → client Initial packets
pub fn quic_initial_client_keys(
    dcid: &[u8],
    cipher_key_len: usize,
    iv_len: usize,
    hash: &Hasher,
) -> (TrafficKeys, TrafficKeys) {
    let initial_secret = hash.extract(INITIAL_SALT_V1, dcid);

    let client_in_secret = hash.quic_expand_label(&initial_secret, "client in", &[], hash.len());
    let server_in_secret = hash.quic_expand_label(&initial_secret, "server in", &[], hash.len());

    let write_keys = quic_traffic_keys(&server_in_secret, cipher_key_len, iv_len, hash);
    let read_keys = quic_traffic_keys(&client_in_secret, cipher_key_len, iv_len, hash);

    (write_keys, read_keys)
}

/// Initial traffic keys for a QUIC server (RFC 9001 §5.2).
///
/// Returns `(write_keys, read_keys)` where:
/// - `write_keys` — keys for encrypting server → client Initial packets
/// - `read_keys` — keys for decrypting client → server Initial packets
pub fn quic_initial_server_keys(
    dcid: &[u8],
    cipher_key_len: usize,
    iv_len: usize,
    hash: &Hasher,
) -> (TrafficKeys, TrafficKeys) {
    let initial_secret = hash.extract(INITIAL_SALT_V1, dcid);

    let client_in_secret = hash.quic_expand_label(&initial_secret, "client in", &[], hash.len());
    let server_in_secret = hash.quic_expand_label(&initial_secret, "server in", &[], hash.len());

    let write_keys = quic_traffic_keys(&client_in_secret, cipher_key_len, iv_len, hash);
    let read_keys = quic_traffic_keys(&server_in_secret, cipher_key_len, iv_len, hash);

    (write_keys, read_keys)
}

/// Derive AEAD traffic keys (key + IV) from a QUIC traffic secret.
///
/// Uses QUIC-specific labels `"quic key"` and `"quic iv"` (RFC 9001 §5.1).
pub fn quic_traffic_keys(
    secret: &[u8],
    cipher_key_len: usize,
    iv_len: usize,
    hash: &Hasher,
) -> TrafficKeys {
    TrafficKeys {
        write_key: hash.quic_expand_label(secret, "key", &[], cipher_key_len),
        write_iv: hash.quic_expand_label(secret, "iv", &[], iv_len),
    }
}

/// Derive header protection key from a QUIC traffic secret.
///
/// Uses QUIC-specific label `"quic hp"` (RFC 9001 §5.4).
pub fn quic_hp_key(secret: &[u8], cipher_key_len: usize, hash: &Hasher) -> Vec<u8> {
    hash.quic_expand_label(secret, "hp", &[], cipher_key_len)
}

// HMAC helpers re-exported from edgerun-crypto (single source of truth)
pub use edgerun_crypto::{hmac_sha256, hmac_sha384};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_label_output_length() {
        let hash = Hasher::Sha256;
        let secret = vec![0xAB; 32];
        let out = hash.expand_label(&secret, "key", &[], 16);
        assert_eq!(out.len(), 16);

        let out = hash.expand_label(&secret, "iv", &[], 12);
        assert_eq!(out.len(), 12);
    }

    #[test]
    fn test_key_schedule_derives_keys() {
        let hash = Hasher::Sha256;
        let mut ks = Tls13KeySchedule::new(hash.clone());

        // After new(), we have early_secret
        let _early = ks.secret.clone();

        // Advance with fake shared secret
        ks.advance_to_handshake(b"shared_secret_here", &[1u8; 32], &[2u8; 32]);

        let _client_hs = ks.client_handshake_traffic_secret(&[1u8; 32]);
        let _server_hs = ks.server_handshake_traffic_secret(&[1u8; 32]);

        ks.advance_to_master();
        let dummy_hash = vec![0u8; 32];
        let _client_app = ks.client_app_traffic_secret(&dummy_hash);
        let _server_app = ks.server_app_traffic_secret(&dummy_hash);
    }

    /// Verify key derivation against RFC 8446 test vectors
    #[test]
    fn test_server_handshake_keys_against_vectors() {
        let hash = Hasher::Sha256;
        let secret: [u8; 32] = [
            0x30, 0x31, 0xe9, 0xc2, 0xc2, 0x6e, 0xcc, 0x15, 0x4b, 0xc3, 0x68, 0x26, 0xe8, 0x7f,
            0xee, 0xff, 0x8f, 0x45, 0x47, 0xdf, 0x52, 0x59, 0x67, 0x47, 0xb2, 0xdc, 0xab, 0xf9,
            0x2b, 0x18, 0xfb, 0x59,
        ];
        let keys = server_write_keys(&secret, 16, 12, &hash);
        let expected_key: [u8; 16] = [
            0x4d, 0x15, 0xc0, 0x0e, 0x47, 0x31, 0x7f, 0xe9, 0x9c, 0x71, 0x4f, 0x8e, 0xbd, 0x92,
            0xc4, 0xd1,
        ];
        let expected_iv: [u8; 12] = [
            0x18, 0x22, 0x30, 0x84, 0x73, 0x5f, 0x2f, 0x2d, 0x85, 0x88, 0xca, 0xaa,
        ];
        assert_eq!(keys.write_key, expected_key, "Key mismatch");
        assert_eq!(keys.write_iv, expected_iv, "IV mismatch");
    }

    // --- QUIC key derivation tests (RFC 9001) ---

    #[test]
    fn test_quic_expand_label_differs_from_tls13() {
        let hash = Hasher::Sha256;
        let secret = vec![0xAB; 32];
        let tls_label = hash.expand_label(&secret, "key", &[], 16);
        let quic_label = hash.quic_expand_label(&secret, "key", &[], 16);
        assert_eq!(tls_label.len(), quic_label.len());
        assert_ne!(tls_label, quic_label);
    }

    #[test]
    fn test_quic_initial_keys_derive() {
        let hash = Hasher::Sha256;
        let dcid = vec![0x83, 0x94, 0xc8, 0xf0, 0x3e, 0x51, 0x57, 0x08];
        let (write, read) = quic_initial_client_keys(&dcid, 16, 12, &hash);
        assert_eq!(write.write_key.len(), 16);
        assert_eq!(write.write_iv.len(), 12);
        assert_eq!(read.write_key.len(), 16);
        assert_eq!(read.write_iv.len(), 12);
        assert_ne!(write.write_key, read.write_key);
    }

    #[test]
    fn test_quic_client_server_keys_are_inverses() {
        // Client and server should use inverses - client's write = server's read
        let hash = Hasher::Sha256;
        let dcid = vec![0x83, 0x94, 0xc8, 0xf0, 0x3e, 0x51, 0x57, 0x08];

        let (client_write, client_read) = quic_initial_client_keys(&dcid, 16, 12, &hash);
        let (server_write, server_read) = quic_initial_server_keys(&dcid, 16, 12, &hash);

        // Verify the inverse relationship
        assert_eq!(client_write.write_key, server_read.write_key);
        assert_eq!(client_write.write_iv, server_read.write_iv);
        assert_eq!(server_write.write_key, client_read.write_key);
        assert_eq!(server_write.write_iv, client_read.write_iv);
    }

    #[test]
    fn test_key_bytes_are_deterministic() {
        // Same DCID should always produce same keys
        let hash = Hasher::Sha256;
        let dcid = vec![0x5b, 0x6c, 0x2f, 0xd6, 0xcc, 0xe0, 0x29, 0x1c];

        let (a, _) = quic_initial_client_keys(&dcid, 16, 12, &hash);
        let (b, _) = quic_initial_client_keys(&dcid, 16, 12, &hash);

        assert_eq!(a.write_key, b.write_key);
        assert_eq!(a.write_iv, b.write_iv);
    }

    #[test]
    fn test_different_dcid_produces_different_keys() {
        let hash = Hasher::Sha256;

        let dcid1 = vec![0x5b, 0x6c, 0x2f, 0xd6, 0xcc, 0xe0, 0x29, 0x1c];
        let dcid2 = vec![0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00, 0x11];

        let (a, _) = quic_initial_client_keys(&dcid1, 16, 12, &hash);
        let (b, _) = quic_initial_client_keys(&dcid2, 16, 12, &hash);

        assert_ne!(a.write_key, b.write_key);
    }

    #[test]
    fn test_quic_traffic_keys_derive() {
        let hash = Hasher::Sha256;
        let secret = vec![0xCD; 32];
        let keys = quic_traffic_keys(&secret, 16, 12, &hash);
        assert_eq!(keys.write_key.len(), 16);
        assert_eq!(keys.write_iv.len(), 12);
    }

    #[test]
    fn test_quic_hp_key_derive() {
        let hash = Hasher::Sha256;
        let secret = vec![0xCD; 32];
        let hp = quic_hp_key(&secret, 16, &hash);
        assert_eq!(hp.len(), 16);
    }
}
