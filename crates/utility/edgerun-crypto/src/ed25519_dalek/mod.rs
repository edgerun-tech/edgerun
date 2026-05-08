pub use crate::ed25519;

mod constants;
#[cfg(feature = "digest")]
mod context;
mod errors;
mod signature;
mod signing;
mod verifying;

#[cfg(feature = "hazmat")]
pub mod hazmat;
#[cfg(not(feature = "hazmat"))]
mod hazmat;

#[cfg(feature = "digest")]
pub use crate::curve25519_dalek::digest::Digest;
#[cfg(feature = "digest")]
pub use crate::sha2::Sha512;

pub use constants::*;
#[cfg(feature = "digest")]
pub use context::Context;
pub use errors::*;
pub use signing::*;
pub use verifying::*;

#[cfg(feature = "digest")]
pub use crate::ed25519::signature::{DigestSigner, DigestVerifier};
pub use crate::ed25519::signature::{Signer, Verifier};
pub use crate::ed25519::Signature;

#[cfg(feature = "pkcs8")]
pub use crate::ed25519::pkcs8;
