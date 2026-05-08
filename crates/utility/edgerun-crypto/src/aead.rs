//! AES-GCM authenticated encryption.

extern crate alloc;

use crate::aes::{Aes128, Aes256};
use alloc::vec::Vec;

pub const NONCE_SIZE: usize = 12;
pub const TAG_SIZE: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Error;

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("AES-GCM error")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Nonce([u8; NONCE_SIZE]);

impl Nonce {
    pub fn from_slice(bytes: &[u8]) -> Self {
        let mut nonce = [0u8; NONCE_SIZE];
        nonce.copy_from_slice(bytes);
        Self(nonce)
    }
}

impl From<[u8; NONCE_SIZE]> for Nonce {
    fn from(value: [u8; NONCE_SIZE]) -> Self {
        Self(value)
    }
}

impl From<&[u8; NONCE_SIZE]> for Nonce {
    fn from(value: &[u8; NONCE_SIZE]) -> Self {
        Self(*value)
    }
}

impl From<&Nonce> for Nonce {
    fn from(value: &Nonce) -> Self {
        *value
    }
}

impl AsRef<[u8]> for Nonce {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tag([u8; TAG_SIZE]);

impl Tag {
    pub fn from_slice(bytes: &[u8]) -> Self {
        let mut tag = [0u8; TAG_SIZE];
        tag.copy_from_slice(bytes);
        Self(tag)
    }

    pub fn len(&self) -> usize {
        TAG_SIZE
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; TAG_SIZE]> for Tag {
    fn from(value: [u8; TAG_SIZE]) -> Self {
        Self(value)
    }
}

impl AsRef<[u8]> for Tag {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl core::ops::Deref for Tag {
    type Target = [u8; TAG_SIZE];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub mod aead {
    pub use super::{Aead, AeadInPlace, Error, KeyInit, Payload};
}

#[derive(Clone, Copy)]
pub struct Payload<'a> {
    pub msg: &'a [u8],
    pub aad: &'a [u8],
}

impl<'a> From<&'a [u8]> for Payload<'a> {
    fn from(msg: &'a [u8]) -> Self {
        Self { msg, aad: &[] }
    }
}

impl<'a, const N: usize> From<&'a [u8; N]> for Payload<'a> {
    fn from(msg: &'a [u8; N]) -> Self {
        Self {
            msg: msg.as_slice(),
            aad: &[],
        }
    }
}

pub trait KeyInit: Sized {
    fn new_from_slice(key: &[u8]) -> Result<Self, Error>;
}

pub trait Aead {
    fn encrypt<'a, P>(&self, nonce: &Nonce, payload: P) -> Result<Vec<u8>, Error>
    where
        P: Into<Payload<'a>>;

    fn decrypt<'a, P>(&self, nonce: &Nonce, payload: P) -> Result<Vec<u8>, Error>
    where
        P: Into<Payload<'a>>;
}

pub trait AeadInPlace {
    fn encrypt_in_place_detached(
        &self,
        nonce: &Nonce,
        aad: &[u8],
        buffer: &mut [u8],
    ) -> Result<Tag, Error>;

    fn decrypt_in_place_detached(
        &self,
        nonce: &Nonce,
        aad: &[u8],
        buffer: &mut [u8],
        tag: &Tag,
    ) -> Result<(), Error>;
}

#[derive(Clone)]
pub struct Aes128Gcm {
    cipher: Aes128,
}

impl KeyInit for Aes128Gcm {
    fn new_from_slice(key: &[u8]) -> Result<Self, Error> {
        if key.len() != 16 {
            return Err(Error);
        }
        let mut key_bytes = [0u8; 16];
        key_bytes.copy_from_slice(key);
        Ok(Self {
            cipher: Aes128::new(&key_bytes),
        })
    }
}

impl Aes128Gcm {
    pub fn encrypt<'a, N, P>(&self, nonce: N, payload: P) -> Result<Vec<u8>, Error>
    where
        N: Into<Nonce>,
        P: Into<Payload<'a>>,
    {
        let nonce = nonce.into();
        encrypt_vec(&BlockCipher::Aes128(&self.cipher), &nonce, payload.into())
    }

