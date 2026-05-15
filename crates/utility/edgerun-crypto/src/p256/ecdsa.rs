//! Elliptic Curve Digital Signature Algorithm (ECDSA)
//!
//! This module contains support for computing and verifying ECDSA signatures.
//! To use it, you will need to enable one of the two following Cargo features:
//!
//! - `ecdsa-core`: provides only the [`Signature`] type (which represents an
//!   ECDSA/P-256 signature). Does not require the `arithmetic` feature.
//!   This is useful for 3rd-party crates which wish to use the `Signature`
//!   type for interoperability purposes (particularly in conjunction with the
//!   [`signature::Signer`] trait. Example use cases for this include other
//!   software implementations of ECDSA/P-256 and wrappers for cloud KMS
//!   services or hardware devices (HSM or crypto hardware wallet).
//! - `ecdsa`: provides `ecdsa-core` features plus the [`SigningKey`] and
//!   [`VerifyingKey`] types which natively implement ECDSA/P-256 signing and
//!   verification.
//!
//! ## Signing/Verification Example
//!
//! This example requires the `ecdsa` Cargo feature is enabled:
//!
//! ```ignore
//! # #[cfg(feature = "p256_ecdsa")]
//! # {
//! use p256::{
//!     ecdsa::{SigningKey, Signature, signature::Signer},
//! };
//! use rand_core::OsRng; // requires 'getrandom' feature
//!
//! // Signing
//! let signing_key = SigningKey::random(&mut OsRng); // Serialize with `::to_bytes()`
//! let message = b"ECDSA proves knowledge of a secret number in the context of a single message";
//! let signature: Signature = signing_key.sign(message);
//!
//! // Verification
//! use p256::ecdsa::{VerifyingKey, signature::Verifier};
//!
//! let verifying_key = VerifyingKey::from(&signing_key); // Serialize with `::to_encoded_point()`
//! assert!(verifying_key.verify(message, &signature).is_ok());
//! # }
//! ```ignore

pub use crate::ecdsa_core::signature::{self, Error};

use super::NistP256;

#[cfg(feature = "p256_ecdsa")]
use {
    crate::ecdsa_core::hazmat::{SignPrimitive, VerifyPrimitive},
    crate::p256::{AffinePoint, Scalar},
};

/// ECDSA/P-256 signature (fixed-size)
pub type Signature = crate::ecdsa_core::Signature<NistP256>;

/// ECDSA/P-256 signature (ASN.1 DER encoded)
pub type DerSignature = crate::ecdsa_core::der::Signature<NistP256>;

/// ECDSA/P-256 signing key
#[cfg(feature = "p256_ecdsa")]
pub type SigningKey = crate::ecdsa_core::SigningKey<NistP256>;

/// ECDSA/P-256 verification key (i.e. public key)
#[cfg(feature = "p256_ecdsa")]
pub type VerifyingKey = crate::ecdsa_core::VerifyingKey<NistP256>;

#[cfg(feature = "p256_sha256")]
impl crate::ecdsa_core::hazmat::DigestPrimitive for NistP256 {
    type Digest = crate::sha2::Sha256;
}

#[cfg(feature = "p256_ecdsa")]
impl SignPrimitive<NistP256> for Scalar {}

#[cfg(feature = "p256_ecdsa")]
impl VerifyPrimitive<NistP256> for AffinePoint {}
