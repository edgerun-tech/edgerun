//! TLS 1.3 key derivation using HKDF (RFC 8446 §7)
//!
//! Uses the workspace `hkdf` and `hmac` crates backed by SHA-256/384.

use edgerun_crypto::hmac::{Hmac, Mac};
use edgerun_crypto::sha2::{Digest, Sha256, Sha384};

/// Hash abstraction for TLS 1.3 key derivation
#[derive(Clone)]
pub enum Hasher {
    Sha256,
    Sha384,
}

impl Hasher {
    /// Output length in bytes
    pub fn len(&self) -> usize {
        match self {
            Hasher::Sha256 => 32,
            Hasher::Sha384 => 48,
        }
    }

    /// HKDF-Extract(salt, ikm)
    pub fn extract(&self, salt: &[u8], ikm: &[u8]) -> Vec<u8> {
        match self {
            Self::Sha256 => {
                let (prk, _hk) = Hkdf::<Sha256>::extract(Some(salt), ikm);
                prk.to_vec()
            }
            Self::Sha384 => {
                let (prk, _hk) = Hkdf::<Sha384>::extract(Some(salt), ikm);
                prk.to_vec()
            }
        }
    }

    /// HKDF-Expand-Label(secret, label, context, length) per RFC 8446 §7.1
    pub fn expand_label(&self, secret: &[u8], label: &str, context: &[u8], length: usize) -> Vec<u8> {
        let hkdf_label = build_hkdf_label(label, context, length);
        self.expand(secret, &hkdf_label, length)
    }

