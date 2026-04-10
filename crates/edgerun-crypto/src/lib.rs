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
pub use ecdsa;
pub use elliptic_curve;
pub use primeorder;
pub use sec1;
pub use ff;
pub use group;
pub use rfc6979;

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

// ---------------------------------------------------------------------------
// Convenience re-exports for most-used types
// ---------------------------------------------------------------------------

// P-256 ECDSA
pub use p256::ecdsa::{Signature, SigningKey, VerifyingKey};
pub use p256::ecdh::EphemeralSecret;
pub use p256::{PublicKey, EncodedPoint, FieldBytes};
pub use p256::elliptic_curve::sec1::ToEncodedPoint;

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
