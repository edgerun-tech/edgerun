//! Support for the [Probabilistic Signature Scheme] (PSS) a.k.a. RSASSA-PSS.
//!
//! Designed by Mihir Bellare and Phillip Rogaway. Specified in [RFC8017 § 8.1].
//!
//! # Usage
//!
//! See [code example in the toplevel rustdoc](../index.html#pss-signatures).
//!
//! [Probabilistic Signature Scheme]: https://en.wikipedia.org/wiki/Probabilistic_signature_scheme
//! [RFC8017 § 8.1]: https://datatracker.ietf.org/doc/html/rfc8017#section-8.1

mod blinded_signing_key;
mod signature;
mod signing_key;
mod verifying_key;

pub use self::{
    blinded_signing_key::BlindedSigningKey, signature::Signature, signing_key::SigningKey,
    verifying_key::VerifyingKey,
};

use alloc::{boxed::Box, vec::Vec};
use core::fmt::{self, Debug};

use crate::const_oid::{AssociatedOid, ObjectIdentifier};
use crate::digest::{Digest, DynDigest, FixedOutputReset};
use crate::num_bigint::BigUint;
use crate::pkcs1::RsaPssParams;
use crate::pkcs8::spki::{AlgorithmIdentifierOwned, der::Any};
use crate::rand_core::CryptoRngCore;

use crate::rsa::algorithms::pad::{uint_to_be_pad, uint_to_zeroizing_be_pad};
use crate::rsa::algorithms::pss::*;
use crate::rsa::algorithms::rsa::{rsa_decrypt_and_check, rsa_encrypt};
use crate::rsa::errors::{Error, Result};
use crate::rsa::traits::PublicKeyParts;
use crate::rsa::traits::SignatureScheme;
use crate::rsa::{RsaPrivateKey, RsaPublicKey};

/// Digital signatures using PSS padding.
pub struct Pss {
    /// Create blinded signatures.
    pub blinded: bool,

    /// Digest type to use.
    pub digest: Box<dyn DynDigest + Send + Sync>,

    /// Salt length.
    pub salt_len: usize,
}

impl Pss {
    /// New PSS padding for the given digest.
    /// Digest output size is used as a salt length.
    pub fn new<T: 'static + Digest + DynDigest + Send + Sync>() -> Self {
        Self::new_with_salt::<T>(<T as Digest>::output_size())
    }

    /// New PSS padding for the given digest with a salt value of the given length.
    pub fn new_with_salt<T: 'static + Digest + DynDigest + Send + Sync>(len: usize) -> Self {
        Self {
            blinded: false,
            digest: Box::new(T::new()),
            salt_len: len,
        }
    }

    /// New PSS padding for blinded signatures (RSA-BSSA) for the given digest.
    /// Digest output size is used as a salt length.
    pub fn new_blinded<T: 'static + Digest + DynDigest + Send + Sync>() -> Self {
        Self::new_blinded_with_salt::<T>(<T as Digest>::output_size())
    }

    /// New PSS padding for blinded signatures (RSA-BSSA) for the given digest
    /// with a salt value of the given length.
    pub fn new_blinded_with_salt<T: 'static + Digest + DynDigest + Send + Sync>(
        len: usize,
    ) -> Self {
        Self {
            blinded: true,
            digest: Box::new(T::new()),
            salt_len: len,
        }
    }
}

impl SignatureScheme for Pss {
    fn sign<Rng: CryptoRngCore>(
        mut self,
        rng: Option<&mut Rng>,
        priv_key: &RsaPrivateKey,
        hashed: &[u8],
    ) -> Result<Vec<u8>> {
        sign(
            rng.ok_or(Error::InvalidPaddingScheme)?,
            self.blinded,
            priv_key,
            hashed,
            self.salt_len,
            &mut *self.digest,
        )
    }

    fn verify(mut self, pub_key: &RsaPublicKey, hashed: &[u8], sig: &[u8]) -> Result<()> {
        verify(
            pub_key,
            hashed,
            &BigUint::from_bytes_be(sig),
            sig.len(),
            &mut *self.digest,
            self.salt_len,
        )
    }
}

impl Debug for Pss {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PSS")
            .field("blinded", &self.blinded)
            .field("digest", &"...")
            .field("salt_len", &self.salt_len)
            .finish()
    }
}