    /// HKDF-Expand(prk, info, length)
    fn expand(&self, prk: &[u8], info: &[u8], length: usize) -> Vec<u8> {
        let hash_len = self.len();
        let n = (length + hash_len - 1) / hash_len;
        let mut okm = Vec::with_capacity(length);
        let mut t = Vec::new();

        for i in 1..=n {
            let mut input = t.clone();
            input.extend_from_slice(info);
            input.push(i as u8);
            t = match self {
                Hasher::Sha256 => {
                    type HmacSha256 = Hmac<Sha256>;
                    hmac_sha256(prk, &input)
                }
                Hasher::Sha384 => {
                    type HmacSha384 = Hmac<Sha384>;
                    hmac_sha384(prk, &input)
                }
            };
            okm.extend_from_slice(&t);
        }

        okm.truncate(length);
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
    /// Start from a zero salt (early secret derivation)
    pub fn new(hash: Hasher) -> Self {
        let zero = vec![0u8; hash.len()];
        let secret = hash.extract(&zero, &zero); // early_secret
        Tls13KeySchedule { hash, secret }
    }

    /// Advance to handshake secret using the ECDH shared secret
    pub fn advance_to_handshake(&mut self, shared_secret: &[u8], _ch_hash: &[u8], _sh_hash: &[u8]) {
        // derive_secret("derived")
        let derived = self.hash.expand_label(&self.secret, "derived", &[], self.hash.len());

        // handshake_secret = HKDF-Extract(derived_secret, shared_secret)
        // Per RFC 8446 §7.1: salt=derived, ikm=shared_secret
        let handshake_secret = self.hash.extract(&derived, shared_secret);
        self.secret = handshake_secret;
    }

    /// Derive client handshake traffic secret
    pub fn client_handshake_traffic_secret(&self, ch_hash: &[u8]) -> Vec<u8> {
        self.hash.expand_label(&self.secret, "c hs traffic", ch_hash, self.hash.len())
    }

    /// Derive server handshake traffic secret
    pub fn server_handshake_traffic_secret(&self, ch_hash: &[u8]) -> Vec<u8> {
        self.hash.expand_label(&self.secret, "s hs traffic", ch_hash, self.hash.len())
    }

    /// Advance to master secret
    pub fn advance_to_master(&mut self) {
        let derived = self.hash.expand_label(&self.secret, "derived", &[], self.hash.len());
        let zero = vec![0u8; self.hash.len()];
        self.secret = self.hash.extract(&derived, &zero); // master_secret
    }

    /// Derive client application traffic secret
    pub fn client_app_traffic_secret(&self) -> Vec<u8> {
        self.hash.expand_label(&self.secret, "c ap traffic", &[], self.hash.len())
    }

    /// Derive server application traffic secret
    pub fn server_app_traffic_secret(&self) -> Vec<u8> {
        self.hash.expand_label(&self.secret, "s ap traffic", &[], self.hash.len())
    }

    /// Derive resumption master secret
    pub fn resumption_master_secret(&self) -> Vec<u8> {
        self.hash.expand_label(&self.secret, "res master", &[], self.hash.len())
    }
}

/// Derive record-layer keys from traffic secret
pub struct TrafficKeys {
    pub write_key: Vec<u8>,
    pub write_iv: Vec<u8>,
}

/// Derive AEAD keys for client → server (handshake)
/// Labels per RFC 8446 §7.3: "c hs key", "c hs iv"
pub fn client_write_keys(secret: &[u8], cipher_key_len: usize, iv_len: usize, hash: &Hasher) -> TrafficKeys {
    TrafficKeys {
        write_key: hash.expand_label(secret, "c hs key", &[], cipher_key_len),
        write_iv: hash.expand_label(secret, "c hs iv", &[], iv_len),
    }
}

/// Derive AEAD keys for server → client (handshake)
/// Labels per RFC 8446 §7.3: "s hs key", "s hs iv"
pub fn server_write_keys(secret: &[u8], cipher_key_len: usize, iv_len: usize, hash: &Hasher) -> TrafficKeys {
    TrafficKeys {
        write_key: hash.expand_label(secret, "s hs key", &[], cipher_key_len),
        write_iv: hash.expand_label(secret, "s hs iv", &[], iv_len),
    }
}

/// Derive AEAD keys for client → server (application data)
/// Labels per RFC 8446 §7.3: "c ap key", "c ap iv"
pub fn client_app_write_keys(secret: &[u8], cipher_key_len: usize, iv_len: usize, hash: &Hasher) -> TrafficKeys {
    TrafficKeys {
        write_key: hash.expand_label(secret, "c ap key", &[], cipher_key_len),
        write_iv: hash.expand_label(secret, "c ap iv", &[], iv_len),
    }
}

/// Derive AEAD keys for server → client (application data)
/// Labels per RFC 8446 §7.3: "s ap key", "s ap iv"
pub fn server_app_write_keys(secret: &[u8], cipher_key_len: usize, iv_len: usize, hash: &Hasher) -> TrafficKeys {
    TrafficKeys {
        write_key: hash.expand_label(secret, "s ap key", &[], cipher_key_len),
        write_iv: hash.expand_label(secret, "s ap iv", &[], iv_len),
    }
}

/// HKDF-Label encoding (RFC 8446 §7.1)
fn build_hkdf_label(label: &str, context: &[u8], length: usize) -> Vec<u8> {
    let full_label = format!("tls13 {}", label);
    let mut out = Vec::with_capacity(2 + 1 + full_label.len() + 1 + context.len());
    out.extend_from_slice(&(length as u16).to_be_bytes());
    out.push(full_label.len() as u8);
    out.extend_from_slice(full_label.as_bytes());
    out.push(context.len() as u8);
    out.extend_from_slice(context);
    out
}

pub fn hmac_sha256(key: &[u8], msg: &[u8]) -> Vec<u8> {
    type HmacSha256Inner = Hmac<Sha256>;
    let mut mac = HmacSha256Inner::new_from_slice(key).expect("HMAC key length ok");
    mac.update(msg);
    mac.finalize().into_bytes().to_vec()
}

pub fn hmac_sha384(key: &[u8], msg: &[u8]) -> Vec<u8> {
    type HmacSha384Inner = Hmac<Sha384>;
    let mut mac = HmacSha384Inner::new_from_slice(key).expect("HMAC key length ok");
    mac.update(msg);
    mac.finalize().into_bytes().to_vec()
}

// Re-export hkdf::Hkdf for internal use (the crate's extract/expand semantics)
use edgerun_crypto::hkdf::Hkdf;

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
        let _client_app = ks.client_app_traffic_secret();
        let _server_app = ks.server_app_traffic_secret();
    }
}