    pub fn decrypt<'a, N, P>(&self, nonce: N, payload: P) -> Result<Vec<u8>, Error>
    where
        N: Into<Nonce>,
        P: Into<Payload<'a>>,
    {
        let nonce = nonce.into();
        decrypt_vec(&BlockCipher::Aes128(&self.cipher), &nonce, payload.into())
    }

    pub fn encrypt_in_place_detached<N>(
        &self,
        nonce: N,
        aad: &[u8],
        buffer: &mut [u8],
    ) -> Result<Tag, Error>
    where
        N: Into<Nonce>,
    {
        let nonce = nonce.into();
        encrypt_detached(&BlockCipher::Aes128(&self.cipher), &nonce, aad, buffer)
    }

    pub fn decrypt_in_place_detached<N>(
        &self,
        nonce: N,
        aad: &[u8],
        buffer: &mut [u8],
        tag: &Tag,
    ) -> Result<(), Error>
    where
        N: Into<Nonce>,
    {
        let nonce = nonce.into();
        decrypt_detached(&BlockCipher::Aes128(&self.cipher), &nonce, aad, buffer, tag)
    }
}

#[derive(Clone)]
pub struct Aes256Gcm {
    cipher: Aes256,
}

impl KeyInit for Aes256Gcm {
    fn new_from_slice(key: &[u8]) -> Result<Self, Error> {
        if key.len() != 32 {
            return Err(Error);
        }
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(key);
        Ok(Self {
            cipher: Aes256::new(&key_bytes),
        })
    }
}

impl Aes256Gcm {
    pub fn encrypt<'a, N, P>(&self, nonce: N, payload: P) -> Result<Vec<u8>, Error>
    where
        N: Into<Nonce>,
        P: Into<Payload<'a>>,
    {
        let nonce = nonce.into();
        encrypt_vec(&BlockCipher::Aes256(&self.cipher), &nonce, payload.into())
    }

    pub fn decrypt<'a, N, P>(&self, nonce: N, payload: P) -> Result<Vec<u8>, Error>
    where
        N: Into<Nonce>,
        P: Into<Payload<'a>>,
    {
        let nonce = nonce.into();
        decrypt_vec(&BlockCipher::Aes256(&self.cipher), &nonce, payload.into())
    }

    pub fn encrypt_in_place_detached<N>(
        &self,
        nonce: N,
        aad: &[u8],
        buffer: &mut [u8],
    ) -> Result<Tag, Error>
    where
        N: Into<Nonce>,
    {
        let nonce = nonce.into();
        encrypt_detached(&BlockCipher::Aes256(&self.cipher), &nonce, aad, buffer)
    }

    pub fn decrypt_in_place_detached<N>(
        &self,
        nonce: N,
        aad: &[u8],
        buffer: &mut [u8],
        tag: &Tag,
    ) -> Result<(), Error>
    where
        N: Into<Nonce>,
    {
        let nonce = nonce.into();
        decrypt_detached(&BlockCipher::Aes256(&self.cipher), &nonce, aad, buffer, tag)
    }
}

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
    pub fn new(key: &[u8]) -> Result<Self, Error> {
        match key.len() {
            16 => Ok(Self {
                inner: AesGcmInner::Aes128(Aes128Gcm::new_from_slice(key)?),
            }),
            32 => Ok(Self {
                inner: AesGcmInner::Aes256(Aes256Gcm::new_from_slice(key)?),
            }),
            _ => Err(Error),
        }
    }

    pub fn new_from_key(key: &[u8]) -> Result<Self, Error> {
        Self::new(key)
    }

    pub fn new_from_slice(key: &[u8]) -> Result<Self, Error> {
        Self::new(key)
    }

    pub fn encrypt<'a, N, P>(&self, nonce: N, payload: P) -> Result<Vec<u8>, Error>
    where
        N: Into<Nonce>,
        P: Into<Payload<'a>>,
    {
        let nonce = nonce.into();
        match &self.inner {
            AesGcmInner::Aes128(cipher) => cipher.encrypt(nonce, payload),
            AesGcmInner::Aes256(cipher) => cipher.encrypt(nonce, payload),
        }
    }

    pub fn decrypt<'a, N, P>(&self, nonce: N, payload: P) -> Result<Vec<u8>, Error>
    where
        N: Into<Nonce>,
        P: Into<Payload<'a>>,
    {
        let nonce = nonce.into();
        match &self.inner {
            AesGcmInner::Aes128(cipher) => cipher.decrypt(nonce, payload),
            AesGcmInner::Aes256(cipher) => cipher.decrypt(nonce, payload),
        }
    }

