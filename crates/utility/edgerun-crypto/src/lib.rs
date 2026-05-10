#![no_std]
#![allow(clippy::all)]

pub extern crate self as generic_array;
#[cfg(feature = "zeroize")]
pub mod zeroize;

#[cfg(test)]
#[doc(hidden)]
pub const fn __hex_is_ignored(byte: u8) -> bool {
    matches!(byte, b' ' | b'\n' | b'\r' | b'\t')
}

#[cfg(test)]
#[doc(hidden)]
pub const fn __hex_decoded_len(input: &str) -> usize {
    let bytes = input.as_bytes();
    let mut i = 0;
    let mut digits = 0;
    while i < bytes.len() {
        if !__hex_is_ignored(bytes[i]) {
            digits += 1;
        }
        i += 1;
    }
    if digits % 2 != 0 {
        panic!("hex literal length mismatch");
    }
    digits / 2
}

#[cfg(test)]
#[doc(hidden)]
pub const fn __hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        b'A'..=b'F' => byte - b'A' + 10,
        _ => panic!("invalid hex literal"),
    }
}

#[cfg(test)]
#[doc(hidden)]
pub const fn __hex_decode<const N: usize>(input: &str) -> [u8; N] {
    let bytes = input.as_bytes();
    let mut out = [0u8; N];
    let mut input_i = 0;
    let mut out_i = 0;
    while out_i < N {
        while __hex_is_ignored(bytes[input_i]) {
            input_i += 1;
        }
        let high = __hex_nibble(bytes[input_i]);
        input_i += 1;
        while __hex_is_ignored(bytes[input_i]) {
            input_i += 1;
        }
        let low = __hex_nibble(bytes[input_i]);
        input_i += 1;
        out[out_i] = (high << 4) | low;
        out_i += 1;
    }
    out
}

#[cfg(test)]
#[macro_export]
macro_rules! hex {
    ($($input:literal)+) => {{
        const INPUT: &str = concat!($($input),+);
        const LEN: usize = $crate::__hex_decoded_len(INPUT);
        const BYTES: [u8; LEN] = $crate::__hex_decode::<LEN>(INPUT);
        BYTES
    }};
}

#[macro_use]
pub extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

#[macro_export]
macro_rules! cfg_if {
    (
        if #[cfg( $($i_meta:tt)+ )] { $( $i_tokens:tt )* }
        $(
            else if #[cfg( $($ei_meta:tt)+ )] { $( $ei_tokens:tt )* }
        )*
        $(
            else { $( $e_tokens:tt )* }
        )?
    ) => {
        $crate::cfg_if! {
            @__items () ;
            (( $($i_meta)+ ) ( $( $i_tokens )* )),
            $(
                (( $($ei_meta)+ ) ( $( $ei_tokens )* )),
            )*
            $(
                (() ( $( $e_tokens )* )),
            )?
        }
    };
    (@__items ( $( ($($_:tt)*) , )* ) ; ) => {};
    (
        @__items ( $( ($($no:tt)+) , )* ) ;
        (( $( $($yes:tt)+ )? ) ( $( $tokens:tt )* )),
        $( $rest:tt , )*
    ) => {
        #[cfg(all(
            $( $($yes)+ , )?
            not(any( $( $($no)+ ),* ))
        ))]
        $crate::cfg_if! { @__temp_group $( $tokens )* }

        $crate::cfg_if! {
            @__items ( $( ($($no)+) , )* $( ($($yes)+) , )? ) ;
            $( $rest , )*
        }
    };
    (@__temp_group $( $tokens:tt )* ) => {
        $( $tokens )*
    };
}

