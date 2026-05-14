//! Low-level interfaces to ed25519 functions
//!
//! # ⚠️ Warning: Hazmat
//!
//! These primitives are easy-to-misuse low-level interfaces.
//!
//! If you are an end user / non-expert in cryptography, **do not use any of these functions**.
//! Failure to use them correctly can lead to catastrophic failures including **full private key
//! recovery.**

// Permit dead code because 1) this module is only public when the `hazmat` feature is set, and 2)
// even without `hazmat` we still need this module because this is where `ExpandedSecretKey` is
// defined.
#![allow(dead_code)]

use core::fmt::Debug;

use crate::ed25519_dalek::{InternalError, SignatureError};

use crate::curve25519_dalek::scalar::{Scalar, clamp_integer};

use crate::subtle::{Choice, ConstantTimeEq};
#[cfg(feature = "zeroize")]
use crate::zeroize::{Zeroize, ZeroizeOnDrop};

// These are used in the functions that are made public when the hazmat feature is set
use crate::curve25519_dalek::digest::{Digest, generic_array::typenum::U64};
use crate::ed25519_dalek::{Signature, VerifyingKey};

/// Contains the secret scalar and domain separator used for generating signatures.
///
/// This is used internally for signing.
///
/// In the usual Ed25519 signing algorithm, `scalar` and `hash_prefix` are defined such that
/// `scalar || hash_prefix = H(sk)` where `sk` is the signing key and `H` is SHA-512.
/// **WARNING:** Deriving the values for these fields in any other way can lead to full key
/// recovery, as documented in [`raw_sign`] and [`raw_sign_prehashed`].
///
/// Instances of this secret are automatically overwritten with zeroes when they fall out of scope.
pub struct ExpandedSecretKey {
    /// The secret scalar used for signing
    pub scalar: Scalar,
    /// The domain separator used when hashing the message to generate the pseudorandom `r` value
    pub hash_prefix: [u8; 32],
}

impl Debug for ExpandedSecretKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ExpandedSecretKey").finish_non_exhaustive() // avoids printing secrets
    }
}

impl ConstantTimeEq for ExpandedSecretKey {
    fn ct_eq(&self, other: &Self) -> Choice {
        self.scalar.to_bytes().ct_eq(&other.scalar.to_bytes())
            & self.hash_prefix.ct_eq(&other.hash_prefix)
    }
}

impl PartialEq for ExpandedSecretKey {
    fn eq(&self, other: &Self) -> bool {
        self.ct_eq(other).into()
    }
}

impl Eq for ExpandedSecretKey {}

#[cfg(feature = "zeroize")]
impl Drop for ExpandedSecretKey {
    fn drop(&mut self) {
        self.scalar.zeroize();
        self.hash_prefix.zeroize()
    }
}

#[cfg(feature = "zeroize")]
impl ZeroizeOnDrop for ExpandedSecretKey {}

// Some conversion methods for `ExpandedSecretKey`. The signing methods are defined in
// `signing.rs`, since we need them even when `not(feature = "hazmat")`
impl ExpandedSecretKey {
    /// Construct an `ExpandedSecretKey` from an array of 64 bytes. In the spec, the bytes are the
    /// output of a SHA-512 hash. This clamps the first 32 bytes and uses it as a scalar, and uses
    /// the second 32 bytes as a domain separator for hashing.
    pub fn from_bytes(bytes: &[u8; 64]) -> Self {
        // TODO: Use bytes.split_array_ref once it’s in MSRV.
        let mut scalar_bytes: [u8; 32] = [0u8; 32];
        let mut hash_prefix: [u8; 32] = [0u8; 32];
        scalar_bytes.copy_from_slice(&bytes[00..32]);
        hash_prefix.copy_from_slice(&bytes[32..64]);

        // For signing, we'll need the integer, clamped, and converted to a Scalar. See
        // PureEdDSA.keygen in RFC 8032 Appendix A.
        let scalar = Scalar::from_bytes_mod_order(clamp_integer(scalar_bytes));

        ExpandedSecretKey {
            scalar,
            hash_prefix,
        }
    }

    /// Construct an `ExpandedSecretKey` from a slice of 64 bytes.
    ///
    /// # Returns
    ///
    /// A `Result` whose okay value is an EdDSA `ExpandedSecretKey` or whose error value is an
    /// `SignatureError` describing the error that occurred, namely that the given slice's length
    /// is not 64.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, SignatureError> {
        // Try to coerce bytes to a [u8; 64]
        bytes.try_into().map(Self::from_bytes).map_err(|_| {
            InternalError::BytesLength {
                name: "ExpandedSecretKey",
                length: 64,
            }
            .into()
        })
    }
}

impl TryFrom<&[u8]> for ExpandedSecretKey {
    type Error = SignatureError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::from_slice(bytes)
    }
}

/// Compute an ordinary Ed25519 signature over the given message. `CtxDigest` is the digest used to
/// calculate the pseudorandomness needed for signing. According to the Ed25519 spec, `CtxDigest =
/// Sha512`.
///
/// # ⚠️  Cryptographically Unsafe
///
/// Do NOT use this function unless you absolutely must. Using the wrong values in
/// `ExpandedSecretKey` can leak your signing key. See
/// [here](https://github.com/MystenLabs/ed25519-unsafe-libs) for more details on this attack.
pub fn raw_sign<CtxDigest>(
    esk: &ExpandedSecretKey,
    message: &[u8],
    verifying_key: &VerifyingKey,
) -> Signature
where
    CtxDigest: Digest<OutputSize = U64>,
{
    esk.raw_sign::<CtxDigest>(message, verifying_key)
}

