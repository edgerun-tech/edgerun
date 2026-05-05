#![no_std]

extern crate alloc;

use alloc::vec::Vec;

use edgerun_crypto::aes_gcm::aead::generic_array::GenericArray;
use edgerun_crypto::pbkdf2::pbkdf2_hmac_array;
use edgerun_crypto::{Aes256GcmCipher, Sha256};
use edgerun_keygen::{node_signing_key_from_bytes, NodeSigningKey};

const MAGIC: &[u8] = b"EDGERUN-SEAL-GCM1";
const LEGACY_P256_MAGIC: &[u8] = b"EDGERUN-P256-GCM1";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;
const KEY_LEN: usize = 32;
const P256_PRIVATE_KEY_LEN: usize = 32;
pub const DEFAULT_PBKDF2_ITERATIONS: u32 = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SealError {
    InvalidEnvelope,
    InvalidKey,
    RandomFailed,
    SealFailed,
    UnsealFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SealProfile {
    pub iterations: u32,
}

impl SealProfile {
    pub const fn new(iterations: u32) -> Self {
        Self { iterations }
    }
}

impl Default for SealProfile {
    fn default() -> Self {
        Self::new(DEFAULT_PBKDF2_ITERATIONS)
    }
}

pub fn seal_with_passphrase(plaintext: &[u8], passphrase: &str) -> Result<Vec<u8>, SealError> {
    seal_with_profile(plaintext, passphrase, SealProfile::default())
}

pub fn unseal_with_passphrase(envelope: &[u8], passphrase: &str) -> Result<Vec<u8>, SealError> {
    unseal_with_profile(envelope, passphrase, SealProfile::default())
}

pub fn seal_node_signing_key(
    signing_key: &NodeSigningKey,
    passphrase: &str,
) -> Result<Vec<u8>, SealError> {
    seal_with_passphrase(signing_key.to_bytes().as_slice(), passphrase)
}

pub fn unseal_node_signing_key(
    envelope: &[u8],
    passphrase: &str,
) -> Result<NodeSigningKey, SealError> {
    let plaintext = if envelope.starts_with(LEGACY_P256_MAGIC) {
        unseal_legacy_p256_key(envelope, passphrase)?
    } else {
        unseal_with_passphrase(envelope, passphrase)?
    };
    if plaintext.len() != P256_PRIVATE_KEY_LEN {
        return Err(SealError::InvalidKey);
    }
    let mut key_bytes = [0u8; P256_PRIVATE_KEY_LEN];
    key_bytes.copy_from_slice(&plaintext);
    node_signing_key_from_bytes(key_bytes).map_err(|_| SealError::InvalidKey)
}

pub fn seal_with_profile(
    plaintext: &[u8],
    passphrase: &str,
    profile: SealProfile,
) -> Result<Vec<u8>, SealError> {
    if profile.iterations == 0 {
        return Err(SealError::InvalidKey);
    }

    let mut salt = [0u8; SALT_LEN];
    edgerun_crypto::fill_random(&mut salt).map_err(|_| SealError::RandomFailed)?;
    let mut nonce = [0u8; NONCE_LEN];
    edgerun_crypto::fill_random(&mut nonce).map_err(|_| SealError::RandomFailed)?;

    let key = derive_key(passphrase, &salt, profile.iterations);
    let cipher = Aes256GcmCipher::new(&key).map_err(|_| SealError::InvalidKey)?;
    let mut ciphertext = plaintext.to_vec();
    let tag = cipher
        .encrypt_in_place_detached(GenericArray::from_slice(&nonce), MAGIC, &mut ciphertext)
        .map_err(|_| SealError::SealFailed)?;

    let mut out =
        Vec::with_capacity(MAGIC.len() + 4 + SALT_LEN + NONCE_LEN + ciphertext.len() + TAG_LEN);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&profile.iterations.to_le_bytes());
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    out.extend_from_slice(&tag);
    Ok(out)
}

pub fn unseal_with_profile(
    envelope: &[u8],
    passphrase: &str,
    expected_profile: SealProfile,
) -> Result<Vec<u8>, SealError> {
    let parsed = parse_envelope(envelope)?;
    if parsed.iterations != expected_profile.iterations {
        return Err(SealError::InvalidEnvelope);
    }
    unseal_parsed(parsed, passphrase)
}