#[cfg(feature = "aead")]
pub mod aead;
#[cfg(feature = "aes")]
pub mod aes;
#[cfg(feature = "p256")]
pub(crate) mod base16ct;
#[cfg(any(feature = "digest", feature = "p256", feature = "rsa"))]
pub mod const_oid;
#[cfg(any(feature = "sha2_impl", feature = "curve25519"))]
pub(crate) mod cpu_features;
#[cfg(feature = "p256")]
pub mod crypto_bigint;
#[cfg(any(feature = "ed25519", feature = "x25519"))]
pub mod curve25519_dalek;
#[cfg(any(feature = "p256", feature = "rsa"))]
pub mod der;
#[cfg(feature = "des")]
pub mod des;
#[cfg(feature = "digest")]
pub mod digest;
pub mod error;
#[cfg(any(feature = "digest", feature = "p256"))]
mod generic_array_impl;
#[cfg(feature = "rsa")]
pub mod num_bigint;
#[cfg(any(feature = "p256", feature = "rsa"))]
pub mod pem_rfc7468;
#[cfg(any(feature = "p256", feature = "rsa"))]
pub mod rand_core;
#[cfg(all(test, any(feature = "p256", feature = "rsa")))]
pub(crate) mod test_rng {
    use crate::rand_core::{CryptoRng, Error, RngCore};

    pub struct ChaCha8Rng(u64);
    pub type ChaChaRng = ChaCha8Rng;

    impl ChaCha8Rng {
        pub fn from_seed(seed: [u8; 32]) -> Self {
            let mut state = 0x9e37_79b9_7f4a_7c15u64;
            for chunk in seed.chunks(8) {
                let mut word = [0u8; 8];
                word[..chunk.len()].copy_from_slice(chunk);
                state ^= u64::from_le_bytes(word);
                state = next(state);
            }
            Self(state)
        }

        pub fn seed_from_u64(seed: u64) -> Self {
            Self(next(seed ^ 0x9e37_79b9_7f4a_7c15))
        }

        fn next_u64_inner(&mut self) -> u64 {
            self.0 = next(self.0);
            self.0
        }
    }

    impl RngCore for ChaCha8Rng {
        fn next_u32(&mut self) -> u32 {
            self.next_u64_inner() as u32
        }

        fn next_u64(&mut self) -> u64 {
            self.next_u64_inner()
        }

        fn fill_bytes(&mut self, dest: &mut [u8]) {
            for chunk in dest.chunks_mut(8) {
                let word = self.next_u64_inner().to_le_bytes();
                chunk.copy_from_slice(&word[..chunk.len()]);
            }
        }

        fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
            self.fill_bytes(dest);
            Ok(())
        }
    }

    impl CryptoRng for ChaCha8Rng {}

    fn next(mut x: u64) -> u64 {
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
}
pub mod rng;
pub mod sha;
#[cfg(any(feature = "ed25519", feature = "p256_sha256", feature = "rsa_sha2"))]
pub mod sha2;
#[cfg(any(feature = "ed25519", feature = "p256", feature = "rsa"))]
pub mod signature;
#[cfg(any(feature = "ed25519", feature = "p256", feature = "rsa"))]
pub mod subtle;
#[cfg(feature = "x25519")]
pub mod x25519;

#[cfg(feature = "p256")]
mod ecdsa_core;
#[cfg(feature = "ed25519")]
pub mod ed25519;
#[cfg(feature = "ed25519")]
pub mod ed25519_dalek;
#[cfg(feature = "p256")]
pub mod elliptic_curve;
#[cfg(feature = "p256")]
pub mod ff;
#[cfg(feature = "p256")]
pub mod group;
#[cfg(feature = "p256")]
pub mod p256;
#[cfg(feature = "rsa")]
pub mod pkcs1;
#[cfg(any(feature = "p256", feature = "rsa"))]
pub mod pkcs8;
#[cfg(feature = "p256")]
pub mod primeorder;
#[cfg(feature = "rsa")]
pub mod rsa;
#[cfg(feature = "p256")]
pub mod sec1;
#[cfg(any(feature = "p256", feature = "rsa"))]
pub mod spki;
#[cfg(any(feature = "digest", feature = "p256"))]
pub mod typenum;

#[cfg(all(feature = "zeroize", feature = "rsa"))]
mod zeroize_num_bigint;

use crate::error::{CryptoError, Result};

#[cfg(any(feature = "digest", feature = "p256"))]
pub use generic_array_impl::*;
#[cfg(any(feature = "digest", feature = "p256"))]
pub use typenum::*;

#[cfg(feature = "aead")]
pub use aead::Aes256Gcm as AesGcmCipher;
#[cfg(feature = "aead")]
pub use aead::{
    Aead, AeadInPlace, Aes128Gcm, Aes256Gcm, Aes256GcmCipher, CipherU12, CipherU16, KeyInit, Nonce,
    Payload, Tag,
};