    pub fn encrypt_in_place_detached<N>(
        &self,
        nonce: N,
        aad: &[u8],
        buffer: &mut [u8],
    ) -> Result<Tag, Error>
    where
        N: Into<Nonce>,
    {
        let nonce = nonce.into();
        match &self.inner {
            AesGcmInner::Aes128(cipher) => cipher.encrypt_in_place_detached(nonce, aad, buffer),
            AesGcmInner::Aes256(cipher) => cipher.encrypt_in_place_detached(nonce, aad, buffer),
        }
    }

    pub fn decrypt_in_place_detached<N>(
        &self,
        nonce: N,
        aad: &[u8],
        buffer: &mut [u8],
        tag: &Tag,
    ) -> Result<(), Error>
    where
        N: Into<Nonce>,
    {
        let nonce = nonce.into();
        match &self.inner {
            AesGcmInner::Aes128(cipher) => {
                cipher.decrypt_in_place_detached(nonce, aad, buffer, tag)
            }
            AesGcmInner::Aes256(cipher) => {
                cipher.decrypt_in_place_detached(nonce, aad, buffer, tag)
            }
        }
    }
}

impl KeyInit for Aes256GcmCipher {
    fn new_from_slice(key: &[u8]) -> Result<Self, Error> {
        Self::new(key)
    }
}

impl Aead for Aes128Gcm {
    fn encrypt<'a, P>(&self, nonce: &Nonce, payload: P) -> Result<Vec<u8>, Error>
    where
        P: Into<Payload<'a>>,
    {
        encrypt_vec(&BlockCipher::Aes128(&self.cipher), nonce, payload.into())
    }

    fn decrypt<'a, P>(&self, nonce: &Nonce, payload: P) -> Result<Vec<u8>, Error>
    where
        P: Into<Payload<'a>>,
    {
        decrypt_vec(&BlockCipher::Aes128(&self.cipher), nonce, payload.into())
    }
}

impl AeadInPlace for Aes128Gcm {
    fn encrypt_in_place_detached(
        &self,
        nonce: &Nonce,
        aad: &[u8],
        buffer: &mut [u8],
    ) -> Result<Tag, Error> {
        encrypt_detached(&BlockCipher::Aes128(&self.cipher), nonce, aad, buffer)
    }

    fn decrypt_in_place_detached(
        &self,
        nonce: &Nonce,
        aad: &[u8],
        buffer: &mut [u8],
        tag: &Tag,
    ) -> Result<(), Error> {
        decrypt_detached(&BlockCipher::Aes128(&self.cipher), nonce, aad, buffer, tag)
    }
}

impl Aead for Aes256Gcm {
    fn encrypt<'a, P>(&self, nonce: &Nonce, payload: P) -> Result<Vec<u8>, Error>
    where
        P: Into<Payload<'a>>,
    {
        encrypt_vec(&BlockCipher::Aes256(&self.cipher), nonce, payload.into())
    }

    fn decrypt<'a, P>(&self, nonce: &Nonce, payload: P) -> Result<Vec<u8>, Error>
    where
        P: Into<Payload<'a>>,
    {
        decrypt_vec(&BlockCipher::Aes256(&self.cipher), nonce, payload.into())
    }
}

impl AeadInPlace for Aes256Gcm {
    fn encrypt_in_place_detached(
        &self,
        nonce: &Nonce,
        aad: &[u8],
        buffer: &mut [u8],
    ) -> Result<Tag, Error> {
        encrypt_detached(&BlockCipher::Aes256(&self.cipher), nonce, aad, buffer)
    }

    fn decrypt_in_place_detached(
        &self,
        nonce: &Nonce,
        aad: &[u8],
        buffer: &mut [u8],
        tag: &Tag,
    ) -> Result<(), Error> {
        decrypt_detached(&BlockCipher::Aes256(&self.cipher), nonce, aad, buffer, tag)
    }
}

impl Aead for Aes256GcmCipher {
    fn encrypt<'a, P>(&self, nonce: &Nonce, payload: P) -> Result<Vec<u8>, Error>
    where
        P: Into<Payload<'a>>,
    {
        match &self.inner {
            AesGcmInner::Aes128(cipher) => cipher.encrypt(nonce, payload),
            AesGcmInner::Aes256(cipher) => cipher.encrypt(nonce, payload),
        }
    }

