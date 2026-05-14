//! Encryption and Decryption using [OAEP padding](https://datatracker.ietf.org/doc/html/rfc8017#section-7.1).
//!
//! # Usage
//!
//! See [code example in the toplevel rustdoc](../index.html#oaep-encryption).

mod decrypting_key;
mod encrypting_key;

pub use self::{decrypting_key::DecryptingKey, encrypting_key::EncryptingKey};

use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

use crate::digest::{Digest, DynDigest, FixedOutputReset};
use crate::num_bigint::BigUint;
use crate::rand_core::CryptoRngCore;
use crate::zeroize::Zeroizing;

use crate::rsa::algorithms::oaep::*;
use crate::rsa::algorithms::pad::{uint_to_be_pad, uint_to_zeroizing_be_pad};
use crate::rsa::algorithms::rsa::{rsa_decrypt_and_check, rsa_encrypt};
use crate::rsa::errors::{Error, Result};
use crate::rsa::key::{self, RsaPrivateKey, RsaPublicKey};
use crate::rsa::traits::{PaddingScheme, PublicKeyParts};

/// Encryption and Decryption using [OAEP padding](https://datatracker.ietf.org/doc/html/rfc8017#section-7.1).
///
/// - `digest` is used to hash the label. The maximum possible plaintext length is `m = k - 2 * h_len - 2`,
///   where `k` is the size of the RSA modulus.
/// - `mgf_digest` specifies the hash function that is used in the [MGF1](https://datatracker.ietf.org/doc/html/rfc8017#appendix-B.2).
/// - `label` is optional data that can be associated with the message.
///
/// The two hash functions can, but don't need to be the same.
///
/// A prominent example is the [`AndroidKeyStore`](https://developer.android.com/guide/topics/security/cryptography#oaep-mgf1-digest).
/// It uses SHA-1 for `mgf_digest` and a user-chosen SHA flavour for `digest`.
pub struct Oaep {
    /// Digest type to use.
    pub digest: Box<dyn DynDigest + Send + Sync>,

    /// Digest to use for Mask Generation Function (MGF).
    pub mgf_digest: Box<dyn DynDigest + Send + Sync>,

    /// Optional label.
    pub label: Option<String>,
}

