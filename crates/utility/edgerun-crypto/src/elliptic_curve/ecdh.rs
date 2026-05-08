//! Elliptic Curve Diffie-Hellman Support.
//!
//! This module contains a generic ECDH implementation which is usable with
//! any elliptic curve which implements the [`CurveArithmetic`] trait (presently
//! the `k256` and `p256` crates)
//!
//! # ECDH Ephemeral (ECDHE) Usage
//!
//! Ephemeral Diffie-Hellman provides a one-time key exchange between two peers
//! using a randomly generated set of keys for each exchange.
//!
//! In practice ECDHE is used as part of an [Authenticated Key Exchange (AKE)][AKE]
//! protocol (e.g. [SIGMA]), where an existing cryptographic trust relationship
//! can be used to determine the authenticity of the ephemeral keys, such as
//! a digital signature. Without such an additional step, ECDHE is insecure!
//! (see security warning below)
//!
//! See the documentation for the [`EphemeralSecret`] type for more information
//! on performing ECDH ephemeral key exchanges.
//!
//! # Static ECDH Usage
//!
//! Static ECDH key exchanges are supported via the low-level
//! [`diffie_hellman`] function.
//!
//! [AKE]: https://en.wikipedia.org/wiki/Authenticated_Key_Exchange
//! [SIGMA]: https://webee.technion.ac.il/~hugo/sigma-pdf.pdf

use crate::digest::{crypto_common::BlockSizeUser, Digest, Output, OutputSizeUser};
use crate::elliptic_curve::rand_core::CryptoRngCore;
use crate::elliptic_curve::{
    point::AffineCoordinates, AffinePoint, Curve, CurveArithmetic, Error, FieldBytes,
    NonZeroScalar, ProjectivePoint, PublicKey,
};
use crate::group::Curve as _;
use crate::zeroize::{Zeroize, ZeroizeOnDrop};
use core::borrow::Borrow;
use generic_array::typenum::Unsigned;

