#![allow(clippy::all)]

pub use sha2::{Sha256, Sha384, Sha512, Digest};

pub const SHA256_DIGEST_SIZE: usize = 32;
pub const SHA384_DIGEST_SIZE: usize = 48;
pub const SHA512_DIGEST_SIZE: usize = 64;

pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    sha2::Digest::update(&mut hasher, data);
    sha2::Digest::finalize(hasher).into()
}

pub fn sha384(data: &[u8]) -> [u8; 48] {
    let mut hasher = Sha384::new();
    sha2::Digest::update(&mut hasher, data);
    sha2::Digest::finalize(hasher).into()
}

pub fn sha512(data: &[u8]) -> [u8; 64] {
    let mut hasher = Sha512::new();
    sha2::Digest::update(&mut hasher, data);
    sha2::Digest::finalize(hasher).into()
}