impl Oaep {
    /// Create a new OAEP `PaddingScheme`, using `T` as the hash function for both the default (empty) label and for MGF1.
    ///
    /// # Example
    /// ```ignore
    /// use sha1::Sha1;
    /// use crate::sha2::Sha256;
    /// use rsa::{BigUint, RsaPublicKey, Oaep, };
    /// use base64ct::{Base64, Encoding};
    ///
    /// let n = Base64::decode_vec("ALHgDoZmBQIx+jTmgeeHW6KsPOrj11f6CvWsiRleJlQpW77AwSZhd21ZDmlTKfaIHBSUxRUsuYNh7E2SHx8rkFVCQA2/gXkZ5GK2IUbzSTio9qXA25MWHvVxjMfKSL8ZAxZyKbrG94FLLszFAFOaiLLY8ECs7g+dXOriYtBwLUJK+lppbd+El+8ZA/zH0bk7vbqph5pIoiWggxwdq3mEz4LnrUln7r6dagSQzYErKewY8GADVpXcq5mfHC1xF2DFBub7bFjMVM5fHq7RK+pG5xjNDiYITbhLYrbVv3X0z75OvN0dY49ITWjM7xyvMWJXVJS7sJlgmCCL6RwWgP8PhcE=").unwrap();
    /// let e = Base64::decode_vec("AQAB").unwrap();
    ///
    /// let mut rng = crate::rand_core::OsRng;
    /// let key = RsaPublicKey::new(BigUint::from_bytes_be(&n), BigUint::from_bytes_be(&e)).unwrap();
    /// let padding = Oaep::new::<Sha256>();
    /// let encrypted_data = key.encrypt(&mut rng, padding, b"secret").unwrap();
    /// ```ignore
    pub fn new<T: 'static + Digest + DynDigest + Send + Sync>() -> Self {
        Self {
            digest: Box::new(T::new()),
            mgf_digest: Box::new(T::new()),
            label: None,
        }
    }

    /// Create a new OAEP `PaddingScheme` with an associated `label`, using `T` as the hash function for both the label and for MGF1.
    pub fn new_with_label<T: 'static + Digest + DynDigest + Send + Sync, S: AsRef<str>>(
        label: S,
    ) -> Self {
        Self {
            digest: Box::new(T::new()),
            mgf_digest: Box::new(T::new()),
            label: Some(label.as_ref().to_string()),
        }
    }

    /// Create a new OAEP `PaddingScheme`, using `T` as the hash function for the default (empty) label, and `U` as the hash function for MGF1.
    /// If a label is needed use `PaddingScheme::new_oaep_with_label` or `PaddingScheme::new_oaep_with_mgf_hash_with_label`.
    ///
    /// # Example
    /// ```ignore
    /// use sha1::Sha1;
    /// use crate::sha2::Sha256;
    /// use rsa::{BigUint, RsaPublicKey, Oaep, };
    /// use base64ct::{Base64, Encoding};
    ///
    /// let n = Base64::decode_vec("ALHgDoZmBQIx+jTmgeeHW6KsPOrj11f6CvWsiRleJlQpW77AwSZhd21ZDmlTKfaIHBSUxRUsuYNh7E2SHx8rkFVCQA2/gXkZ5GK2IUbzSTio9qXA25MWHvVxjMfKSL8ZAxZyKbrG94FLLszFAFOaiLLY8ECs7g+dXOriYtBwLUJK+lppbd+El+8ZA/zH0bk7vbqph5pIoiWggxwdq3mEz4LnrUln7r6dagSQzYErKewY8GADVpXcq5mfHC1xF2DFBub7bFjMVM5fHq7RK+pG5xjNDiYITbhLYrbVv3X0z75OvN0dY49ITWjM7xyvMWJXVJS7sJlgmCCL6RwWgP8PhcE=").unwrap();
    /// let e = Base64::decode_vec("AQAB").unwrap();
    ///
    /// let mut rng = crate::rand_core::OsRng;
    /// let key = RsaPublicKey::new(BigUint::from_bytes_be(&n), BigUint::from_bytes_be(&e)).unwrap();
    /// let padding = Oaep::new_with_mgf_hash::<Sha256, Sha1>();
    /// let encrypted_data = key.encrypt(&mut rng, padding, b"secret").unwrap();
    /// ```ignore
    pub fn new_with_mgf_hash<
        T: 'static + Digest + DynDigest + Send + Sync,
        U: 'static + Digest + DynDigest + Send + Sync,
    >() -> Self {
        Self {
            digest: Box::new(T::new()),
            mgf_digest: Box::new(U::new()),
            label: None,
        }
    }

    /// Create a new OAEP `PaddingScheme` with an associated `label`, using `T` as the hash function for the label, and `U` as the hash function for MGF1.
    pub fn new_with_mgf_hash_and_label<
        T: 'static + Digest + DynDigest + Send + Sync,
        U: 'static + Digest + DynDigest + Send + Sync,
        S: AsRef<str>,
    >(
        label: S,
    ) -> Self {
        Self {
            digest: Box::new(T::new()),
            mgf_digest: Box::new(U::new()),
            label: Some(label.as_ref().to_string()),
        }
    }
}

impl PaddingScheme for Oaep {
    fn decrypt<Rng: CryptoRngCore>(
        mut self,
        rng: Option<&mut Rng>,
        priv_key: &RsaPrivateKey,
        ciphertext: &[u8],
    ) -> Result<Vec<u8>> {
        decrypt(
            rng,
            priv_key,
            ciphertext,
            &mut *self.digest,
            &mut *self.mgf_digest,
            self.label,
        )
    }

    fn encrypt<Rng: CryptoRngCore>(
        mut self,
        rng: &mut Rng,
        pub_key: &RsaPublicKey,
        msg: &[u8],
    ) -> Result<Vec<u8>> {
        encrypt(
            rng,
            pub_key,
            msg,
            &mut *self.digest,
            &mut *self.mgf_digest,
            self.label,
        )
    }
}

impl fmt::Debug for Oaep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OAEP")
            .field("digest", &"...")
            .field("mgf_digest", &"...")
            .field("label", &self.label)
            .finish()
    }
}