#[derive(Clone, Debug)]
pub struct Hkdf<D>
where
    D: BlockSizeUser + Clone + Digest,
{
    prk: Output<D>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidLength;

impl<D> Hkdf<D>
where
    D: BlockSizeUser + Clone + Digest,
{
    pub fn new(salt: Option<&[u8]>, ikm: &[u8]) -> Self {
        let default_salt = Output::<D>::default();
        let salt = salt.unwrap_or(&default_salt);
        Self {
            prk: hmac::<D>(salt, ikm),
        }
    }

    pub fn expand(&self, info: &[u8], okm: &mut [u8]) -> Result<(), InvalidLength> {
        let chunk_len = <D as OutputSizeUser>::OutputSize::to_usize();
        if okm.len() > chunk_len * 255 {
            return Err(InvalidLength);
        }

        let mut prev: Option<Output<D>> = None;
        for (block_n, block) in okm.chunks_mut(chunk_len).enumerate() {
            let mut input = alloc::vec::Vec::new();
            if let Some(ref prev) = prev {
                input.extend_from_slice(prev);
            }
            input.extend_from_slice(info);
            input.push(block_n as u8 + 1);

            let output = hmac::<D>(&self.prk, &input);
            block.copy_from_slice(&output[..block.len()]);
            prev = Some(output);
        }

        Ok(())
    }
}

fn hmac<D>(key: &[u8], data: &[u8]) -> Output<D>
where
    D: BlockSizeUser + Clone + Digest,
{
    use crate::digest::generic_array::GenericArray;

    let mut key_block = GenericArray::<u8, <D as BlockSizeUser>::BlockSize>::default();
    if key.len() <= key_block.len() {
        key_block[..key.len()].copy_from_slice(key);
    } else {
        let hash = D::digest(key);
        let n = core::cmp::min(hash.len(), key_block.len());
        key_block[..n].copy_from_slice(&hash[..n]);
    }

    let mut ipad = key_block.clone();
    let mut opad = key_block;
    for byte in ipad.iter_mut() {
        *byte ^= 0x36;
    }
    for byte in opad.iter_mut() {
        *byte ^= 0x5c;
    }

    let mut inner = D::new();
    inner.update(&ipad);
    inner.update(data);
    let inner = inner.finalize();

    let mut outer = D::new();
    outer.update(&opad);
    outer.update(inner);
    outer.finalize()
}

/// Low-level Elliptic Curve Diffie-Hellman (ECDH) function.
///
/// Whenever possible, we recommend using the high-level ECDH ephemeral API
/// provided by [`EphemeralSecret`].
///
/// However, if you are implementing a protocol which requires a static scalar
/// value as part of an ECDH exchange, this API can be used to compute a
/// [`SharedSecret`] from that value.
///
/// Note that this API operates on the low-level [`NonZeroScalar`] and
/// [`AffinePoint`] types. If you are attempting to use the higher-level
/// [`SecretKey`][`crate::elliptic_curve::SecretKey`] and [`PublicKey`] types, you will
/// need to use the following conversions:
///
/// ```ignore
/// let shared_secret = elliptic_curve::ecdh::diffie_hellman(
///     secret_key.to_nonzero_scalar(),
///     public_key.as_affine()
/// );
/// ```
pub fn diffie_hellman<C>(
    secret_key: impl Borrow<NonZeroScalar<C>>,
    public_key: impl Borrow<AffinePoint<C>>,
) -> SharedSecret<C>
where
    C: CurveArithmetic,
{
    let public_point = ProjectivePoint::<C>::from(*public_key.borrow());
    let secret_point = (public_point * secret_key.borrow().as_ref()).to_affine();
    SharedSecret::new(secret_point)
}

/// Ephemeral Diffie-Hellman Secret.
///
/// These are ephemeral "secret key" values which are deliberately designed
/// to avoid being persisted.
///
/// To perform an ephemeral Diffie-Hellman exchange, do the following:
///
/// - Have each participant generate an [`EphemeralSecret`] value
/// - Compute the [`PublicKey`] for that value
/// - Have each peer provide their [`PublicKey`] to their counterpart
/// - Use [`EphemeralSecret`] and the other participant's [`PublicKey`]
///   to compute a [`SharedSecret`] value.
///
/// # ⚠️ SECURITY WARNING ⚠️
///
/// Ephemeral Diffie-Hellman exchanges are unauthenticated and without a
/// further authentication step are trivially vulnerable to man-in-the-middle
/// attacks!
///
/// These exchanges should be performed in the context of a protocol which
/// takes further steps to authenticate the peers in a key exchange.
pub struct EphemeralSecret<C>
where
    C: CurveArithmetic,
{
    scalar: NonZeroScalar<C>,
}

impl<C> EphemeralSecret<C>
where
    C: CurveArithmetic,
{
    /// Generate a cryptographically random [`EphemeralSecret`].
    pub fn random(rng: &mut impl CryptoRngCore) -> Self {
        Self {
            scalar: NonZeroScalar::random(rng),
        }
    }

    /// Construct an [`EphemeralSecret`] from a big-endian scalar for replay.
    ///
    /// This is intended for deterministic replay and test vectors. Production
    /// key exchange should use [`Self::random`] unless the scalar is supplied
    /// by a cryptographically sound deterministic transcript.
    pub fn from_replay_scalar_bytes(bytes: FieldBytes<C>) -> crate::elliptic_curve::Result<Self> {
        let scalar = Option::from(NonZeroScalar::from_repr(bytes)).ok_or(Error)?;
        Ok(Self { scalar })
    }

    /// Get the public key associated with this ephemeral secret.
    ///
    /// The `compress` flag enables point compression.
    pub fn public_key(&self) -> PublicKey<C> {
        PublicKey::from_secret_scalar(&self.scalar)
    }

    /// Compute a Diffie-Hellman shared secret from an ephemeral secret and the
    /// public key of the other participant in the exchange.
    pub fn diffie_hellman(&self, public_key: &PublicKey<C>) -> SharedSecret<C> {
        diffie_hellman(self.scalar, public_key.as_affine())
    }
}

impl<C> From<&EphemeralSecret<C>> for PublicKey<C>
where
    C: CurveArithmetic,
{
    fn from(ephemeral_secret: &EphemeralSecret<C>) -> Self {
        ephemeral_secret.public_key()
    }
}

impl<C> Zeroize for EphemeralSecret<C>
where
    C: CurveArithmetic,
{
    fn zeroize(&mut self) {
        self.scalar.zeroize()
    }
}

impl<C> ZeroizeOnDrop for EphemeralSecret<C> where C: CurveArithmetic {}

impl<C> Drop for EphemeralSecret<C>
where
    C: CurveArithmetic,
{
    fn drop(&mut self) {
        self.zeroize();
    }
}

/// Shared secret value computed via ECDH key agreement.
pub struct SharedSecret<C: Curve> {
    /// Computed secret value
    secret_bytes: FieldBytes<C>,
}

impl<C: Curve> SharedSecret<C> {
    /// Create a new [`SharedSecret`] from an [`AffinePoint`] for this curve.
    #[inline]
    fn new(point: AffinePoint<C>) -> Self
    where
        C: CurveArithmetic,
    {
        Self {
            secret_bytes: point.x(),
        }
    }

    /// Use [HKDF] (HMAC-based Extract-and-Expand Key Derivation Function) to
    /// extract entropy from this shared secret.
    ///
    /// This method can be used to transform the shared secret into uniformly
    /// random values which are suitable as key material.
    ///
    /// The `D` type parameter is a cryptographic digest function.
    /// `crate::sha2::Sha256` is a common choice for use with HKDF.
    ///
    /// The `salt` parameter can be used to supply additional randomness.
    /// Some examples include:
    ///
    /// - randomly generated (but authenticated) string
    /// - fixed application-specific value
    /// - previous shared secret used for rekeying (as in TLS 1.3 and Noise)
    ///
    /// After initializing HKDF, use [`Hkdf::expand`] to obtain output key
    /// material.
    ///
    /// [HKDF]: https://en.wikipedia.org/wiki/HKDF
    pub fn extract<D>(&self, salt: Option<&[u8]>) -> Hkdf<D>
    where
        D: BlockSizeUser + Clone + Digest,
    {
        Hkdf::new(salt, &self.secret_bytes)
    }

    /// This value contains the raw serialized x-coordinate of the elliptic curve
    /// point computed from a Diffie-Hellman exchange, serialized as bytes.
    ///
    /// When in doubt, use [`SharedSecret::extract`] instead.
    ///
    /// # ⚠️ WARNING: NOT UNIFORMLY RANDOM! ⚠️
    ///
    /// This value is not uniformly random and should not be used directly
    /// as a cryptographic key for anything which requires that property
    /// (e.g. symmetric ciphers).
    ///
    /// Instead, the resulting value should be used as input to a Key Derivation
    /// Function (KDF) or cryptographic hash function to produce a symmetric key.
    /// The [`SharedSecret::extract`] function will do this for you.
    pub fn raw_secret_bytes(&self) -> &FieldBytes<C> {
        &self.secret_bytes
    }
}

impl<C: Curve> From<FieldBytes<C>> for SharedSecret<C> {
    /// NOTE: this impl is intended to be used by curve implementations to
    /// instantiate a [`SharedSecret`] value from their respective
    /// [`AffinePoint`] type.
    ///
    /// Curve implementations should provide the field element representing
    /// the affine x-coordinate as `secret_bytes`.
    fn from(secret_bytes: FieldBytes<C>) -> Self {
        Self { secret_bytes }
    }
}

impl<C: Curve> ZeroizeOnDrop for SharedSecret<C> {}

impl<C: Curve> Drop for SharedSecret<C> {
    fn drop(&mut self) {
        self.secret_bytes.zeroize()
    }
}