fn unseal_legacy_p256_key(envelope: &[u8], passphrase: &str) -> Result<Vec<u8>, SealError> {
    let header_len = LEGACY_P256_MAGIC.len();
    let min_len = header_len + SALT_LEN + NONCE_LEN + TAG_LEN;
    if envelope.len() <= min_len {
        return Err(SealError::InvalidEnvelope);
    }

    let salt_start = header_len;
    let nonce_start = salt_start + SALT_LEN;
    let ciphertext_start = nonce_start + NONCE_LEN;
    let parsed = ParsedEnvelope {
        aad: &[],
        iterations: DEFAULT_PBKDF2_ITERATIONS,
        salt: &envelope[salt_start..nonce_start],
        nonce: &envelope[nonce_start..ciphertext_start],
        ciphertext_with_tag: &envelope[ciphertext_start..],
    };
    unseal_parsed(parsed, passphrase)
}

struct ParsedEnvelope<'a> {
    aad: &'a [u8],
    iterations: u32,
    salt: &'a [u8],
    nonce: &'a [u8],
    ciphertext_with_tag: &'a [u8],
}

fn parse_envelope(envelope: &[u8]) -> Result<ParsedEnvelope<'_>, SealError> {
    let header_len = MAGIC.len();
    let min_len = header_len + 4 + SALT_LEN + NONCE_LEN + TAG_LEN;
    if envelope.len() <= min_len || envelope.get(..header_len) != Some(MAGIC) {
        return Err(SealError::InvalidEnvelope);
    }

    let iterations_start = header_len;
    let salt_start = iterations_start + 4;
    let nonce_start = salt_start + SALT_LEN;
    let ciphertext_start = nonce_start + NONCE_LEN;
    let iterations = u32::from_le_bytes(
        envelope[iterations_start..salt_start]
            .try_into()
            .map_err(|_| SealError::InvalidEnvelope)?,
    );
    if iterations == 0 {
        return Err(SealError::InvalidEnvelope);
    }

    Ok(ParsedEnvelope {
        aad: MAGIC,
        iterations,
        salt: &envelope[salt_start..nonce_start],
        nonce: &envelope[nonce_start..ciphertext_start],
        ciphertext_with_tag: &envelope[ciphertext_start..],
    })
}

fn unseal_parsed(parsed: ParsedEnvelope<'_>, passphrase: &str) -> Result<Vec<u8>, SealError> {
    if parsed.nonce.len() != NONCE_LEN || parsed.ciphertext_with_tag.len() < TAG_LEN {
        return Err(SealError::InvalidEnvelope);
    }

    let tag_start = parsed.ciphertext_with_tag.len() - TAG_LEN;
    let mut plaintext = parsed.ciphertext_with_tag[..tag_start].to_vec();
    let tag = GenericArray::from_slice(&parsed.ciphertext_with_tag[tag_start..]);
    let key = derive_key(passphrase, parsed.salt, parsed.iterations);
    let cipher = Aes256GcmCipher::new(&key).map_err(|_| SealError::InvalidKey)?;
    cipher
        .decrypt_in_place_detached(
            GenericArray::from_slice(parsed.nonce),
            parsed.aad,
            &mut plaintext,
            tag,
        )
        .map_err(|_| SealError::UnsealFailed)?;
    Ok(plaintext)
}

fn derive_key(passphrase: &str, salt: &[u8], iterations: u32) -> [u8; KEY_LEN] {
    pbkdf2_hmac_array::<Sha256, KEY_LEN>(passphrase.as_bytes(), salt, iterations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_keygen::generate_node_signing_key;

    #[test]
    fn sealed_bytes_roundtrip() {
        let sealed = seal_with_passphrase(b"secret", "correct horse").unwrap();
        assert_ne!(sealed, b"secret");
        assert_eq!(
            unseal_with_passphrase(&sealed, "correct horse").unwrap(),
            b"secret"
        );
    }

    #[test]
    fn wrong_passphrase_fails() {
        let sealed = seal_with_passphrase(b"secret", "correct horse").unwrap();
        assert_eq!(
            unseal_with_passphrase(&sealed, "wrong").unwrap_err(),
            SealError::UnsealFailed
        );
    }

    #[test]
    fn node_signing_key_roundtrips() {
        let (signing_key, identity) = generate_node_signing_key();
        let sealed = seal_node_signing_key(&signing_key, "passphrase").unwrap();
        let unsealed = unseal_node_signing_key(&sealed, "passphrase").unwrap();
        assert_eq!(
            edgerun_keygen::node_id_from_signing_key(&unsealed),
            identity.node_id
        );
    }

    #[test]
    fn legacy_p256_key_envelope_unseals() {
        let (signing_key, identity) = generate_node_signing_key();
        let sealed = edgerun_crypto::encrypt_signing_key(&signing_key, "passphrase");
        assert!(sealed.starts_with(LEGACY_P256_MAGIC));

        let unsealed = unseal_node_signing_key(&sealed, "passphrase").unwrap();
        assert_eq!(
            edgerun_keygen::node_id_from_signing_key(&unsealed),
            identity.node_id
        );
    }
}
