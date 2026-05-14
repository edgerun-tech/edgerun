//! PKCS#1 v1.5 support as described in [RFC8017 § 8.2].
//!
//! # Usage
//!
//! See [code example in the toplevel rustdoc](../index.html#pkcs1-v15-signatures).
//!
//! [RFC8017 § 8.2]: https://datatracker.ietf.org/doc/html/rfc8017#section-8.2

mod decrypting_key;
mod encrypting_key;
mod signature;
mod signing_key;
mod verifying_key;

pub use self::{
    decrypting_key::DecryptingKey, encrypting_key::EncryptingKey, signature::Signature,
    signing_key::SigningKey, verifying_key::VerifyingKey,
};

use crate::digest::Digest;
use crate::num_bigint::BigUint;
use crate::pkcs8::AssociatedOid;
use crate::rand_core::CryptoRngCore;
use crate::zeroize::Zeroizing;
use alloc::{boxed::Box, vec::Vec};
use core::fmt::Debug;

use crate::rsa::algorithms::pad::{uint_to_be_pad, uint_to_zeroizing_be_pad};
use crate::rsa::algorithms::pkcs1v15::*;
use crate::rsa::algorithms::rsa::{rsa_decrypt_and_check, rsa_encrypt};
use crate::rsa::errors::{Error, Result};
use crate::rsa::key::{self, RsaPrivateKey, RsaPublicKey};
use crate::rsa::traits::{PaddingScheme, PublicKeyParts, SignatureScheme};

/// Encryption using PKCS#1 v1.5 padding.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Pkcs1v15Encrypt;

impl PaddingScheme for Pkcs1v15Encrypt {
    fn decrypt<Rng: CryptoRngCore>(
        self,
        rng: Option<&mut Rng>,
        priv_key: &RsaPrivateKey,
        ciphertext: &[u8],
    ) -> Result<Vec<u8>> {
        decrypt(rng, priv_key, ciphertext)
    }

    fn encrypt<Rng: CryptoRngCore>(
        self,
        rng: &mut Rng,
        pub_key: &RsaPublicKey,
        msg: &[u8],
    ) -> Result<Vec<u8>> {
        encrypt(rng, pub_key, msg)
    }
}

/// `RSASSA-PKCS1-v1_5`: digital signatures using PKCS#1 v1.5 padding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pkcs1v15Sign {
    /// Length of hash to use.
    pub hash_len: Option<usize>,

    /// Prefix.
    pub prefix: Box<[u8]>,
}

impl Pkcs1v15Sign {
    /// Create new PKCS#1 v1.5 padding for the given digest.
    ///
    /// The digest must have an [`AssociatedOid`]. Make sure to enable the `oid`
    /// feature of the relevant digest crate.
    pub fn new<D>() -> Self
    where
        D: Digest + AssociatedOid,
    {
        Self {
            hash_len: Some(<D as Digest>::output_size()),
            prefix: pkcs1v15_generate_prefix::<D>().into_boxed_slice(),
        }
    }

    /// Create new PKCS#1 v1.5 padding for computing an unprefixed signature.
    ///
    /// This sets `hash_len` to `None` and uses an empty `prefix`.
    pub fn new_unprefixed() -> Self {
        Self {
            hash_len: None,
            prefix: Box::new([]),
        }
    }

    /// Create new PKCS#1 v1.5 padding for computing an unprefixed signature.
    ///
    /// This sets `hash_len` to `None` and uses an empty `prefix`.
    #[deprecated(since = "0.9.0", note = "use Pkcs1v15Sign::new_unprefixed instead")]
    pub fn new_raw() -> Self {
        Self::new_unprefixed()
    }
}

impl SignatureScheme for Pkcs1v15Sign {
    fn sign<Rng: CryptoRngCore>(
        self,
        rng: Option<&mut Rng>,
        priv_key: &RsaPrivateKey,
        hashed: &[u8],
    ) -> Result<Vec<u8>> {
        if let Some(hash_len) = self.hash_len {
            if hashed.len() != hash_len {
                return Err(Error::InputNotHashed);
            }
        }

        sign(rng, priv_key, &self.prefix, hashed)
    }

    fn verify(self, pub_key: &RsaPublicKey, hashed: &[u8], sig: &[u8]) -> Result<()> {
        if let Some(hash_len) = self.hash_len {
            if hashed.len() != hash_len {
                return Err(Error::InputNotHashed);
            }
        }

        verify(
            pub_key,
            self.prefix.as_ref(),
            hashed,
            &BigUint::from_bytes_be(sig),
            sig.len(),
        )
    }
}

