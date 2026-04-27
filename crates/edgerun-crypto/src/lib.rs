#![no_std]
#![allow(clippy::all)]

extern crate alloc;

pub mod aead;
pub mod aes;
pub mod error;
pub mod rng;
pub mod sha;

pub use ::aes_gcm;
pub use ::ecdsa;
pub use ::ed25519_dalek;
pub use ::elliptic_curve;
pub use ::p256;
pub use ::x25519_dalek;

use crate::error::{CryptoError, Result};
use crate::sha::Digest;

pub use aead::{Aes256GcmCipher, CipherU12, CipherU16};
pub use aes_gcm::aead::{Aead, AeadInPlace, KeyInit};
pub use aes_gcm::AeadCore;
pub use aes_gcm::Aes256Gcm as AesGcmCipher;
pub use aes_gcm::Nonce;
pub use chacha20poly1305::ChaCha20Poly1305;
pub use ed25519_dalek::SigningKey as Ed25519SigningKey;
pub use p256::ecdsa::SigningKey;
pub use rng::{fill_random, mix_entropy, random_bytes, random_u32, random_u64};
pub use signature::Signer;

pub use crate::rng::OsRng;
pub use rand_core::{CryptoRng, RngCore};
pub mod rand_core {
    pub use crate::rng::OsRng;
    pub use rand_core::{CryptoRng, RngCore};
}

pub mod digest {
    pub use sha2::Digest;
}

pub use sha::{sha256, sha384, sha512, Sha256, Sha384, Sha512};

pub mod sha2 {
    pub use crate::sha::{Sha256, Sha384, Sha512};
    pub use sha2::Digest;
}

