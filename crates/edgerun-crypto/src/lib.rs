//! Central crypto crate for the edgerun project.
//!
//! Re-exports all cryptographic primitives used across the workspace
//! through a single dependency boundary.
//!
//! # Usage
//!
//! Instead of depending on `p256`, `sha2`, `aes-gcm`, `hkdf`, etc. directly,
//! consumer crates should depend on `edgerun-crypto` and import through it:
//! ```ignore
//! use edgerun_crypto::p256::ecdsa::SigningKey;
//! use edgerun_crypto::sha2::Sha256;
//! use edgerun_crypto::aes_gcm::Aes256Gcm;
//! ```

// ---------------------------------------------------------------------------
// Hash / digest
// ---------------------------------------------------------------------------
pub use digest;
pub use sha2;
pub use sha1;
pub use pbkdf2;

// ---------------------------------------------------------------------------
// MAC / KDF
// ---------------------------------------------------------------------------
pub use hmac;
pub use hkdf;

// ---------------------------------------------------------------------------
// Symmetric encryption (AEAD)
// ---------------------------------------------------------------------------
pub use aes_gcm;

// ---------------------------------------------------------------------------
// Elliptic curve / asymmetric
// ---------------------------------------------------------------------------
pub use p256;
pub use rsa;
pub use ed448_goldilocks;
pub use ecdsa;
pub use elliptic_curve;
pub use primeorder;
pub use sec1;
pub use ff;
pub use group;
pub use rfc6979;
pub use x25519_dalek;
pub use ed25519_dalek;
pub use curve25519_dalek;
pub use chacha20poly1305;

// ---------------------------------------------------------------------------
// Signature trait abstraction
// ---------------------------------------------------------------------------
pub use signature;

// ---------------------------------------------------------------------------
// Encoding / formats
// ---------------------------------------------------------------------------
pub use der;
pub use base16ct;
pub use const_oid;

// ---------------------------------------------------------------------------
// X.509 certificate generation / parsing
// ---------------------------------------------------------------------------
pub use x509_cert;
pub use rcgen;

// ---------------------------------------------------------------------------
// Core crypto primitives
// ---------------------------------------------------------------------------
pub use crypto_bigint;
pub use crypto_common;
pub use block_buffer;
pub use rand_core;
pub use subtle;
pub use zeroize;
pub use generic_array;
pub use typenum;
pub use getrandom;

// ---------------------------------------------------------------------------
// Convenience re-exports for most-used types
// ---------------------------------------------------------------------------

// P-256 ECDSA
pub use p256::ecdsa::{Signature, SigningKey, VerifyingKey};
pub use p256::ecdh::EphemeralSecret;
pub use p256::{PublicKey, EncodedPoint, FieldBytes};
pub use p256::elliptic_curve::sec1::ToEncodedPoint;

// RSA
pub use rsa::RsaPublicKey;
pub use rsa::pkcs1::DecodeRsaPublicKey;

// ED25519 (RFC 8080 DNSSEC algorithm 15)
pub use ed25519_dalek::{SigningKey as Ed25519SigningKey, VerifyingKey as Ed25519VerifyingKey, Signature as Ed25519Signature};

// ED448 (RFC 8080 DNSSEC algorithm 16)
pub use ed448_goldilocks::Signature as Ed448Signature;
pub use ed448_goldilocks::VerifyingKey as Ed448VerifyingKey;

// Hash
pub use sha2::{Digest, Sha256, Sha384, Sha512};

// MAC / KDF
pub use hmac::{Hmac, Mac};
pub use hkdf::Hkdf;

// AEAD
pub use aes_gcm::{
    aead::{Aead, AeadInPlace, KeyInit},
    Aes128Gcm, Aes256Gcm, Key, Nonce,
};

// Signature traits
pub use signature::{Signer, Verifier};
pub use p256::ecdsa::signature::hazmat::{PrehashSigner, PrehashVerifier};

// DER
pub use der::Tag;

// ---------------------------------------------------------------------------
// Convenience hash functions
// ---------------------------------------------------------------------------

/// SHA-256 hash of `data`, returning 32 bytes.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}
