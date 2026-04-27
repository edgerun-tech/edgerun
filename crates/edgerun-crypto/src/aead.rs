#![allow(clippy::all)]

use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::{AeadInPlace, Aes256Gcm, Key, KeyInit};
use typenum::U12;
use typenum::U16;

#[derive(Clone)]
pub struct Aes256GcmCipher {
    inner: Aes256Gcm,
}

impl Aes256GcmCipher {
    pub fn new(key: &[u8]) -> Result<Self, aes_gcm::aead::Error> {
        if key.len() != 32 {
            return Err(aes_gcm::aead::Error);
        }
        let key = Key::<Aes256Gcm>::from_slice(key);
        Ok(Self {
            inner: Aes256Gcm::new(key),
        })
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
        self.inner.encrypt_in_place_detached(nonce, aad, buffer)
    }

    pub fn decrypt_in_place_detached(
        &self,
        nonce: &GenericArray<u8, U12>,
        aad: &[u8],
        buffer: &mut [u8],
        tag: &GenericArray<u8, U16>,
    ) -> Result<(), aes_gcm::aead::Error> {
        self.inner
            .decrypt_in_place_detached(nonce, aad, buffer, tag)
    }
}

pub type CipherU12 = U12;
pub type CipherU16 = U16;