/// Encrypts the given message with RSA and the padding
/// scheme from PKCS#1 v1.5.  The message must be no longer than the
/// length of the public modulus minus 11 bytes.
#[inline]
fn encrypt<R: CryptoRngCore + ?Sized>(
    rng: &mut R,
    pub_key: &RsaPublicKey,
    msg: &[u8],
) -> Result<Vec<u8>> {
    key::check_public(pub_key)?;

    let em = pkcs1v15_encrypt_pad(rng, msg, pub_key.size())?;
    let int = Zeroizing::new(BigUint::from_bytes_be(&em));
    uint_to_be_pad(rsa_encrypt(pub_key, &int)?, pub_key.size())
}

/// Decrypts a plaintext using RSA and the padding scheme from PKCS#1 v1.5.
///
/// If an `rng` is passed, it uses RSA blinding to avoid timing side-channel attacks.
///
/// Note that whether this function returns an error or not discloses secret
/// information. If an attacker can cause this function to run repeatedly and
/// learn whether each instance returned an error then they can decrypt and
/// forge signatures as if they had the private key. See
/// `decrypt_session_key` for a way of solving this problem.
#[inline]
fn decrypt<R: CryptoRngCore + ?Sized>(
    rng: Option<&mut R>,
    priv_key: &RsaPrivateKey,
    ciphertext: &[u8],
) -> Result<Vec<u8>> {
    key::check_public(priv_key)?;

    let em = rsa_decrypt_and_check(priv_key, rng, &BigUint::from_bytes_be(ciphertext))?;
    let em = uint_to_zeroizing_be_pad(em, priv_key.size())?;

    pkcs1v15_encrypt_unpad(em, priv_key.size())
}

/// Calculates the signature of hashed using
/// RSASSA-PKCS1-V1_5-SIGN from RSA PKCS#1 v1.5. Note that `hashed` must
/// be the result of hashing the input message using the given hash
/// function. If hash is `None`, hashed is signed directly. This isn't
/// advisable except for interoperability.
///
/// If `rng` is not `None` then RSA blinding will be used to avoid timing
/// side-channel attacks.
///
/// This function is deterministic. Thus, if the set of possible
/// messages is small, an attacker may be able to build a map from
/// messages to signatures and identify the signed messages. As ever,
/// signatures provide authenticity, not confidentiality.
#[inline]
fn sign<R: CryptoRngCore + ?Sized>(
    rng: Option<&mut R>,
    priv_key: &RsaPrivateKey,
    prefix: &[u8],
    hashed: &[u8],
) -> Result<Vec<u8>> {
    let em = pkcs1v15_sign_pad(prefix, hashed, priv_key.size())?;

    uint_to_zeroizing_be_pad(
        rsa_decrypt_and_check(priv_key, rng, &BigUint::from_bytes_be(&em))?,
        priv_key.size(),
    )
}

/// Verifies an RSA PKCS#1 v1.5 signature.
#[inline]
fn verify(
    pub_key: &RsaPublicKey,
    prefix: &[u8],
    hashed: &[u8],
    sig: &BigUint,
    sig_len: usize,
) -> Result<()> {
    if sig >= pub_key.n() || sig_len != pub_key.size() {
        return Err(Error::Verification);
    }

    let em = uint_to_be_pad(rsa_encrypt(pub_key, sig)?, pub_key.size())?;

    pkcs1v15_sign_unpad(prefix, hashed, &em, pub_key.size())
}

mod oid {
    use crate::const_oid::ObjectIdentifier;

    /// A trait which associates an RSA-specific OID with a type.
    pub trait RsaSignatureAssociatedOid {
        /// The OID associated with this type.
        const OID: ObjectIdentifier;
    }

    #[cfg(feature = "rsa_sha1")]
    impl RsaSignatureAssociatedOid for sha1::Sha1 {
        const OID: ObjectIdentifier =
            crate::const_oid::ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.5");
    }

    #[cfg(feature = "rsa_sha2")]
    impl RsaSignatureAssociatedOid for crate::sha2::Sha224 {
        const OID: ObjectIdentifier =
            crate::const_oid::ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.14");
    }

    #[cfg(feature = "rsa_sha2")]
    impl RsaSignatureAssociatedOid for crate::sha2::Sha256 {
        const OID: ObjectIdentifier =
            crate::const_oid::ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.11");
    }

    #[cfg(feature = "rsa_sha2")]
    impl RsaSignatureAssociatedOid for crate::sha2::Sha384 {
        const OID: ObjectIdentifier =
            crate::const_oid::ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.12");
    }

    #[cfg(feature = "rsa_sha2")]
    impl RsaSignatureAssociatedOid for crate::sha2::Sha512 {
        const OID: ObjectIdentifier =
            crate::const_oid::ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.13");
    }
}

pub use oid::RsaSignatureAssociatedOid;