#[cfg(feature = "aead")]
pub mod aes_gcm {
    pub use crate::aead::{Aes128Gcm, Aes256Gcm, Error, Nonce, Tag};

    pub mod aead {
        pub use crate::aead::{Aead, AeadInPlace, Error, KeyInit, Payload};

        pub mod generic_array {
            pub struct GenericArray;

            impl GenericArray {
                pub fn from_slice(bytes: &[u8]) -> crate::aead::Tag {
                    crate::aead::Tag::from_slice(bytes)
                }
            }
        }
    }
}
#[cfg(feature = "p256")]
pub use crate::p256::ecdsa::SigningKey;
#[cfg(feature = "ed25519")]
pub use ed25519_dalek::Signer as Ed25519Signer;
#[cfg(feature = "ed25519")]
pub use ed25519_dalek::Signer;
#[cfg(feature = "ed25519")]
pub use ed25519_dalek::SigningKey as Ed25519SigningKey;
pub use rng::{fill_random, mix_entropy, random_bytes, random_u32, random_u64};

pub use crate::rng::OsRng;

pub use sha::{Sha256, Sha384, Sha512, sha256, sha384, sha512};

#[cfg(feature = "hmac")]
pub fn hmac_sha256(key: &[u8], data: &[u8]) -> alloc::vec::Vec<u8> {
    hmac_sha256_array(key, data).to_vec()
}

#[cfg(feature = "hmac")]
pub fn hmac_sha384(key: &[u8], data: &[u8]) -> alloc::vec::Vec<u8> {
    hmac_sha384_array(key, data).to_vec()
}

