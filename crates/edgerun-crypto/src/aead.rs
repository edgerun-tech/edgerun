#![allow(clippy::all)]

use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::{AeadInPlace, Aes128Gcm, Aes256Gcm, Key, KeyInit};
use typenum::U12;
use typenum::U16;

#[derive(Clone)]
enum AesGcmInner {
    Aes128(Aes128Gcm),
    Aes256(Aes256Gcm),
}

#[derive(Clone)]
pub struct Aes256GcmCipher {
    inner: AesGcmInner,
}

impl Aes256GcmCipher {
    pub fn new(key: &[u8]) -> Result<Self, aes_gcm::aead::Error> {
        match key.len() {
            16 => {
                let key = Key::<Aes128Gcm>::from_slice(key);
                Ok(Self {
                    inner: AesGcmInner::Aes128(Aes128Gcm::new(key)),
                })
            }
            32 => {
                let key = Key::<Aes256Gcm>::from_slice(key);
                Ok(Self {
                    inner: AesGcmInner::Aes256(Aes256Gcm::new(key)),
                })
            }
            _ => Err(aes_gcm::aead::Error),
        }
    }

    pub fn new_from_key(key: &[u8]) -> Result<Self, aes_gcm::aead::Error> {
        Self::new(key)
    }

    pub fn encrypt_in_place_detached(
        &self,
        nonce: &GenericArray<u8, U12>,
        aad: &[u8],
        buffer: &mut [u8],
    ) -> Result<GenericArray<u8, U16>, aes_gcm::aead::Error> {
        match &self.inner {
            AesGcmInner::Aes128(inner) => inner.encrypt_in_place_detached(nonce, aad, buffer),
            AesGcmInner::Aes256(inner) => inner.encrypt_in_place_detached(nonce, aad, buffer),
        }
    }

    pub fn decrypt_in_place_detached(
        &self,
        nonce: &GenericArray<u8, U12>,
        aad: &[u8],
        buffer: &mut [u8],
        tag: &GenericArray<u8, U16>,
    ) -> Result<(), aes_gcm::aead::Error> {
        match &self.inner {
            AesGcmInner::Aes128(inner) => inner.decrypt_in_place_detached(nonce, aad, buffer, tag),
            AesGcmInner::Aes256(inner) => inner.decrypt_in_place_detached(nonce, aad, buffer, tag),
        }
    }
}

pub type CipherU12 = U12;
pub type CipherU16 = U16;
