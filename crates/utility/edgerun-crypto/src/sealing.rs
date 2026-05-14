//! Symmetric sealing helpers for authenticated local storage and envelopes.

extern crate alloc;

use alloc::vec::Vec;

use crate::{CryptoError, Result};

#[cfg(feature = "aead")]
const AES256_GCM_NONCE_LEN: usize = 12;
#[cfg(feature = "aead")]
const AES256_GCM_TAG_LEN: usize = 16;
#[cfg(feature = "aead")]
const SEALED_V1_MAGIC: &[u8; 8] = b"ERSEAL1\0";

#[cfg(feature = "aead")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedBytes {
    pub nonce: [u8; AES256_GCM_NONCE_LEN],
    pub ciphertext: Vec<u8>,
    pub tag: [u8; AES256_GCM_TAG_LEN],
}

#[cfg(feature = "aead")]
impl SealedBytes {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(
            SEALED_V1_MAGIC.len() + self.nonce.len() + self.ciphertext.len() + self.tag.len(),
        );
        out.extend_from_slice(SEALED_V1_MAGIC);
        out.extend_from_slice(&self.nonce);
        out.extend_from_slice(&self.ciphertext);
        out.extend_from_slice(&self.tag);
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let header_len = SEALED_V1_MAGIC.len() + AES256_GCM_NONCE_LEN + AES256_GCM_TAG_LEN;
        if bytes.len() < header_len || &bytes[..SEALED_V1_MAGIC.len()] != SEALED_V1_MAGIC {
            return Err(CryptoError::InvalidKey);
        }

        let nonce_start = SEALED_V1_MAGIC.len();
        let ciphertext_start = nonce_start + AES256_GCM_NONCE_LEN;
        let tag_start = bytes.len() - AES256_GCM_TAG_LEN;
        let mut nonce = [0u8; AES256_GCM_NONCE_LEN];
        nonce.copy_from_slice(&bytes[nonce_start..ciphertext_start]);
        let mut tag = [0u8; AES256_GCM_TAG_LEN];
        tag.copy_from_slice(&bytes[tag_start..]);

        Ok(Self {
            nonce,
            ciphertext: bytes[ciphertext_start..tag_start].to_vec(),
            tag,
        })
    }
}

#[cfg(feature = "aead")]
pub fn seal_aes256_gcm(key: &[u8; 32], aad: &[u8], plaintext: &[u8]) -> Result<SealedBytes> {
    let mut nonce = [0u8; AES256_GCM_NONCE_LEN];
    crate::fill_random(&mut nonce)?;
    seal_aes256_gcm_with_nonce(key, aad, plaintext, nonce)
}

#[cfg(feature = "aead")]
pub fn seal_aes256_gcm_with_nonce(
    key: &[u8; 32],
    aad: &[u8],
    plaintext: &[u8],
    nonce: [u8; AES256_GCM_NONCE_LEN],
) -> Result<SealedBytes> {
    use crate::aead::{Aes256Gcm, KeyInit};

    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| CryptoError::InvalidKey)?;
    let mut ciphertext = plaintext.to_vec();
    let tag = cipher
        .encrypt_in_place_detached(crate::Nonce::from(nonce), aad, &mut ciphertext)
        .map_err(|_| CryptoError::EncryptionFailed)?;
    let mut tag_bytes = [0u8; AES256_GCM_TAG_LEN];
    tag_bytes.copy_from_slice(tag.as_ref());

    Ok(SealedBytes {
        nonce,
        ciphertext,
        tag: tag_bytes,
    })
}

#[cfg(feature = "aead")]
pub fn unseal_aes256_gcm(key: &[u8; 32], aad: &[u8], sealed: &SealedBytes) -> Result<Vec<u8>> {
    use crate::aead::{Aes256Gcm, KeyInit, Tag};

    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| CryptoError::InvalidKey)?;
    let mut plaintext = sealed.ciphertext.clone();
    cipher
        .decrypt_in_place_detached(
            crate::Nonce::from(sealed.nonce),
            aad,
            &mut plaintext,
            &Tag::from(sealed.tag),
        )
        .map_err(|_| CryptoError::DecryptionFailed)?;
    Ok(plaintext)
}

#[cfg(all(feature = "aead", feature = "hmac"))]
pub fn derive_sealing_key_sha256(salt: Option<&[u8]>, secret: &[u8], info: &[u8]) -> [u8; 32] {
    let key = crate::hkdf_sha256(salt, secret, info, 32);
    let mut out = [0u8; 32];
    out.copy_from_slice(&key);
    out
}

#[cfg(all(test, feature = "aead"))]
mod tests {
    use super::*;

    #[test]
    fn aes256_gcm_sealed_bytes_roundtrip() {
        let key = [7u8; 32];
        let aad = b"edgerun:test:seal";
        let sealed =
            seal_aes256_gcm_with_nonce(&key, aad, b"trust container bytes", [9u8; 12]).unwrap();
        let encoded = sealed.to_bytes();
        let decoded = SealedBytes::from_bytes(&encoded).unwrap();

        assert_eq!(
            unseal_aes256_gcm(&key, aad, &decoded).unwrap(),
            b"trust container bytes"
        );
    }

    #[test]
    fn aes256_gcm_rejects_tampering() {
        let key = [7u8; 32];
        let mut sealed =
            seal_aes256_gcm_with_nonce(&key, b"aad", b"sealed payload", [9u8; 12]).unwrap();
        sealed.ciphertext[0] ^= 1;

        assert_eq!(
            unseal_aes256_gcm(&key, b"aad", &sealed),
            Err(CryptoError::DecryptionFailed)
        );
    }
}