pub fn hmac_sha256(key: &[u8], data: &[u8]) -> alloc::vec::Vec<u8> {
    use hmac_crate::{Hmac, Mac};

    let mut mac =
        <Hmac<crate::sha::Sha256> as Mac>::new_from_slice(key).expect("HMAC accepts any key");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

pub fn hmac_sha384(key: &[u8], data: &[u8]) -> alloc::vec::Vec<u8> {
    use hmac_crate::{Hmac, Mac};

    let mut mac =
        <Hmac<crate::sha::Sha384> as Mac>::new_from_slice(key).expect("HMAC accepts any key");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn hkdf_expand_sha256(prk: &[u8], info: &[u8], len: usize) -> alloc::vec::Vec<u8> {
    let mut okm = alloc::vec::Vec::with_capacity(len);
    let mut previous = alloc::vec::Vec::new();
    let mut counter = 1u8;

    while okm.len() < len {
        let mut input = alloc::vec::Vec::with_capacity(previous.len() + info.len() + 1);
        input.extend_from_slice(&previous);
        input.extend_from_slice(info);
        input.push(counter);
        previous = hmac_sha256(prk, &input);
        okm.extend_from_slice(&previous);
        counter = counter.wrapping_add(1);
    }

    okm.truncate(len);
    okm
}

fn hkdf_expand_sha384(prk: &[u8], info: &[u8], len: usize) -> alloc::vec::Vec<u8> {
    let mut okm = alloc::vec::Vec::with_capacity(len);
    let mut previous = alloc::vec::Vec::new();
    let mut counter = 1u8;

    while okm.len() < len {
        let mut input = alloc::vec::Vec::with_capacity(previous.len() + info.len() + 1);
        input.extend_from_slice(&previous);
        input.extend_from_slice(info);
        input.push(counter);
        previous = hmac_sha384(prk, &input);
        okm.extend_from_slice(&previous);
        counter = counter.wrapping_add(1);
    }

    okm.truncate(len);
    okm
}

pub fn hkdf_sha256(
    salt: Option<&[u8]>,
    ikm: &[u8],
    info: &[u8],
    len: usize,
) -> alloc::vec::Vec<u8> {
    let prk;
    let prk = if let Some(salt) = salt {
        prk = hmac_sha256(salt, ikm);
        prk.as_slice()
    } else {
        ikm
    };
    hkdf_expand_sha256(prk, info, len)
}

pub fn hkdf_sha384(
    salt: Option<&[u8]>,
    ikm: &[u8],
    info: &[u8],
    len: usize,
) -> alloc::vec::Vec<u8> {
    let prk;
    let prk = if let Some(salt) = salt {
        prk = hmac_sha384(salt, ikm);
        prk.as_slice()
    } else {
        ikm
    };
    hkdf_expand_sha384(prk, info, len)
}

pub fn random_p256_signing_key() -> p256::ecdsa::SigningKey {
    use p256::ecdsa::SigningKey;
    loop {
        let mut bytes = [0u8; 32];
        fill_random(&mut bytes).expect("edgerun RNG should always provide fallback bytes");
        if let Ok(key) = SigningKey::from_bytes(&bytes.into()) {
            return key;
        }
    }
}

const ENCRYPTED_P256_KEY_MAGIC: &[u8] = b"EDGERUN-P256-GCM1";
const ENCRYPTED_P256_SALT_LEN: usize = 16;
const ENCRYPTED_P256_NONCE_LEN: usize = 12;
const ENCRYPTED_P256_PBKDF2_ITERATIONS: u32 = 100_000;
const P256_PRIVATE_KEY_LEN: usize = 32;

fn derive_p256_encryption_key(passphrase: &str, salt: &[u8]) -> [u8; 32] {
    pbkdf2_crate::pbkdf2_hmac_array::<sha::Sha256, 32>(
        passphrase.as_bytes(),
        salt,
        ENCRYPTED_P256_PBKDF2_ITERATIONS,
    )
}

pub fn encrypt_signing_key(
    signing_key: &p256::ecdsa::SigningKey,
    passphrase: &str,
) -> alloc::vec::Vec<u8> {
    let mut salt = [0u8; ENCRYPTED_P256_SALT_LEN];
    let _ = fill_random(&mut salt);
    let mut nonce_bytes = [0u8; ENCRYPTED_P256_NONCE_LEN];
    let _ = fill_random(&mut nonce_bytes);

    let encryption_key = derive_p256_encryption_key(passphrase, &salt);
    let cipher = aes_gcm::Aes256Gcm::new(aes_gcm::Key::<aes_gcm::Aes256Gcm>::from_slice(
        &encryption_key,
    ));
    let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);
    let key_bytes = signing_key.to_bytes();
    let ciphertext = cipher
        .encrypt(nonce, key_bytes.as_slice())
        .expect("AES-GCM encryption failed");

    let mut out = alloc::vec::Vec::with_capacity(
        ENCRYPTED_P256_KEY_MAGIC.len()
            + ENCRYPTED_P256_SALT_LEN
            + ENCRYPTED_P256_NONCE_LEN
            + ciphertext.len(),
    );
    out.extend_from_slice(ENCRYPTED_P256_KEY_MAGIC);
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    out
}

pub fn decrypt_signing_key(
    encrypted_data: &[u8],
    passphrase: &str,
) -> Result<p256::ecdsa::SigningKey> {
    let header_len = ENCRYPTED_P256_KEY_MAGIC.len();
    let min_len = header_len + ENCRYPTED_P256_SALT_LEN + ENCRYPTED_P256_NONCE_LEN;
    if encrypted_data.len() <= min_len
        || !encrypted_data
            .get(..header_len)
            .is_some_and(|header| header == ENCRYPTED_P256_KEY_MAGIC)
    {
        return Err(CryptoError::InvalidKey);
    }

    let salt_start = header_len;
    let nonce_start = salt_start + ENCRYPTED_P256_SALT_LEN;
    let ciphertext_start = nonce_start + ENCRYPTED_P256_NONCE_LEN;
    let salt = &encrypted_data[salt_start..nonce_start];
    let nonce = aes_gcm::Nonce::from_slice(&encrypted_data[nonce_start..ciphertext_start]);
    let ciphertext = &encrypted_data[ciphertext_start..];

    let encryption_key = derive_p256_encryption_key(passphrase, salt);
    let cipher = aes_gcm::Aes256Gcm::new(aes_gcm::Key::<aes_gcm::Aes256Gcm>::from_slice(
        &encryption_key,
    ));
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| CryptoError::DecryptionFailed)?;
    if plaintext.len() != P256_PRIVATE_KEY_LEN {
        return Err(CryptoError::InvalidKey);
    }

    let mut key_bytes = [0u8; P256_PRIVATE_KEY_LEN];
    key_bytes.copy_from_slice(&plaintext);
    p256::ecdsa::SigningKey::from_bytes(&key_bytes.into()).map_err(|_| CryptoError::InvalidKey)
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CipherSuite {
    TLS_AES_128_GCM_SHA256,
    TLS_AES_256_GCM_SHA384,
}

impl CipherSuite {
    pub fn new(_tls_version: u16, _kx: u16, cipher: u16, hash: u16) -> Self {
        match (cipher, hash) {
            (0x1302, _) | (_, 0x0304) => Self::TLS_AES_256_GCM_SHA384,
            _ => Self::TLS_AES_128_GCM_SHA256,
        }
    }

    pub fn client_default() -> alloc::vec::Vec<Self> {
        alloc::vec![Self::TLS_AES_128_GCM_SHA256, Self::TLS_AES_256_GCM_SHA384]
    }

    pub fn from_wire(value: u16) -> core::result::Result<Self, CryptoError> {
        match value {
            0x1301 => Ok(Self::TLS_AES_128_GCM_SHA256),
            0x1302 => Ok(Self::TLS_AES_256_GCM_SHA384),
            _ => Err(CryptoError::UnsupportedAlgorithm),
        }
    }

    pub fn to_wire(self) -> u16 {
        match self {
            Self::TLS_AES_128_GCM_SHA256 => 0x1301,
            Self::TLS_AES_256_GCM_SHA384 => 0x1302,
        }
    }

    pub fn key_len(self) -> usize {
        match self {
            Self::TLS_AES_128_GCM_SHA256 => 16,
            Self::TLS_AES_256_GCM_SHA384 => 32,
        }
    }
}

pub mod hkdf {
    use crate::sha::Sha256;
    use alloc::vec::Vec;

    pub struct Hkdf<H> {
        skm: Vec<u8>,
        _phantom: core::marker::PhantomData<H>,
    }

    impl Hkdf<crate::Sha256> {
        pub fn new(_salt: &[u8]) -> Self {
            Self {
                skm: Vec::new(),
                _phantom: core::marker::PhantomData,
            }
        }

        pub fn extract(salt: Option<&[u8]>, ikm: &[u8]) -> (Vec<u8>, Self) {
            let prk = crate::hmac_sha256(salt.unwrap_or(b""), ikm);
            (
                prk.clone(),
                Self {
                    skm: prk,
                    _phantom: core::marker::PhantomData,
                },
            )
        }

        pub fn expand(&self, info: &[u8], okm: &mut Vec<u8>) -> Result<Vec<u8>, ()> {
            let len = okm.len();
            let result = crate::hkdf_sha256(None, &self.skm, info, len);
            okm.copy_from_slice(&result);
            Ok(result)
        }
    }
}

pub use x509_cert::der::oid::db::rfc4519::CN;
pub use x509_cert::der::oid::db::rfc5280::ID_CE_SUBJECT_ALT_NAME;
pub use x509_cert::der::{asn1, Decode, DecodePem, Encode};
pub use x509_cert::{name::Name, Certificate};
pub mod x509_cert {
    pub use x509_cert::*;
    pub mod ext {
        pub mod pkix {
            pub mod name {
                pub use x509_cert::ext::pkix::name::GeneralName;
            }
            pub use x509_cert::ext::pkix::SubjectAltName;
        }
        pub use x509_cert::ext::*;
    }
    pub mod time {
        pub use x509_cert::time::Time;
    }
    pub mod der {
        pub use x509_cert::der::*;
    }
}

pub mod hmac {
    pub use crate::hmac_sha256 as HMAC;
    pub use hmac_crate::{Hmac, Mac};
}

pub mod pbkdf2 {
    pub use pbkdf2_crate::{pbkdf2, pbkdf2_hmac_array};
}

pub mod sha1 {
    pub use sha1_crate::Digest;
    pub use sha1_crate::Sha1;
}

pub fn load_cert_and_key_from_pem(
    _cert_pem: &str,
    _key_pem: &str,
) -> Result<(alloc::vec::Vec<u8>, alloc::vec::Vec<u8>)> {
    Err(CryptoError::InvalidKey)
}

pub fn generate_self_signed(_key: &SigningKey, _cn: &str) -> alloc::vec::Vec<u8> {
    alloc::vec::Vec::new()
}

pub fn generate_self_signed_pem(_key: &SigningKey, _cn: &str) -> alloc::string::String {
    alloc::string::String::new()
}

pub fn p256_signing_key_from_pem(_pem: &str) -> Option<SigningKey> {
    None
}
pub fn p256_signing_key_from_der(_der: &[u8]) -> Option<SigningKey> {
    None
}
pub fn p256_signing_key_to_pem(_key: &SigningKey) -> alloc::string::String {
    alloc::string::String::new()
}
pub fn pem_encode(_data: &[u8]) -> alloc::string::String {
    alloc::string::String::new()
}
pub fn x509_cert_from_pem(_pem: &str) -> Option<alloc::vec::Vec<u8>> {
    None
}

pub mod signature {
    pub use p256::ecdsa::signature::{Signer, SignerMut, Verifier};
    pub mod hazmat {
        pub use p256::ecdsa::signature::hazmat::{PrehashSigner, PrehashVerifier};
    }
}

#[cfg(feature = "tpm")]
pub mod tpm {
    pub use edgerun_tpm::*;
}
