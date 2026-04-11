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

/// SHA-384 hash of `data`, returning 48 bytes.
pub fn sha384(data: &[u8]) -> [u8; 48] {
    use sha2::Digest;
    let mut hasher = sha2::Sha384::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// SHA-512 hash of `data`, returning 64 bytes.
pub fn sha512(data: &[u8]) -> [u8; 64] {
    use sha2::Digest;
    let mut hasher = sha2::Sha512::new();
    hasher.update(data);
    hasher.finalize().into()
}

// ---------------------------------------------------------------------------
// Convenience MAC / KDF functions
// ---------------------------------------------------------------------------

/// HMAC-SHA-256 of `msg` with `key`.
pub fn hmac_sha256(key: &[u8], msg: &[u8]) -> Vec<u8> {
    use hmac::Mac;
    let mut mac = <hmac::Hmac<sha2::Sha256> as digest::KeyInit>::new_from_slice(key)
        .expect("HMAC key length ok");
    mac.update(msg);
    mac.finalize().into_bytes().to_vec()
}

/// HMAC-SHA-384 of `msg` with `key`.
pub fn hmac_sha384(key: &[u8], msg: &[u8]) -> Vec<u8> {
    use hmac::Mac;
    let mut mac = <hmac::Hmac<sha2::Sha384> as digest::KeyInit>::new_from_slice(key)
        .expect("HMAC key length ok");
    mac.update(msg);
    mac.finalize().into_bytes().to_vec()
}

/// HKDF-SHA-256 derive: expand keying material from salt + ikm.
///
/// Returns `okm_len` bytes of output keying material.
pub fn hkdf_sha256(salt: Option<&[u8]>, ikm: &[u8], info: &[u8], okm_len: usize) -> Vec<u8> {
    use hkdf::Hkdf;
    let hk = Hkdf::<sha2::Sha256>::new(salt, ikm);
    let mut okm = vec![0u8; okm_len];
    hk.expand(info, &mut okm).expect("HKDF expand ok");
    okm
}

// ---------------------------------------------------------------------------
// Convenience key generation
// ---------------------------------------------------------------------------

/// Generate a random P-256 ECDSA signing key.
pub fn random_p256_signing_key() -> p256::ecdsa::SigningKey {
    use rand_core::RngCore;
    let mut bytes = [0u8; 32];
    rand_core::OsRng.fill_bytes(&mut bytes);
    p256::ecdsa::SigningKey::from_bytes(&bytes.into()).unwrap()
}

// ---------------------------------------------------------------------------
// Convenience AEAD helpers
// ---------------------------------------------------------------------------

/// AES-256-GCM encrypt `plaintext` with `key` and random nonce.
/// Returns `(nonce, ciphertext_and_tag)`.
pub fn aes256_gcm_encrypt(key: &[u8; 32], plaintext: &[u8]) -> ([u8; 12], Vec<u8>) {
    use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
    use rand_core::RngCore;
    let cipher = Aes256Gcm::new_from_slice(key).expect("valid AES-256 key");
    let mut nonce_bytes = [0u8; 12];
    rand_core::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from(nonce_bytes);
    let ct = cipher.encrypt(&nonce, plaintext).expect("encryption ok");
    (nonce_bytes, ct)
}

/// AES-256-GCM decrypt `ciphertext_and_tag` with `key` and `nonce`.
/// Returns plaintext or error string.
pub fn aes256_gcm_decrypt(key: &[u8; 32], nonce: &[u8; 12], ciphertext_and_tag: &[u8]) -> Result<Vec<u8>, String> {
    use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
    let cipher = Aes256Gcm::new_from_slice(key).expect("valid AES-256 key");
    let nonce = Nonce::from(*nonce);
    cipher.decrypt(&nonce, ciphertext_and_tag).map_err(|e| format!("decryption failed: {:?}", e))
}