    fn decrypt<'a, P>(&self, nonce: &Nonce, payload: P) -> Result<Vec<u8>, Error>
    where
        P: Into<Payload<'a>>,
    {
        match &self.inner {
            AesGcmInner::Aes128(cipher) => cipher.decrypt(nonce, payload),
            AesGcmInner::Aes256(cipher) => cipher.decrypt(nonce, payload),
        }
    }
}

impl AeadInPlace for Aes256GcmCipher {
    fn encrypt_in_place_detached(
        &self,
        nonce: &Nonce,
        aad: &[u8],
        buffer: &mut [u8],
    ) -> Result<Tag, Error> {
        match &self.inner {
            AesGcmInner::Aes128(cipher) => cipher.encrypt_in_place_detached(nonce, aad, buffer),
            AesGcmInner::Aes256(cipher) => cipher.encrypt_in_place_detached(nonce, aad, buffer),
        }
    }

    fn decrypt_in_place_detached(
        &self,
        nonce: &Nonce,
        aad: &[u8],
        buffer: &mut [u8],
        tag: &Tag,
    ) -> Result<(), Error> {
        match &self.inner {
            AesGcmInner::Aes128(cipher) => {
                cipher.decrypt_in_place_detached(nonce, aad, buffer, tag)
            }
            AesGcmInner::Aes256(cipher) => {
                cipher.decrypt_in_place_detached(nonce, aad, buffer, tag)
            }
        }
    }
}

enum BlockCipher<'a> {
    Aes128(&'a Aes128),
    Aes256(&'a Aes256),
}

impl BlockCipher<'_> {
    fn encrypt_block(&self, block: &[u8; 16]) -> [u8; 16] {
        match self {
            Self::Aes128(cipher) => cipher.encrypt_block(block),
            Self::Aes256(cipher) => cipher.encrypt_block(block),
        }
    }
}

fn encrypt_vec(
    cipher: &BlockCipher<'_>,
    nonce: &Nonce,
    payload: Payload<'_>,
) -> Result<Vec<u8>, Error> {
    let mut out = payload.msg.to_vec();
    let tag = encrypt_detached(cipher, nonce, payload.aad, &mut out)?;
    out.extend_from_slice(tag.as_ref());
    Ok(out)
}

fn decrypt_vec(
    cipher: &BlockCipher<'_>,
    nonce: &Nonce,
    payload: Payload<'_>,
) -> Result<Vec<u8>, Error> {
    if payload.msg.len() < TAG_SIZE {
        return Err(Error);
    }
    let split = payload.msg.len() - TAG_SIZE;
    let mut out = payload.msg[..split].to_vec();
    let tag = Tag::from_slice(&payload.msg[split..]);
    decrypt_detached(cipher, nonce, payload.aad, &mut out, &tag)?;
    Ok(out)
}

fn encrypt_detached(
    cipher: &BlockCipher<'_>,
    nonce: &Nonce,
    aad: &[u8],
    buffer: &mut [u8],
) -> Result<Tag, Error> {
    let hash_key = cipher.encrypt_block(&[0u8; 16]);
    let j0 = initial_counter(nonce);
    apply_ctr(cipher, &j0, buffer);
    let auth = ghash(&hash_key, aad, buffer);
    let encrypted_j0 = cipher.encrypt_block(&j0);
    Ok(Tag(xor16(encrypted_j0, auth)))
}

fn decrypt_detached(
    cipher: &BlockCipher<'_>,
    nonce: &Nonce,
    aad: &[u8],
    buffer: &mut [u8],
    tag: &Tag,
) -> Result<(), Error> {
    let hash_key = cipher.encrypt_block(&[0u8; 16]);
    let j0 = initial_counter(nonce);
    let auth = ghash(&hash_key, aad, buffer);
    let encrypted_j0 = cipher.encrypt_block(&j0);
    let expected = Tag(xor16(encrypted_j0, auth));
    if !ct_eq(expected.as_ref(), tag.as_ref()) {
        return Err(Error);
    }
    apply_ctr(cipher, &j0, buffer);
    Ok(())
}