/// Compute a signature over the given prehashed message, the Ed25519ph algorithm defined in
/// [RFC8032 §5.1][rfc8032]. `MsgDigest` is the digest function used to hash the signed message.
/// `CtxDigest` is the digest function used to calculate the pseudorandomness needed for signing.
/// According to the Ed25519 spec, `MsgDigest = CtxDigest = Sha512`.
///
/// # ⚠️  Cryptographically Unsafe
//
/// Do NOT use this function unless you absolutely must. Using the wrong values in
/// `ExpandedSecretKey` can leak your signing key. See
/// [here](https://github.com/MystenLabs/ed25519-unsafe-libs) for more details on this attack.
///
/// # Inputs
///
/// * `esk` is the [`ExpandedSecretKey`] being used for signing
/// * `prehashed_message` is an instantiated hash digest with 512-bits of
///   output which has had the message to be signed previously fed into its
///   state.
/// * `verifying_key` is a [`VerifyingKey`] which corresponds to this secret key.
/// * `context` is an optional context string, up to 255 bytes inclusive,
///   which may be used to provide additional domain separation.  If not
///   set, this will default to an empty string.
///
/// `scalar` and `hash_prefix` are usually selected such that `scalar || hash_prefix = H(sk)` where
/// `sk` is the signing key
///
/// # Returns
///
/// A `Result` whose `Ok` value is an Ed25519ph [`Signature`] on the
/// `prehashed_message` if the context was 255 bytes or less, otherwise
/// a `SignatureError`.
///
/// [rfc8032]: https://tools.ietf.org/html/rfc8032#section-5.1
#[cfg(feature = "digest")]
#[allow(non_snake_case)]
pub fn raw_sign_prehashed<CtxDigest, MsgDigest>(
    esk: &ExpandedSecretKey,
    prehashed_message: MsgDigest,
    verifying_key: &VerifyingKey,
    context: Option<&[u8]>,
) -> Result<Signature, SignatureError>
where
    MsgDigest: Digest<OutputSize = U64>,
    CtxDigest: Digest<OutputSize = U64>,
{
    esk.raw_sign_prehashed::<CtxDigest, MsgDigest>(prehashed_message, verifying_key, context)
}

/// Compute an ordinary Ed25519 signature, with the message contents provided incrementally by
/// updating a digest instance.
///
/// The `msg_update` closure provides the message content, updating a hasher argument. It will be
/// called twice. This closure MUST leave its hasher in the same state (i.e., must hash the same
/// values) after both calls. Otherwise it will produce an invalid signature.
///
/// `CtxDigest` is the digest used to calculate the pseudorandomness needed for signing. According
/// to the Ed25519 spec, `CtxDigest = Sha512`.
///
/// # ⚠️  Cryptographically Unsafe
///
/// Do NOT use this function unless you absolutely must. Using the wrong values in
/// `ExpandedSecretKey` can leak your signing key. See
/// [here](https://github.com/MystenLabs/ed25519-unsafe-libs) for more details on this attack.
pub fn raw_sign_byupdate<CtxDigest, F>(
    esk: &ExpandedSecretKey,
    msg_update: F,
    verifying_key: &VerifyingKey,
) -> Result<Signature, SignatureError>
where
    CtxDigest: Digest<OutputSize = U64>,
    F: Fn(&mut CtxDigest) -> Result<(), SignatureError>,
{
    esk.raw_sign_byupdate::<CtxDigest, F>(msg_update, verifying_key)
}

/// The ordinary non-batched Ed25519 verification check, rejecting non-canonical R
/// values.`CtxDigest` is the digest used to calculate the pseudorandomness needed for signing.
/// According to the Ed25519 spec, `CtxDigest = Sha512`.
pub fn raw_verify<CtxDigest>(
    vk: &VerifyingKey,
    message: &[u8],
    signature: &crate::ed25519::Signature,
) -> Result<(), SignatureError>
where
    CtxDigest: Digest<OutputSize = U64>,
{
    vk.raw_verify::<CtxDigest>(message, signature)
}

/// The batched Ed25519 verification check, rejecting non-canonical R values. `MsgDigest` is the
/// digest used to hash the signed message. `CtxDigest` is the digest used to calculate the
/// pseudorandomness needed for signing. According to the Ed25519 spec, `MsgDigest = CtxDigest =
/// Sha512`.
#[cfg(feature = "digest")]
#[allow(non_snake_case)]
pub fn raw_verify_prehashed<CtxDigest, MsgDigest>(
    vk: &VerifyingKey,
    prehashed_message: MsgDigest,
    context: Option<&[u8]>,
    signature: &crate::ed25519::Signature,
) -> Result<(), SignatureError>
where
    MsgDigest: Digest<OutputSize = U64>,
    CtxDigest: Digest<OutputSize = U64>,
{
    vk.raw_verify_prehashed::<CtxDigest, MsgDigest>(prehashed_message, context, signature)
}