#[cfg(feature = "hmac")]
fn hmac_sha256_array(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut normalized = [0u8; 64];
    if key.len() > 64 {
        normalized[..32].copy_from_slice(&sha256(key));
    } else {
        normalized[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5cu8; 64];
    for index in 0..64 {
        ipad[index] ^= normalized[index];
        opad[index] ^= normalized[index];
    }
    let mut inner = alloc::vec::Vec::with_capacity(64 + data.len());
    inner.extend_from_slice(&ipad);
    inner.extend_from_slice(data);
    let inner_digest = sha256(&inner);
    let mut outer = alloc::vec::Vec::with_capacity(64 + inner_digest.len());
    outer.extend_from_slice(&opad);
    outer.extend_from_slice(&inner_digest);
    sha256(&outer)
}

#[cfg(feature = "hmac")]
fn hmac_sha384_array(key: &[u8], data: &[u8]) -> [u8; 48] {
    let mut normalized = [0u8; 128];
    if key.len() > 128 {
        normalized[..48].copy_from_slice(&sha384(key));
    } else {
        normalized[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; 128];
    let mut opad = [0x5cu8; 128];
    for index in 0..128 {
        ipad[index] ^= normalized[index];
        opad[index] ^= normalized[index];
    }
    let mut inner = alloc::vec::Vec::with_capacity(128 + data.len());
    inner.extend_from_slice(&ipad);
    inner.extend_from_slice(data);
    let inner_digest = sha384(&inner);
    let mut outer = alloc::vec::Vec::with_capacity(128 + inner_digest.len());
    outer.extend_from_slice(&opad);
    outer.extend_from_slice(&inner_digest);
    sha384(&outer)
}

#[cfg(feature = "hmac")]
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

#[cfg(feature = "hmac")]
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

#[cfg(feature = "hmac")]
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

#[cfg(feature = "hmac")]
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

#[cfg(feature = "p256")]
pub fn random_p256_signing_key() -> crate::p256::ecdsa::SigningKey {
    use crate::p256::ecdsa::SigningKey;
    loop {
        let mut bytes = [0u8; 32];
        fill_random(&mut bytes).expect("edgerun RNG should always provide fallback bytes");
        if let Ok(key) = SigningKey::from_bytes(&bytes.into()) {
            return key;
        }
    }
}

#[cfg(feature = "p256")]
pub fn random_p256_ephemeral_secret() -> crate::p256::ecdh::EphemeralSecret {
    let mut rng = p256_rng::P256Rng;
    crate::p256::ecdh::EphemeralSecret::random(&mut rng)
}

#[cfg(feature = "ed25519")]
pub fn random_ed25519_signing_key() -> ed25519_dalek::SigningKey {
    let mut bytes = [0u8; 32];
    fill_random(&mut bytes).expect("edgerun RNG should always provide fallback bytes");
    ed25519_dalek::SigningKey::from_bytes(&bytes)
}

#[cfg(feature = "rsa")]
pub fn random_rsa_private_key(
    bit_size: usize,
) -> core::result::Result<crate::rsa::RsaPrivateKey, crate::rsa::errors::Error> {
    let mut rng = rsa_rng::RsaRng;
    crate::rsa::RsaPrivateKey::new(&mut rng, bit_size)
}

#[cfg(feature = "rsa")]
pub fn rsa_pss_sha256_sign(
    private_key: crate::rsa::RsaPrivateKey,
    msg: &[u8],
) -> crate::rsa::pss::Signature {
    use crate::rsa::signature::RandomizedSigner;

    let signing_key = crate::rsa::pss::SigningKey::<crate::rsa::sha2::Sha256>::new(private_key);
    let mut rng = rsa_rng::RsaRng;
    signing_key.sign_with_rng(&mut rng, msg)
}

#[cfg(feature = "p256")]
mod p256_rng {
    use core::num::NonZeroU32;

    const RNG_ERROR_CODE: u32 = crate::p256::elliptic_curve::rand_core::Error::CUSTOM_START + 1;

    pub struct P256Rng;

    impl crate::p256::elliptic_curve::rand_core::RngCore for P256Rng {
        fn next_u32(&mut self) -> u32 {
            crate::random_u32().unwrap_or(0)
        }

        fn next_u64(&mut self) -> u64 {
            crate::random_u64().unwrap_or(0)
        }

        fn fill_bytes(&mut self, dest: &mut [u8]) {
            let _ = crate::fill_random(dest);
        }

        fn try_fill_bytes(
            &mut self,
            dest: &mut [u8],
        ) -> core::result::Result<(), crate::p256::elliptic_curve::rand_core::Error> {
            crate::fill_random(dest).map_err(|_| {
                crate::p256::elliptic_curve::rand_core::Error::from(
                    NonZeroU32::new(RNG_ERROR_CODE).unwrap(),
                )
            })
        }
    }

    impl crate::p256::elliptic_curve::rand_core::CryptoRng for P256Rng {}
}

#[cfg(feature = "rsa")]
mod rsa_rng {
    use core::num::NonZeroU32;

    const RNG_ERROR_CODE: u32 = crate::rsa::rand_core::Error::CUSTOM_START + 1;

    pub struct RsaRng;

    impl crate::rsa::rand_core::RngCore for RsaRng {
        fn next_u32(&mut self) -> u32 {
            crate::random_u32().unwrap_or(0)
        }

        fn next_u64(&mut self) -> u64 {
            crate::random_u64().unwrap_or(0)
        }

        fn fill_bytes(&mut self, dest: &mut [u8]) {
            let _ = crate::fill_random(dest);
        }

        fn try_fill_bytes(
            &mut self,
            dest: &mut [u8],
        ) -> core::result::Result<(), crate::rsa::rand_core::Error> {
            crate::fill_random(dest).map_err(|_| {
                crate::rsa::rand_core::Error::from(NonZeroU32::new(RNG_ERROR_CODE).unwrap())
            })
        }
    }

    impl crate::rsa::rand_core::CryptoRng for RsaRng {}
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

#[cfg(feature = "hmac")]
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

#[cfg(feature = "hmac")]
pub mod hmac {
    pub use crate::hmac_sha256 as HMAC;
}

#[cfg(feature = "sha1")]
pub mod sha1 {
    pub use crate::sha::{Digest, Sha1};
}

pub fn load_cert_and_key_from_pem(
    cert_pem: &str,
    key_pem: &str,
) -> Result<(alloc::vec::Vec<u8>, alloc::vec::Vec<u8>)> {
    let cert = x509_cert_from_pem(cert_pem).ok_or(CryptoError::InvalidKey)?;
    let key = pem_block(key_pem, "PRIVATE KEY").ok_or(CryptoError::InvalidKey)?;
    Ok((cert, key))
}

#[cfg(feature = "p256")]
pub fn generate_self_signed(key: &SigningKey, cn: &str) -> alloc::vec::Vec<u8> {
    generate_self_signed_for_names(key, &[cn])
}

#[cfg(feature = "p256")]
pub fn generate_self_signed_for_names(key: &SigningKey, names: &[&str]) -> alloc::vec::Vec<u8> {
    use crate::p256::ecdsa::signature::hazmat::PrehashSigner;
    use crate::p256::elliptic_curve::sec1::ToEncodedPoint;

    let common_name = names.first().copied().unwrap_or("localhost");
    let subject = x509_name(common_name);
    let issuer = subject.clone();
    let public_key = key.verifying_key().to_encoded_point(false);
    let public_key = public_key.as_bytes();

    let not_before = current_unix_secs().saturating_sub(60);
    let not_after = not_before.saturating_add(365 * 24 * 60 * 60);

    let serial = random_serial();
    let sig_alg = seq(concat(&[oid(&[
        0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02,
    ])]));
    let spki_alg = seq(concat(&[
        oid(&[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01]),
        oid(&[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07]),
    ]));
    let spki = seq(concat(&[spki_alg, bit_string(public_key)]));

    let validity = seq(concat(&[utc_time(not_before), utc_time(not_after)]));
    let san_ext = subject_alt_name_extension(names);
    let extensions = tagged(0xa3, seq(concat(&[san_ext])));

    let tbs = seq(concat(&[
        tagged(0xa0, integer(&[2])),
        integer(&serial),
        sig_alg.clone(),
        issuer,
        validity,
        subject,
        spki,
        extensions,
    ]));

    let digest = sha256(&tbs);
    let signature: crate::p256::ecdsa::Signature = key
        .sign_prehash(&digest)
        .expect("P-256 ECDSA signing should accept SHA-256 prehash");
    seq(concat(&[
        tbs,
        sig_alg,
        bit_string(signature.to_der().as_bytes()),
    ]))
}

#[cfg(feature = "p256")]
pub fn generate_csr_for_names(key: &SigningKey, names: &[&str]) -> alloc::vec::Vec<u8> {
    use crate::p256::ecdsa::signature::hazmat::PrehashSigner;
    use crate::p256::elliptic_curve::sec1::ToEncodedPoint;

    let common_name = names.first().copied().unwrap_or("localhost");
    let subject = x509_name(common_name);
    let public_key = key.verifying_key().to_encoded_point(false);
    let public_key = public_key.as_bytes();
    let spki_alg = seq(concat(&[
        oid(&[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01]),
        oid(&[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07]),
    ]));
    let spki = seq(concat(&[spki_alg, bit_string(public_key)]));
    let extension_request = seq(concat(&[
        oid(&[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x09, 0x0e]),
        set(seq(concat(&[subject_alt_name_extension(names)]))),
    ]));
    let attributes = tagged(0xa0, extension_request);
    let certification_request_info = seq(concat(&[integer(&[0]), subject, spki, attributes]));
    let sig_alg = seq(concat(&[oid(&[
        0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02,
    ])]));
    let digest = sha256(&certification_request_info);
    let signature: crate::p256::ecdsa::Signature = key
        .sign_prehash(&digest)
        .expect("P-256 ECDSA signing should accept SHA-256 prehash");
    seq(concat(&[
        certification_request_info,
        sig_alg,
        bit_string(signature.to_der().as_bytes()),
    ]))
}

#[cfg(feature = "p256")]
pub fn generate_self_signed_pem(key: &SigningKey, cn: &str) -> alloc::string::String {
    pem_encode(&generate_self_signed(key, cn))
}

#[cfg(feature = "p256")]
pub fn p256_signing_key_from_pem(pem: &str) -> Option<SigningKey> {
    let der = pem_block(pem, "PRIVATE KEY").or_else(|| pem_block(pem, "EC PRIVATE KEY"))?;
    p256_signing_key_from_der(&der)
}
#[cfg(feature = "p256")]
pub fn p256_signing_key_from_der(der: &[u8]) -> Option<SigningKey> {
    if der.len() == 32 {
        return SigningKey::from_bytes(der.into()).ok();
    }

    let mut i = 0;
    while i + 34 <= der.len() {
        if der[i] == 0x04 && der[i + 1] == 0x20 {
            let bytes = &der[i + 2..i + 34];
            if let Ok(key) = SigningKey::from_bytes(bytes.into()) {
                return Some(key);
            }
        }
        i += 1;
    }
    None
}
#[cfg(feature = "p256")]
pub fn p256_signing_key_to_pem(key: &SigningKey) -> alloc::string::String {
    pem_encode_labeled("PRIVATE KEY", &p256_private_key_info_der(key))
}

#[cfg(feature = "p256")]
pub fn p256_signing_key_to_der(key: &SigningKey) -> alloc::vec::Vec<u8> {
    p256_private_key_info_der(key)
}
pub fn pem_encode(data: &[u8]) -> alloc::string::String {
    pem_encode_labeled("CERTIFICATE", data)
}
pub fn x509_cert_from_pem(pem: &str) -> Option<alloc::vec::Vec<u8>> {
    pem_block(pem, "CERTIFICATE")
}

fn der_len(len: usize) -> alloc::vec::Vec<u8> {
    if len < 128 {
        return alloc::vec![len as u8];
    }
    let mut tmp = [0u8; core::mem::size_of::<usize>()];
    let mut value = len;
    let mut pos = tmp.len();
    while value > 0 {
        pos -= 1;
        tmp[pos] = value as u8;
        value >>= 8;
    }
    let len_len = tmp.len() - pos;
    let mut out = alloc::vec::Vec::with_capacity(1 + len_len);
    out.push(0x80 | len_len as u8);
    out.extend_from_slice(&tmp[pos..]);
    out
}

fn tlv(tag: u8, value: &[u8]) -> alloc::vec::Vec<u8> {
    let mut out = alloc::vec::Vec::with_capacity(1 + value.len() + 5);
    out.push(tag);
    out.extend_from_slice(&der_len(value.len()));
    out.extend_from_slice(value);
    out
}

fn seq(value: alloc::vec::Vec<u8>) -> alloc::vec::Vec<u8> {
    tlv(0x30, &value)
}

fn set(value: alloc::vec::Vec<u8>) -> alloc::vec::Vec<u8> {
    tlv(0x31, &value)
}

fn tagged(tag: u8, value: alloc::vec::Vec<u8>) -> alloc::vec::Vec<u8> {
    tlv(tag, &value)
}

fn integer(bytes: &[u8]) -> alloc::vec::Vec<u8> {
    let mut start = 0;
    while start + 1 < bytes.len() && bytes[start] == 0 {
        start += 1;
    }
    let mut value = alloc::vec::Vec::from(&bytes[start..]);
    if value.is_empty() {
        value.push(0);
    }
    if value[0] & 0x80 != 0 {
        value.insert(0, 0);
    }
    tlv(0x02, &value)
}

fn oid(encoded_body: &[u8]) -> alloc::vec::Vec<u8> {
    tlv(0x06, encoded_body)
}

fn utf8_string(value: &str) -> alloc::vec::Vec<u8> {
    tlv(0x0c, value.as_bytes())
}

fn ia5_string_tagged(tag: u8, value: &str) -> alloc::vec::Vec<u8> {
    tlv(tag, value.as_bytes())
}

fn bit_string(value: &[u8]) -> alloc::vec::Vec<u8> {
    let mut body = alloc::vec::Vec::with_capacity(value.len() + 1);
    body.push(0);
    body.extend_from_slice(value);
    tlv(0x03, &body)
}

fn octet_string(value: &[u8]) -> alloc::vec::Vec<u8> {
    tlv(0x04, value)
}

fn concat(parts: &[alloc::vec::Vec<u8>]) -> alloc::vec::Vec<u8> {
    let len = parts.iter().map(|p| p.len()).sum();
    let mut out = alloc::vec::Vec::with_capacity(len);
    for part in parts {
        out.extend_from_slice(part);
    }
    out
}

#[cfg(feature = "p256")]
fn x509_name(common_name: &str) -> alloc::vec::Vec<u8> {
    seq(concat(&[set(seq(concat(&[
        oid(&[0x55, 0x04, 0x03]),
        utf8_string(common_name),
    ])))]))
}

#[cfg(feature = "p256")]
fn subject_alt_name_extension(names: &[&str]) -> alloc::vec::Vec<u8> {
    let dns_names = if names.is_empty() {
        alloc::vec!["localhost"]
    } else {
        names.to_vec()
    };
    let mut general_names = alloc::vec::Vec::new();
    for name in dns_names {
        general_names.extend_from_slice(&ia5_string_tagged(0x82, name));
    }
    let san_der = seq(general_names);
    seq(concat(&[oid(&[0x55, 0x1d, 0x11]), octet_string(&san_der)]))
}

#[cfg(feature = "p256")]
fn random_serial() -> [u8; 16] {
    let mut serial = [0u8; 16];
    let _ = fill_random(&mut serial);
    serial[0] &= 0x7f;
    if serial.iter().all(|b| *b == 0) {
        serial[15] = 1;
    }
    serial
}

#[cfg(feature = "p256")]
fn current_unix_secs() -> u64 {
    #[cfg(feature = "std")]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(1_704_067_200)
    }
    #[cfg(not(feature = "std"))]
    {
        1_704_067_200
    }
}

fn utc_time(unix_secs: u64) -> alloc::vec::Vec<u8> {
    let days = (unix_secs / 86_400) as i64;
    let seconds = unix_secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hour = seconds / 3_600;
    let minute = (seconds % 3_600) / 60;
    let second = seconds % 60;
    let year = (year % 100) as u64;
    let mut value = alloc::string::String::new();
    push_two_digits(&mut value, year);
    push_two_digits(&mut value, month as u64);
    push_two_digits(&mut value, day as u64);
    push_two_digits(&mut value, hour);
    push_two_digits(&mut value, minute);
    push_two_digits(&mut value, second);
    value.push('Z');
    tlv(0x17, value.as_bytes())
}

fn push_two_digits(out: &mut alloc::string::String, value: u64) {
    out.push((b'0' + ((value / 10) % 10) as u8) as char);
    out.push((b'0' + (value % 10) as u8) as char);
}

fn civil_from_days(days_since_epoch: i64) -> (i32, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if m <= 2 { 1 } else { 0 };
    (year as i32, m as u32, d as u32)
}

#[cfg(feature = "p256")]
fn p256_private_key_info_der(key: &SigningKey) -> alloc::vec::Vec<u8> {
    use crate::p256::elliptic_curve::sec1::ToEncodedPoint;

    let private_key = key.to_bytes();
    let public_key = key.verifying_key().to_encoded_point(false);
    let ec_private_key = seq(concat(&[
        integer(&[1]),
        octet_string(private_key.as_slice()),
        tagged(0xa1, bit_string(public_key.as_bytes())),
    ]));
    let alg = seq(concat(&[
        oid(&[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01]),
        oid(&[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07]),
    ]));
    seq(concat(&[integer(&[0]), alg, octet_string(&ec_private_key)]))
}

fn pem_encode_labeled(label: &str, data: &[u8]) -> alloc::string::String {
    let encoded = base64_encode(data);
    let mut out = alloc::string::String::new();
    out.push_str("-----BEGIN ");
    out.push_str(label);
    out.push_str("-----\n");
    for chunk in encoded.as_bytes().chunks(64) {
        for byte in chunk {
            out.push(*byte as char);
        }
        out.push('\n');
    }
    out.push_str("-----END ");
    out.push_str(label);
    out.push_str("-----");
    out
}

fn pem_block(pem: &str, label: &str) -> Option<alloc::vec::Vec<u8>> {
    let mut begin = alloc::string::String::from("-----BEGIN ");
    begin.push_str(label);
    begin.push_str("-----");
    let mut end = alloc::string::String::from("-----END ");
    end.push_str(label);
    end.push_str("-----");

    let start = pem.find(&begin)? + begin.len();
    let rest = &pem[start..];
    let stop = rest.find(&end)?;
    let body = &rest[..stop];
    let mut compact = alloc::string::String::new();
    for ch in body.chars() {
        if !ch.is_whitespace() {
            compact.push(ch);
        }
    }
    base64_decode(&compact)
}

fn base64_encode(data: &[u8]) -> alloc::string::String {
    edgerun_encoding::base64::standard_encode(data)
}

fn base64_decode(input: &str) -> Option<alloc::vec::Vec<u8>> {
    edgerun_encoding::base64::standard_decode(input).ok()
}

#[cfg(feature = "tpm")]
pub mod tpm {
    pub use edgerun_tpm::*;
}