fn initial_counter(nonce: &Nonce) -> [u8; 16] {
    let mut j0 = [0u8; 16];
    j0[..NONCE_SIZE].copy_from_slice(nonce.as_ref());
    j0[15] = 1;
    j0
}

fn apply_ctr(cipher: &BlockCipher<'_>, j0: &[u8; 16], buffer: &mut [u8]) {
    let mut counter = *j0;
    for chunk in buffer.chunks_mut(16) {
        inc32(&mut counter);
        let stream = cipher.encrypt_block(&counter);
        for (byte, mask) in chunk.iter_mut().zip(stream.iter()) {
            *byte ^= *mask;
        }
    }
}

fn inc32(counter: &mut [u8; 16]) {
    let value = u32::from_be_bytes([counter[12], counter[13], counter[14], counter[15]]);
    counter[12..].copy_from_slice(&value.wrapping_add(1).to_be_bytes());
}

fn ghash(hash_key: &[u8; 16], aad: &[u8], ciphertext: &[u8]) -> [u8; 16] {
    let h = u128::from_be_bytes(*hash_key);
    let mut y = 0u128;
    for block in aad.chunks(16) {
        y ^= padded_u128(block);
        y = gf_mul_gcm(y, h);
    }
    for block in ciphertext.chunks(16) {
        y ^= padded_u128(block);
        y = gf_mul_gcm(y, h);
    }
    let aad_bits = (aad.len() as u64).wrapping_mul(8);
    let ciphertext_bits = (ciphertext.len() as u64).wrapping_mul(8);
    let len_block = ((aad_bits as u128) << 64) | ciphertext_bits as u128;
    y ^= len_block;
    gf_mul_gcm(y, h).to_be_bytes()
}

fn padded_u128(block: &[u8]) -> u128 {
    let mut padded = [0u8; 16];
    padded[..block.len()].copy_from_slice(block);
    u128::from_be_bytes(padded)
}

fn gf_mul_gcm(x: u128, y: u128) -> u128 {
    const R: u128 = 0xe100_0000_0000_0000_0000_0000_0000_0000;
    let mut z = 0u128;
    let mut v = x;
    for i in 0..128 {
        if ((y >> (127 - i)) & 1) != 0 {
            z ^= v;
        }
        if v & 1 == 0 {
            v >>= 1;
        } else {
            v = (v >> 1) ^ R;
        }
    }
    z
}

fn xor16(left: [u8; 16], right: [u8; 16]) -> [u8; 16] {
    let mut out = [0u8; 16];
    for i in 0..16 {
        out[i] = left[i] ^ right[i];
    }
    out
}

fn ct_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0u8;
    for i in 0..left.len() {
        diff |= left[i] ^ right[i];
    }
    diff == 0
}

pub type CipherU12 = Nonce;
pub type CipherU16 = Tag;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aes128_gcm_matches_nist_vector() {
        let key = [0u8; 16];
        let nonce = Nonce::from([0u8; 12]);
        let plaintext = [0u8; 16];
        let expected_ciphertext = [
            0x03, 0x88, 0xda, 0xce, 0x60, 0xb6, 0xa3, 0x92, 0xf3, 0x28, 0xc2, 0xb9, 0x71, 0xb2,
            0xfe, 0x78, 0xab, 0x6e, 0x47, 0xd4, 0x2c, 0xec, 0x13, 0xbd, 0xf5, 0x3a, 0x67, 0xb2,
            0x12, 0x57, 0xbd, 0xdf,
        ];
        let cipher = Aes128Gcm::new_from_slice(&key).unwrap();
        let ciphertext = cipher.encrypt(&nonce, plaintext.as_slice()).unwrap();
        assert_eq!(ciphertext, expected_ciphertext);
        assert_eq!(
            cipher.decrypt(&nonce, ciphertext.as_slice()).unwrap(),
            plaintext
        );
    }

    #[test]
    fn rejects_modified_tag() {
        let key = [7u8; 32];
        let nonce = Nonce::from([9u8; 12]);
        let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
        let mut ciphertext = cipher.encrypt(&nonce, b"message".as_slice()).unwrap();
        let last = ciphertext.len() - 1;
        ciphertext[last] ^= 1;
        assert!(cipher.decrypt(&nonce, ciphertext.as_slice()).is_err());
    }
}
