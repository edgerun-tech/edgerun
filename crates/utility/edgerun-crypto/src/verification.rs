//! High-level signature verification helpers.

use crate::{CryptoError, Result};

#[cfg(feature = "ed25519")]
pub fn ed25519_verify(public_key: &[u8], message: &[u8], signature: &[u8]) -> Result<()> {
    use crate::ed25519::Signature;
    use crate::ed25519_dalek::VerifyingKey;
    use crate::signature::Verifier;

    let public_key: [u8; 32] = public_key.try_into().map_err(|_| CryptoError::InvalidKey)?;
    let key = VerifyingKey::from_bytes(&public_key).map_err(|_| CryptoError::InvalidKey)?;
    let signature = Signature::from_slice(signature).map_err(|_| CryptoError::InvalidSignature)?;
    key.verify(message, &signature)
        .map_err(|_| CryptoError::SignatureVerificationFailed)
}

#[cfg(feature = "ed25519")]
pub fn ed25519_verify_strict(
    public_key: &[u8; 32],
    message: &[u8],
    signature: &[u8],
) -> Result<()> {
    use crate::ed25519::Signature;
    use crate::ed25519_dalek::VerifyingKey;

    let key = VerifyingKey::from_bytes(public_key).map_err(|_| CryptoError::InvalidKey)?;
    let signature = Signature::from_slice(signature).map_err(|_| CryptoError::InvalidSignature)?;
    key.verify_strict(message, &signature)
        .map_err(|_| CryptoError::SignatureVerificationFailed)
}

#[cfg(feature = "p256")]
pub fn p256_verify_sha256_der(
    public_key_sec1: &[u8],
    message: &[u8],
    signature_der: &[u8],
) -> Result<()> {
    p256_verify_prehash_der(public_key_sec1, &crate::sha256(message), signature_der)
}

#[cfg(feature = "p256")]
pub fn p256_verify_sha256_fixed(
    public_key_sec1: &[u8],
    message: &[u8],
    signature: &[u8],
) -> Result<()> {
    p256_verify_prehash_fixed(public_key_sec1, &crate::sha256(message), signature)
}

#[cfg(feature = "p256")]
pub fn p256_verify_prehash_fixed(
    public_key_sec1: &[u8],
    prehash: &[u8],
    signature: &[u8],
) -> Result<()> {
    use crate::p256::ecdsa::signature::hazmat::PrehashVerifier;

    let key = crate::p256::ecdsa::VerifyingKey::from_sec1_bytes(public_key_sec1)
        .map_err(|_| CryptoError::InvalidKey)?;
    let signature = crate::p256::ecdsa::Signature::from_slice(signature)
        .map_err(|_| CryptoError::InvalidSignature)?;
    key.verify_prehash(prehash, &signature)
        .map_err(|_| CryptoError::SignatureVerificationFailed)
}

#[cfg(feature = "p256")]
pub fn p256_verify_prehash_der(
    public_key_sec1: &[u8],
    prehash: &[u8],
    signature_der: &[u8],
) -> Result<()> {
    use crate::p256::ecdsa::signature::hazmat::PrehashVerifier;

    let key = crate::p256::ecdsa::VerifyingKey::from_sec1_bytes(public_key_sec1)
        .map_err(|_| CryptoError::InvalidKey)?;
    let signature = crate::p256::ecdsa::Signature::from_der(signature_der)
        .map_err(|_| CryptoError::InvalidSignature)?;
    key.verify_prehash(prehash, &signature)
        .map_err(|_| CryptoError::SignatureVerificationFailed)
}

#[cfg(feature = "rsa")]
pub fn rsa_pss_sha256(
    public_key: crate::rsa::RsaPublicKey,
    message: &[u8],
    signature: &[u8],
) -> Result<()> {
    use crate::rsa::signature::Verifier;

    let signature = crate::rsa::pss::Signature::try_from(signature)
        .map_err(|_| CryptoError::InvalidSignature)?;
    let key = crate::rsa::pss::VerifyingKey::<crate::rsa::sha2::Sha256>::new(public_key);
    key.verify(message, &signature)
        .map_err(|_| CryptoError::SignatureVerificationFailed)
}
