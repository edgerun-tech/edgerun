#![no_std]
#![allow(clippy::all)]

extern crate alloc;

pub mod error;
pub mod aes;
pub mod sha;
pub mod rng;
pub mod aead;

pub use ::aes_gcm;
pub use ::ecdsa;
pub use ::elliptic_curve;
pub use ::p256;
pub use ::x25519_dalek;

use crate::error::{CryptoError, Result};
use crate::rng::fill_random;
use crate::sha::Digest;

pub use aead::{Aes256GcmCipher, CipherU12, CipherU16};
pub use aes_gcm::AeadCore;
pub use aes_gcm::aead::{AeadInPlace, KeyInit};
pub use chacha20poly1305::ChaCha20Poly1305;
pub use p256::ecdsa::SigningKey;

pub use rand_core::{OsRng, CryptoRng, RngCore};
pub mod rand_core {
    pub use rand_core::{OsRng, CryptoRng, RngCore};
}

pub mod digest {
    pub use sha2::Digest;
}

pub use sha::{Sha256, Sha384, Sha512, sha256, sha384, sha512};

pub mod sha2 {
    pub use sha2::Digest;
    pub use crate::sha::{Sha256, Sha384, Sha512};
}

pub fn hmac_sha256(key: &[u8], data: &[u8]) -> alloc::vec::Vec<u8> {
    let mut d = crate::sha::Sha256::new();
    sha2::Digest::update(&mut d, data);
    let hash = sha2::Digest::finalize(d);
    let mut result = alloc::vec::Vec::with_capacity(32);
    for (i, k) in key.iter().enumerate() {
        result.push(hash.get(i).copied().unwrap_or(0) ^ k);
    }
    result
}

pub fn hmac_sha384(key: &[u8], data: &[u8]) -> alloc::vec::Vec<u8> {
    let mut d = crate::sha::Sha384::new();
    sha2::Digest::update(&mut d, data);
    let hash = sha2::Digest::finalize(d);
    let mut result = alloc::vec::Vec::with_capacity(48);
    for (i, k) in key.iter().enumerate() {
        result.push(hash.get(i).copied().unwrap_or(0) ^ k);
    }
    result
}

pub fn hkdf_sha256(salt: Option<&[u8]>, ikm: &[u8], info: &[u8], len: usize) -> alloc::vec::Vec<u8> {
    let prk = hmac_sha256(salt.unwrap_or(b""), ikm);
    let mut okm = alloc::vec::Vec::with_capacity(len);
    let mut t = alloc::vec::Vec::new();
    let mut counter = 0u8;
    while okm.len() < len {
        counter = counter.wrapping_add(1);
        if !t.is_empty() {
            t.extend_from_slice(&okm[okm.len().saturating_sub(32)..]);
        }
        t.push(counter);
        t.extend_from_slice(info);
        let mut d = crate::sha::Sha256::new();
        sha2::Digest::update(&mut d, &t);
        let hash = sha2::Digest::finalize(d);
        okm.extend_from_slice(&hash);
    }
    okm.truncate(len);
    okm
}

pub fn hkdf_sha384(salt: Option<&[u8]>, ikm: &[u8], info: &[u8], len: usize) -> alloc::vec::Vec<u8> {
    use sha2::Digest;
    let prk = hmac_sha384(salt.unwrap_or(b""), ikm);
    let mut okm = alloc::vec::Vec::with_capacity(len);
    let mut t = alloc::vec::Vec::new();
    let mut counter = 0u8;
    while okm.len() < len {
        counter = counter.wrapping_add(1);
        if !t.is_empty() {
            t.extend_from_slice(&okm[okm.len().saturating_sub(48)..]);
        }
        t.push(counter);
        t.extend_from_slice(info);
        let mut d = Sha384::new();
        d.update(&t);
        let hash = d.finalize();
        okm.extend_from_slice(&hash);
    }
    okm.truncate(len);
    okm
}

pub fn random_p256_signing_key() -> p256::ecdsa::SigningKey {
    use p256::ecdsa::SigningKey;
    let mut bytes = [0u8; 32];
    let _ = fill_random(&mut bytes);
    SigningKey::from_bytes(&bytes.into()).unwrap()
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
    use alloc::vec::Vec;
    use crate::sha::Sha256;
    
    pub struct Hkdf<H> {
        skm: Vec<u8>,
        _phantom: core::marker::PhantomData<H>,
    }
    
    impl Hkdf<crate::Sha256> {
        pub fn new(_salt: &[u8]) -> Self {
            Self { skm: Vec::new(), _phantom: core::marker::PhantomData }
        }
        
        pub fn extract(salt: Option<&[u8]>, ikm: &[u8]) -> (Vec<u8>, Self) {
            let prk = crate::hmac_sha256(salt.unwrap_or(b""), ikm);
            (prk.clone(), Self { skm: prk, _phantom: core::marker::PhantomData })
        }
        
        pub fn expand(&self, info: &[u8], okm: &mut Vec<u8>) -> Result<Vec<u8>, ()> {
            let len = okm.len();
            let result = crate::hkdf_sha256(None, &self.skm, info, len);
            okm.copy_from_slice(&result);
            Ok(result)
        }
    }
}

pub use x509_cert::{Certificate, name::Name};
pub use x509_cert::der::{Decode, DecodePem, Encode, asn1};
pub use x509_cert::der::oid::db::rfc4519::CN;
pub use x509_cert::der::oid::db::rfc5280::ID_CE_SUBJECT_ALT_NAME;
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
}

pub fn load_cert_and_key_from_pem(_cert_pem: &str, _key_pem: &str) -> Result<(alloc::vec::Vec<u8>, alloc::vec::Vec<u8>)> {
    Err(CryptoError::InvalidKey)
}

pub fn generate_self_signed(_key: &SigningKey, _cn: &str) -> alloc::vec::Vec<u8> {
    alloc::vec::Vec::new()
}

pub fn generate_self_signed_pem(_key: &SigningKey, _cn: &str) -> alloc::string::String {
    alloc::string::String::new()
}

pub fn p256_signing_key_from_pem(_pem: &str) -> Option<SigningKey> { None }
pub fn p256_signing_key_from_der(_der: &[u8]) -> Option<SigningKey> { None }
pub fn p256_signing_key_to_pem(_key: &SigningKey) -> alloc::string::String { alloc::string::String::new() }
pub fn pem_encode(_data: &[u8]) -> alloc::string::String { alloc::string::String::new() }
pub fn x509_cert_from_pem(_pem: &str) -> Option<alloc::vec::Vec<u8>> { None }

pub mod signature {
    pub use p256::ecdsa::signature::{Signer, SignerMut, Verifier};
    pub mod hazmat {
        pub use p256::ecdsa::signature::hazmat::{PrehashSigner, PrehashVerifier};
    }
}

pub fn getrandom(_dest: &mut [u8]) -> core::result::Result<(), CryptoError> {
    fill_random(_dest).map_err(|_| CryptoError::RandomGenerationFailed)
}

#[cfg(feature = "tpm")]
pub mod tpm {
    pub use edgerun_tpm::*;
}
