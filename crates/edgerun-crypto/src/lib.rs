#![no_std]
#![cfg_attr(target_arch = "x86_64", target_feature = "aes,sse2")]
#![cfg_attr(target_arch = "x86_64", target_feature = "sha,sse2")]
#![allow(clippy::all)]

extern crate alloc;

pub mod error;

pub mod aes;
pub mod sha2;
pub mod ecdsa;
pub mod rng;
pub mod hmac;
pub mod hkdf;
pub mod aead;
pub mod signing;

pub use error::CryptoError;

pub type Result<T> = core::result::Result<T, CryptoError>;

pub const SHA256_OUTPUT_LEN: usize = 32;
pub const SHA384_OUTPUT_LEN: usize = 48;
pub const AES128_KEY_LEN: usize = 16;
pub const AES256_KEY_LEN: usize = 32;
pub const AES_GCM_NONCE_LEN: usize = 12;
pub const AES_GCM_TAG_LEN: usize = 16;
pub const ECDSA_P256_SIGNATURE_LEN: usize = 64;
pub const ECDSA_P256_PUBLIC_KEY_LEN: usize = 64;