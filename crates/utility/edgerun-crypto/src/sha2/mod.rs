//! SHA-256 and SHA-384 using x86_64 SHA-NI intrinsics when available.
//!
//! Falls back to portable software implementation when hardware is unavailable.

use crate::error::{CryptoError, Result};
use crate::sha2::{Digest, Sha256, Sha384};

pub use crate::sha2::{Digest, Sha256, Sha384};

pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize()
}

pub fn sha384(data: &[u8]) -> [u8; 48] {
    let mut hasher = Sha384::new();
    hasher.update(data);
    hasher.finalize()
}

pub fn sha512(data: &[u8]) -> [u8; 64] {
    let mut hasher = Sha512::new();
    hasher.update(data);
    hasher.finalize()
}

pub use crate::sha2::Sha512;