/// Encrypts the given message with RSA and the padding scheme from
/// [PKCS#1 OAEP].
///
/// The message must be no longer than the length of the public modulus minus
/// `2 + (2 * hash.size())`.
///
/// [PKCS#1 OAEP]: https://datatracker.ietf.org/doc/html/rfc8017#section-7.1
#[inline]
fn encrypt<R: CryptoRngCore + ?Sized>(
    rng: &mut R,
    pub_key: &RsaPublicKey,
    msg: &[u8],
    digest: &mut dyn DynDigest,
    mgf_digest: &mut dyn DynDigest,
    label: Option<String>,
) -> Result<Vec<u8>> {
    key::check_public(pub_key)?;

    let em = oaep_encrypt(rng, msg, digest, mgf_digest, label, pub_key.size())?;

    let int = Zeroizing::new(BigUint::from_bytes_be(&em));
    uint_to_be_pad(rsa_encrypt(pub_key, &int)?, pub_key.size())
}

/// Encrypts the given message with RSA and the padding scheme from
/// [PKCS#1 OAEP].
///
/// The message must be no longer than the length of the public modulus minus
/// `2 + (2 * hash.size())`.
///
/// [PKCS#1 OAEP]: https://datatracker.ietf.org/doc/html/rfc8017#section-7.1
fn encrypt_digest<R: CryptoRngCore + ?Sized, D: Digest, MGD: Digest + FixedOutputReset>(
    rng: &mut R,
    pub_key: &RsaPublicKey,
    msg: &[u8],
    label: Option<String>,
) -> Result<Vec<u8>> {
    key::check_public(pub_key)?;

    let em = oaep_encrypt_digest::<_, D, MGD>(rng, msg, label, pub_key.size())?;

    let int = Zeroizing::new(BigUint::from_bytes_be(&em));
    uint_to_be_pad(rsa_encrypt(pub_key, &int)?, pub_key.size())
}

/// Decrypts a plaintext using RSA and the padding scheme from [PKCS#1 OAEP].
///
/// If an `rng` is passed, it uses RSA blinding to avoid timing side-channel attacks.
///
/// Note that whether this function returns an error or not discloses secret
/// information. If an attacker can cause this function to run repeatedly and
/// learn whether each instance returned an error then they can decrypt and
/// forge signatures as if they had the private key.
///
/// See `decrypt_session_key` for a way of solving this problem.
///
/// [PKCS#1 OAEP]: https://datatracker.ietf.org/doc/html/rfc8017#section-7.1
#[inline]
fn decrypt<R: CryptoRngCore + ?Sized>(
    rng: Option<&mut R>,
    priv_key: &RsaPrivateKey,
    ciphertext: &[u8],
    digest: &mut dyn DynDigest,
    mgf_digest: &mut dyn DynDigest,
    label: Option<String>,
) -> Result<Vec<u8>> {
    key::check_public(priv_key)?;

    if ciphertext.len() != priv_key.size() {
        return Err(Error::Decryption);
    }

    let em = rsa_decrypt_and_check(priv_key, rng, &BigUint::from_bytes_be(ciphertext))?;
    let mut em = uint_to_zeroizing_be_pad(em, priv_key.size())?;

    oaep_decrypt(&mut em, digest, mgf_digest, label, priv_key.size())
}

/// Decrypts a plaintext using RSA and the padding scheme from [PKCS#1 OAEP].
///
/// If an `rng` is passed, it uses RSA blinding to avoid timing side-channel attacks.
///
/// Note that whether this function returns an error or not discloses secret
/// information. If an attacker can cause this function to run repeatedly and
/// learn whether each instance returned an error then they can decrypt and
/// forge signatures as if they had the private key.
///
/// See `decrypt_session_key` for a way of solving this problem.
///
/// [PKCS#1 OAEP]: https://datatracker.ietf.org/doc/html/rfc8017#section-7.1
#[inline]
fn decrypt_digest<R: CryptoRngCore + ?Sized, D: Digest, MGD: Digest + FixedOutputReset>(
    rng: Option<&mut R>,
    priv_key: &RsaPrivateKey,
    ciphertext: &[u8],
    label: Option<String>,
) -> Result<Vec<u8>> {
    key::check_public(priv_key)?;

    if ciphertext.len() != priv_key.size() {
        return Err(Error::Decryption);
    }

    let em = rsa_decrypt_and_check(priv_key, rng, &BigUint::from_bytes_be(ciphertext))?;
    let mut em = uint_to_zeroizing_be_pad(em, priv_key.size())?;

    oaep_decrypt_digest::<D, MGD>(&mut em, label, priv_key.size())
}
