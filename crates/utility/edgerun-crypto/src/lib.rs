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
pub mod certs;
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
#[cfg(feature = "digest")]
pub mod digest;
pub mod error;
#[cfg(any(feature = "digest", feature = "p256"))]
mod generic_array_impl;
#[cfg(any(feature = "ed25519", feature = "p256"))]
pub mod keys;
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
#[cfg(feature = "aead")]
pub mod sealing;
pub mod sha;
#[cfg(any(feature = "ed25519", feature = "p256_sha256", feature = "rsa_sha2"))]
pub mod sha2;
#[cfg(any(feature = "ed25519", feature = "p256", feature = "rsa"))]
pub mod signature;
#[cfg(any(feature = "ed25519", feature = "p256", feature = "rsa"))]
pub mod signing;
#[cfg(any(
    feature = "curve25519",
    feature = "ed25519",
    feature = "p256",
    feature = "rsa"
))]
pub mod subtle;
#[cfg(any(feature = "ed25519", feature = "p256", feature = "rsa"))]
pub mod verification;
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
#[cfg(feature = "ed25519")]
pub use keys::{Ed25519SigningKey, Ed25519VerifyingKey};
#[cfg(feature = "p256")]
pub use keys::{P256SigningKey, P256VerifyingKey};
pub use rng::{
    fill_random, mix_entropy, random_below_u64, random_bytes, random_choice, random_f64,
    random_f64_range, random_i32_range, random_u32, random_u64, random_u128, random_usize_range,
    register_random_source, unregister_random_source,
};

pub use crate::rng::OsRng;

pub use sha::{Sha256, Sha384, Sha512, sha256, sha384, sha512};

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

#[cfg(feature = "ed25519")]
pub fn random_ed25519_key() -> Ed25519SigningKey {
    let mut bytes = [0u8; 32];
    fill_random(&mut bytes).expect("edgerun RNG should always provide fallback bytes");
    Ed25519SigningKey::from_bytes(&bytes)
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

#[cfg(feature = "tpm")]
pub mod tpm {
    pub use edgerun_tpm::*;
}
