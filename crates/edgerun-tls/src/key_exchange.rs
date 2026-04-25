//! ECDH key exchange for TLS 1.3 key_share.
//! Supports P-256 (SECP256R1) and X25519.
//! All crypto flows through edgerun-crypto.

use edgerun_crypto::getrandom;
use edgerun_crypto::p256::ecdh::EphemeralSecret as P256Secret;
use edgerun_crypto::p256::EncodedPoint;
use edgerun_crypto::x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret as X25519Secret};
use edgerun_crypto::OsRng;

/// Named group for key exchange
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyExchangeGroup {
    /// secp256r1 (NIST P-256) — 65-byte uncompressed SEC1 points
    SECP256R1,
    /// x25519 (Curve25519) — 32-byte public keys
    X25519,
}

impl KeyExchangeGroup {
    /// Parse a key exchange group from its TLS wire-format value.
    pub fn from_wire(value: u16) -> Option<Self> {
        match value {
            0x0017 => Some(KeyExchangeGroup::SECP256R1),
            0x001D => Some(KeyExchangeGroup::X25519),
            _ => None,
        }
    }

    /// Encode this group to its TLS wire-format value.
    pub fn to_wire(self) -> u16 {
        match self {
            KeyExchangeGroup::SECP256R1 => 0x0017,
            KeyExchangeGroup::X25519 => 0x001D,
        }
    }

    /// Expected public key length in bytes for this group.
    pub fn public_key_len(self) -> usize {
        match self {
            KeyExchangeGroup::SECP256R1 => 65, // 0x04 || x(32) || y(32)
            KeyExchangeGroup::X25519 => 32,
        }
    }
}

/// A key pair for ECDH key exchange.
/// Supports both P-256 and X25519.
pub enum EcdhKeyPair {
    /// P-256 (secp256r1) key pair with uncompressed point.
    P256 {
        /// The secret scalar (private key).
        secret: P256Secret,
        /// The public key as an uncompressed SEC1 encoded point.
        public: EncodedPoint,
    },
    /// X25519 (Curve25519) key pair.
    X25519 {
        /// The secret scalar (private key).
        secret: X25519Secret,
        /// The public key as a 32-byte Montgomery point.
        public: [u8; 32],
    },
}

impl EcdhKeyPair {
    /// Generate a new key pair for the specified group
    pub fn generate(group: KeyExchangeGroup) -> Result<Self, String> {
        match group {
            KeyExchangeGroup::SECP256R1 => {
                let secret = P256Secret::random(&mut OsRng);
                let public = EncodedPoint::from(secret.public_key());
                Ok(EcdhKeyPair::P256 { secret, public })
            }
            KeyExchangeGroup::X25519 => {
                let mut secret_bytes = [0u8; 32];
                getrandom::fill(&mut secret_bytes).expect("getrandom failed");
                let secret = X25519Secret::from(secret_bytes);
                let public: X25519PublicKey = (&secret).into();
                Ok(EcdhKeyPair::X25519 {
                    secret,
                    public: public.to_bytes(),
                })
            }
        }
    }

    /// Raw public key bytes (SEC1 uncompressed for P-256, raw 32 bytes for X25519)
    pub fn public_key_bytes(&self) -> Vec<u8> {
        match self {
            EcdhKeyPair::P256 { public, .. } => public.as_bytes().to_vec(),
            EcdhKeyPair::X25519 { public, .. } => public.to_vec(),
        }
    }

    /// The group this key pair uses
    pub fn group(&self) -> KeyExchangeGroup {
        match self {
            EcdhKeyPair::P256 { .. } => KeyExchangeGroup::SECP256R1,
            EcdhKeyPair::X25519 { .. } => KeyExchangeGroup::X25519,
        }
    }

    /// Compute the shared secret with the peer's public key.
    /// The peer_pk must match this key pair's group format.
    pub fn exchange(&self, peer_pk: &[u8]) -> Result<Vec<u8>, String> {
        match self {
            EcdhKeyPair::P256 { secret, .. } => {
                let peer_pk = edgerun_crypto::p256::PublicKey::from_sec1_bytes(peer_pk)
                    .map_err(|e| format!("Invalid P-256 public key: {:?}", e))?;
                let shared = secret.diffie_hellman(&peer_pk);
                Ok(shared.raw_secret_bytes().to_vec())
            }
            EcdhKeyPair::X25519 { secret, .. } => {
                if peer_pk.len() != 32 {
                    return Err(format!(
                        "X25519 public key must be 32 bytes, got {}",
                        peer_pk.len()
                    ));
                }
                let mut pk_bytes = [0u8; 32];
                pk_bytes.copy_from_slice(peer_pk);
                let peer_pk = X25519PublicKey::from(pk_bytes);
                let shared = secret.diffie_hellman(&peer_pk);
                Ok(shared.to_bytes().to_vec())
            }
        }
    }
}