pub(crate) fn verify(
    pub_key: &RsaPublicKey,
    hashed: &[u8],
    sig: &BigUint,
    sig_len: usize,
    digest: &mut dyn DynDigest,
    salt_len: usize,
) -> Result<()> {
    if sig_len != pub_key.size() {
        return Err(Error::Verification);
    }

    let mut em = uint_to_be_pad(rsa_encrypt(pub_key, sig)?, pub_key.size())?;

    emsa_pss_verify(hashed, &mut em, salt_len, digest, pub_key.n().bits())
}

pub(crate) fn verify_digest<D>(
    pub_key: &RsaPublicKey,
    hashed: &[u8],
    sig: &BigUint,
    sig_len: usize,
    salt_len: usize,
) -> Result<()>
where
    D: Digest + FixedOutputReset,
{
    if sig >= pub_key.n() || sig_len != pub_key.size() {
        return Err(Error::Verification);
    }

    let mut em = uint_to_be_pad(rsa_encrypt(pub_key, sig)?, pub_key.size())?;

    emsa_pss_verify_digest::<D>(hashed, &mut em, salt_len, pub_key.n().bits())
}

/// SignPSS calculates the signature of hashed using RSASSA-PSS.
///
/// Note that hashed must be the result of hashing the input message using the
/// given hash function. The opts argument may be nil, in which case sensible
/// defaults are used.
pub(crate) fn sign<T: CryptoRngCore>(
    rng: &mut T,
    blind: bool,
    priv_key: &RsaPrivateKey,
    hashed: &[u8],
    salt_len: usize,
    digest: &mut dyn DynDigest,
) -> Result<Vec<u8>> {
    let mut salt = alloc::vec![0; salt_len];
    rng.fill_bytes(&mut salt[..]);

    sign_pss_with_salt(blind.then_some(rng), priv_key, hashed, &salt, digest)
}

pub(crate) fn sign_digest<T: CryptoRngCore + ?Sized, D: Digest + FixedOutputReset>(
    rng: &mut T,
    blind: bool,
    priv_key: &RsaPrivateKey,
    hashed: &[u8],
    salt_len: usize,
) -> Result<Vec<u8>> {
    let mut salt = alloc::vec![0; salt_len];
    rng.fill_bytes(&mut salt[..]);

    sign_pss_with_salt_digest::<_, D>(blind.then_some(rng), priv_key, hashed, &salt)
}

/// signPSSWithSalt calculates the signature of hashed using PSS with specified salt.
///
/// Note that hashed must be the result of hashing the input message using the
/// given hash function. salt is a random sequence of bytes whose length will be
/// later used to verify the signature.
fn sign_pss_with_salt<T: CryptoRngCore>(
    blind_rng: Option<&mut T>,
    priv_key: &RsaPrivateKey,
    hashed: &[u8],
    salt: &[u8],
    digest: &mut dyn DynDigest,
) -> Result<Vec<u8>> {
    let em_bits = priv_key.n().bits() - 1;
    let em = emsa_pss_encode(hashed, em_bits, salt, digest)?;

    uint_to_zeroizing_be_pad(
        rsa_decrypt_and_check(priv_key, blind_rng, &BigUint::from_bytes_be(&em))?,
        priv_key.size(),
    )
}

fn sign_pss_with_salt_digest<T: CryptoRngCore + ?Sized, D: Digest + FixedOutputReset>(
    blind_rng: Option<&mut T>,
    priv_key: &RsaPrivateKey,
    hashed: &[u8],
    salt: &[u8],
) -> Result<Vec<u8>> {
    let em_bits = priv_key.n().bits() - 1;
    let em = emsa_pss_encode_digest::<D>(hashed, em_bits, salt)?;

    uint_to_zeroizing_be_pad(
        rsa_decrypt_and_check(priv_key, blind_rng, &BigUint::from_bytes_be(&em))?,
        priv_key.size(),
    )
}

/// Returns the [`AlgorithmIdentifierOwned`] associated with PSS signature using a given digest.
pub fn get_default_pss_signature_algo_id<D>() -> crate::pkcs8::spki::Result<AlgorithmIdentifierOwned>
where
    D: Digest + AssociatedOid,
{
    let salt_len: u8 = <D as Digest>::output_size() as u8;
    get_pss_signature_algo_id::<D>(salt_len)
}

fn get_pss_signature_algo_id<D>(
    salt_len: u8,
) -> crate::pkcs8::spki::Result<AlgorithmIdentifierOwned>
where
    D: Digest + AssociatedOid,
{
    const ID_RSASSA_PSS: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.10");

    let pss_params = RsaPssParams::new::<D>(salt_len);

    Ok(AlgorithmIdentifierOwned {
        oid: ID_RSASSA_PSS,
        parameters: Some(Any::encode_from(&pss_params)?),
    })
}
