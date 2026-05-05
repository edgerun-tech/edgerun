#![no_std]

extern crate alloc;

use alloc::vec::Vec;

use edgerun_crypto::aes_gcm::aead::generic_array::GenericArray;
use edgerun_crypto::Aes256GcmCipher;
use edgerun_keygen::{node_signing_key_from_bytes, NodeSigningKey};

const MAGIC: &[u8] = b"EDGERUN-SEAL-KEY1";
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;
const KEY_LEN: usize = 32;
const P256_PRIVATE_KEY_LEN: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SealError {
    InvalidEnvelope,
    InvalidKey,
    RandomFailed,
    SealFailed,
    UnsealFailed,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SealKey([u8; KEY_LEN]);

impl SealKey {
    pub const fn from_bytes(bytes: [u8; KEY_LEN]) -> Self {
        Self(bytes)
    }

    pub fn generate() -> Result<Self, SealError> {
        let mut key = [0u8; KEY_LEN];
        edgerun_crypto::fill_random(&mut key).map_err(|_| SealError::RandomFailed)?;
        Ok(Self(key))
    }

    pub const fn expose_secret(&self) -> &[u8; KEY_LEN] {
        &self.0
    }
}

impl core::fmt::Debug for SealKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("SealKey([redacted])")
    }
}

pub fn generate_seal_key() -> Result<SealKey, SealError> {
    SealKey::generate()
}

pub fn seal_with_key(plaintext: &[u8], key: &SealKey) -> Result<Vec<u8>, SealError> {
    let mut nonce = [0u8; NONCE_LEN];
    edgerun_crypto::fill_random(&mut nonce).map_err(|_| SealError::RandomFailed)?;

    let cipher = Aes256GcmCipher::new(key.expose_secret()).map_err(|_| SealError::InvalidKey)?;
    let mut ciphertext = plaintext.to_vec();
    let tag = cipher
        .encrypt_in_place_detached(GenericArray::from_slice(&nonce), MAGIC, &mut ciphertext)
        .map_err(|_| SealError::SealFailed)?;

    let mut out = Vec::with_capacity(MAGIC.len() + NONCE_LEN + ciphertext.len() + TAG_LEN);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    out.extend_from_slice(&tag);
    Ok(out)
}

pub fn unseal_with_key(envelope: &[u8], key: &SealKey) -> Result<Vec<u8>, SealError> {
    let parsed = parse_envelope(envelope)?;
    let tag_start = parsed.ciphertext_with_tag.len() - TAG_LEN;
    let mut plaintext = parsed.ciphertext_with_tag[..tag_start].to_vec();
    let tag = GenericArray::from_slice(&parsed.ciphertext_with_tag[tag_start..]);
    let cipher = Aes256GcmCipher::new(key.expose_secret()).map_err(|_| SealError::InvalidKey)?;
    cipher
        .decrypt_in_place_detached(
            GenericArray::from_slice(parsed.nonce),
            MAGIC,
            &mut plaintext,
            tag,
        )
        .map_err(|_| SealError::UnsealFailed)?;
    Ok(plaintext)
}

pub fn seal_node_signing_key(
    signing_key: &NodeSigningKey,
    key: &SealKey,
) -> Result<Vec<u8>, SealError> {
    seal_with_key(signing_key.to_bytes().as_slice(), key)
}

pub fn unseal_node_signing_key(
    envelope: &[u8],
    key: &SealKey,
) -> Result<NodeSigningKey, SealError> {
    let plaintext = unseal_with_key(envelope, key)?;
    if plaintext.len() != P256_PRIVATE_KEY_LEN {
        return Err(SealError::InvalidKey);
    }
    let mut key_bytes = [0u8; P256_PRIVATE_KEY_LEN];
    key_bytes.copy_from_slice(&plaintext);
    node_signing_key_from_bytes(key_bytes).map_err(|_| SealError::InvalidKey)
}

struct ParsedEnvelope<'a> {
    nonce: &'a [u8],
    ciphertext_with_tag: &'a [u8],
}

fn parse_envelope(envelope: &[u8]) -> Result<ParsedEnvelope<'_>, SealError> {
    let header_len = MAGIC.len();
    let min_len = header_len + NONCE_LEN + TAG_LEN;
    if envelope.len() <= min_len || envelope.get(..header_len) != Some(MAGIC) {
        return Err(SealError::InvalidEnvelope);
    }

    let nonce_start = header_len;
    let ciphertext_start = nonce_start + NONCE_LEN;
    Ok(ParsedEnvelope {
        nonce: &envelope[nonce_start..ciphertext_start],
        ciphertext_with_tag: &envelope[ciphertext_start..],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_keygen::generate_node_signing_key;

    #[test]
    fn sealed_bytes_roundtrip() {
        let key = generate_seal_key().unwrap();
        let sealed = seal_with_key(b"secret", &key).unwrap();
        assert_ne!(sealed, b"secret");
        assert_eq!(unseal_with_key(&sealed, &key).unwrap(), b"secret");
    }

    #[test]
    fn wrong_key_fails() {
        let key = generate_seal_key().unwrap();
        let wrong_key = generate_seal_key().unwrap();
        let sealed = seal_with_key(b"secret", &key).unwrap();
        assert_eq!(
            unseal_with_key(&sealed, &wrong_key).unwrap_err(),
            SealError::UnsealFailed
        );
    }

    #[test]
    fn node_signing_key_roundtrips() {
        let key = generate_seal_key().unwrap();
        let (signing_key, identity) = generate_node_signing_key();
        let sealed = seal_node_signing_key(&signing_key, &key).unwrap();
        let unsealed = unseal_node_signing_key(&sealed, &key).unwrap();
        assert_eq!(
            edgerun_keygen::node_id_from_signing_key(&unsealed),
            identity.node_id
        );
    }
}
