//! High-level signing helpers backed by EdgeRun-owned primitives.

extern crate alloc;

use alloc::vec::Vec;

use crate::{CryptoError, Result};

#[cfg(feature = "ed25519")]
pub fn ed25519_keypair() -> crate::ed25519_dalek::SigningKey {
    crate::random_ed25519_signing_key()
}

#[cfg(feature = "ed25519")]
pub fn ed25519_key() -> crate::Ed25519SigningKey {
    crate::random_ed25519_key()
}

#[cfg(feature = "ed25519")]
pub fn ed25519_key_from_seed(seed: &[u8; 32]) -> crate::Ed25519SigningKey {
    crate::Ed25519SigningKey::from_bytes(seed)
}

#[cfg(feature = "ed25519")]
pub fn ed25519_sign(key: &crate::Ed25519SigningKey, message: &[u8]) -> [u8; 64] {
    key.sign_bytes(message)
}

#[cfg(feature = "ed25519")]
pub fn ed25519_public_key(key: &crate::Ed25519SigningKey) -> [u8; 32] {
    key.verifying_key().to_bytes()
}

#[cfg(feature = "p256")]
pub fn p256_key() -> crate::P256SigningKey {
    crate::P256SigningKey::random()
}

#[cfg(feature = "p256")]
pub fn p256_public_key_sec1(key: &crate::P256SigningKey) -> Vec<u8> {
    key.public_key_sec1()
}

#[cfg(feature = "p256")]
pub fn p256_sign_sha256_der(key: &crate::P256SigningKey, message: &[u8]) -> Result<Vec<u8>> {
    p256_sign_prehash_der(key, &crate::sha256(message))
}

#[cfg(feature = "p256")]
pub fn p256_sign_sha256_fixed(key: &crate::P256SigningKey, message: &[u8]) -> Result<[u8; 64]> {
    p256_sign_prehash_fixed(key, &crate::sha256(message))
}

#[cfg(feature = "p256")]
pub fn p256_sign_prehash_fixed(key: &crate::P256SigningKey, prehash: &[u8]) -> Result<[u8; 64]> {
    use crate::p256::ecdsa::signature::hazmat::PrehashSigner;
    use crate::signature::SignatureEncoding;

    let signature: crate::p256::ecdsa::Signature = key
        .as_raw()
        .sign_prehash(prehash)
        .map_err(|_| CryptoError::SigningError)?;
    let bytes = signature.to_bytes();
    let mut out = [0u8; 64];
    out.copy_from_slice(bytes.as_slice());
    Ok(out)
}

#[cfg(feature = "p256")]
pub fn p256_sign_prehash_der(key: &crate::P256SigningKey, prehash: &[u8]) -> Result<Vec<u8>> {
    use crate::p256::ecdsa::signature::hazmat::PrehashSigner;

    let signature: crate::p256::ecdsa::Signature = key
        .as_raw()
        .sign_prehash(prehash)
        .map_err(|_| CryptoError::SigningError)?;
    Ok(signature.to_der().as_bytes().to_vec())
}

#[cfg(feature = "rsa")]
pub fn rsa_private_key(bit_size: usize) -> Result<crate::rsa::RsaPrivateKey> {
    crate::random_rsa_private_key(bit_size).map_err(|_| CryptoError::InvalidKey)
}

#[cfg(feature = "rsa")]
pub fn rsa_pss_sha256(key: crate::rsa::RsaPrivateKey, message: &[u8]) -> Vec<u8> {
    use crate::rsa::signature::SignatureEncoding;

    crate::rsa_pss_sha256_sign(key, message).to_bytes().to_vec()
}
