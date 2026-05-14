use crate::num_bigint::BigUint;
use crate::num_bigint::Integer;
use crate::num_bigint::{FromPrimitive, One, ToPrimitive};
use crate::rand_core::CryptoRngCore;
use crate::zeroize::{Zeroize, ZeroizeOnDrop};
use alloc::vec::Vec;
use core::hash::{Hash, Hasher};

use crate::rsa::algorithms::generate::generate_multi_prime_key_with_exp;
use crate::rsa::algorithms::rsa::{compute_modulus, mod_inverse_uint};

use crate::rsa::dummy_rng::DummyRng;
use crate::rsa::errors::{Error, Result};
use crate::rsa::traits::{PrivateKeyParts, PublicKeyParts, SignatureScheme};

/// Represents the public part of an RSA key.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RsaPublicKey {
    /// Modulus: product of prime numbers `p` and `q`
    n: BigUint,
    /// Public exponent: power to which a plaintext message is raised in
    /// order to encrypt it.
    ///
    /// Typically 0x10001 (65537)
    e: BigUint,
}

/// Represents a whole RSA key, public and private parts.
#[derive(Debug, Clone)]
pub struct RsaPrivateKey {
    /// Public components of the private key.
    pubkey_components: RsaPublicKey,
    /// Private exponent
    pub(crate) d: BigUint,
    /// Prime factors of N, contains >= 2 elements.
    pub(crate) primes: Vec<BigUint>,
    /// precomputed values to speed up private operations
    pub(crate) precomputed: Option<PrecomputedValues>,
}

impl Eq for RsaPrivateKey {}
impl PartialEq for RsaPrivateKey {
    #[inline]
    fn eq(&self, other: &RsaPrivateKey) -> bool {
        self.pubkey_components == other.pubkey_components
            && self.d == other.d
            && self.primes == other.primes
    }
}

impl AsRef<RsaPublicKey> for RsaPrivateKey {
    fn as_ref(&self) -> &RsaPublicKey {
        &self.pubkey_components
    }
}

impl Hash for RsaPrivateKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Domain separator for RSA private keys
        state.write(b"RsaPrivateKey");
        Hash::hash(&self.pubkey_components, state);
    }
}

impl Drop for RsaPrivateKey {
    fn drop(&mut self) {
        self.d.zeroize();
        self.primes.zeroize();
        self.precomputed.zeroize();
    }
}

impl ZeroizeOnDrop for RsaPrivateKey {}

#[derive(Debug, Clone)]
pub(crate) struct PrecomputedValues {
    /// D mod (P-1)
    pub(crate) dp: BigUint,
    /// D mod (Q-1)
    pub(crate) dq: BigUint,
    /// Q^-1 mod P
    pub(crate) qinv: BigUint,
}

impl Zeroize for PrecomputedValues {
    fn zeroize(&mut self) {
        self.dp.zeroize();
        self.dq.zeroize();
        self.qinv.zeroize();
    }
}

impl Drop for PrecomputedValues {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl From<RsaPrivateKey> for RsaPublicKey {
    fn from(private_key: RsaPrivateKey) -> Self {
        (&private_key).into()
    }
}

impl From<&RsaPrivateKey> for RsaPublicKey {
    fn from(private_key: &RsaPrivateKey) -> Self {
        let n = private_key.n().clone();
        let e = private_key.e().clone();
        RsaPublicKey { n, e }
    }
}

impl PublicKeyParts for RsaPublicKey {
    fn n(&self) -> &BigUint {
        &self.n
    }

    fn e(&self) -> &BigUint {
        &self.e
    }
}

impl RsaPublicKey {
    /// Verify a signed message.
    ///
    /// `hashed` must be the result of hashing the input using the hashing function
    /// passed in through `hash`.
    ///
    /// If the message is valid `Ok(())` is returned, otherwise an `Err` indicating failure.
    pub fn verify<S: SignatureScheme>(&self, scheme: S, hashed: &[u8], sig: &[u8]) -> Result<()> {
        scheme.verify(self, hashed, sig)
    }
}

impl RsaPublicKey {
    /// Minimum value of the public exponent `e`.
    pub const MIN_PUB_EXPONENT: u64 = 2;

    /// Maximum value of the public exponent `e`.
    pub const MAX_PUB_EXPONENT: u64 = (1 << 33) - 1;

    /// Maximum size of the modulus `n` in bits.
    pub const MAX_SIZE: usize = 4096;

    /// Create a new public key from its components.
    ///
    /// This function accepts public keys with a modulus size up to 4096-bits,
    /// i.e. [`RsaPublicKey::MAX_SIZE`].
    pub fn new(n: BigUint, e: BigUint) -> Result<Self> {
        Self::new_with_max_size(n, e, Self::MAX_SIZE)
    }

    /// Create a new public key from its components.
    pub fn new_with_max_size(n: BigUint, e: BigUint, max_size: usize) -> Result<Self> {
        let k = Self { n, e };
        check_public_with_max_size(&k, Some(max_size))?;
        Ok(k)
    }

    /// Create a new public key, bypassing checks around the modulus and public
    /// exponent size.
    ///
    /// This method is not recommended, and only intended for unusual use cases.
    /// Most applications should use [`RsaPublicKey::new`] or
    /// [`RsaPublicKey::new_with_max_size`] instead.
    pub fn new_unchecked(n: BigUint, e: BigUint) -> Self {
        Self { n, e }
    }
}

impl PublicKeyParts for RsaPrivateKey {
    fn n(&self) -> &BigUint {
        &self.pubkey_components.n
    }

    fn e(&self) -> &BigUint {
        &self.pubkey_components.e
    }
}

impl RsaPrivateKey {
    /// Default exponent for RSA keys.
    const EXP: u64 = 65537;

    /// Generate a new Rsa key pair of the given bit size using the passed in `rng`.
    pub fn new<R: CryptoRngCore + ?Sized>(rng: &mut R, bit_size: usize) -> Result<RsaPrivateKey> {
        let exp = BigUint::from_u64(Self::EXP).expect("invalid static exponent");
        Self::new_with_exp(rng, bit_size, &exp)
    }

    /// Generate a new RSA key pair of the given bit size and the public exponent
    /// using the passed in `rng`.
    ///
    /// Unless you have specific needs, you should use `RsaPrivateKey::new` instead.
    pub fn new_with_exp<R: CryptoRngCore + ?Sized>(
        rng: &mut R,
        bit_size: usize,
        exp: &BigUint,
    ) -> Result<RsaPrivateKey> {
        let components = generate_multi_prime_key_with_exp(rng, 2, bit_size, exp)?;
        RsaPrivateKey::from_components(components.n, components.e, components.d, components.primes)
    }

    /// Constructs an RSA key pair from two-prime RSA components.
    pub fn from_components(
        n: BigUint,
        e: BigUint,
        d: BigUint,
        primes: Vec<BigUint>,
    ) -> Result<RsaPrivateKey> {
        if primes.len() != 2 {
            return Err(Error::NprimesTooSmall);
        }

        let mut k = RsaPrivateKey {
            pubkey_components: RsaPublicKey { n, e },
            d,
            primes,
            precomputed: None,
        };

        // Always validate the key, to ensure precompute can't fail
        k.validate()?;

        // precompute when possible, ignore error otherwise.
        let _ = k.precompute();

        Ok(k)
    }

    /// Get the public key from the private key, cloning `n` and `e`.
    ///
    /// Generally this is not needed since `RsaPrivateKey` implements the `PublicKey` trait,
    /// but it can occasionally be useful to discard the private information entirely.
    pub fn to_public_key(&self) -> RsaPublicKey {
        self.pubkey_components.clone()
    }

    /// Performs some calculations to speed up private key operations.
    pub fn precompute(&mut self) -> Result<()> {
        if self.precomputed.is_some() {
            return Ok(());
        }

        let dp = &self.d % (&self.primes[0] - BigUint::one());
        let dq = &self.d % (&self.primes[1] - BigUint::one());
        let qinv = mod_inverse_uint(&self.primes[1], &self.primes[0]).ok_or(Error::InvalidPrime)?;

        self.precomputed = Some(PrecomputedValues { dp, dq, qinv });

        Ok(())
    }

    /// Clears precomputed values by setting to None
    pub fn clear_precomputed(&mut self) {
        self.precomputed = None;
    }

    /// Compute CRT coefficient: `(1/q) mod p`.
    pub fn crt_coefficient(&self) -> Option<BigUint> {
        mod_inverse_uint(&self.primes[1], &self.primes[0])
    }

    /// Performs basic sanity checks on the key.
    /// Returns `Ok(())` if everything is good, otherwise an appropriate error.
    pub fn validate(&self) -> Result<()> {
        check_public(self)?;

        // Check that Πprimes == n.
        let mut m = BigUint::one();
        for prime in &self.primes {
            // Any primes ≤ 1 will cause divide-by-zero panics later.
            if *prime <= BigUint::one() {
                return Err(Error::InvalidPrime);
            }
            m *= prime;
        }
        if m != self.pubkey_components.n {
            return Err(Error::InvalidModulus);
        }

        // Check that de ≡ 1 mod p-1, for each prime.
        // This implies that e is coprime to each p-1 as e has a multiplicative
        // inverse. Therefore e is coprime to lcm(p-1,q-1,r-1,...) =
        // exponent(ℤ/nℤ). It also implies that a^de ≡ a mod p as a^(p-1) ≡ 1
        // mod p. Thus a^de ≡ a mod n for all a coprime to n, as required.
        let mut de = self.e().clone();
        de *= self.d.clone();
        for prime in &self.primes {
            let congruence: BigUint = &de % (prime - BigUint::one());
            if !congruence.is_one() {
                return Err(Error::InvalidExponent);
            }
        }

        Ok(())
    }

    /// Sign the given digest.
    pub fn sign<S: SignatureScheme>(&self, padding: S, digest_in: &[u8]) -> Result<Vec<u8>> {
        padding.sign(Option::<&mut DummyRng>::None, self, digest_in)
    }

    /// Sign the given digest using the provided `rng`, which is used in the
    /// following ways depending on the [`SignatureScheme`]:
    ///
    /// - [`Pkcs1v15Sign`][`crate::rsa::Pkcs1v15Sign`] padding: uses the RNG
    ///   to mask the private key operation with random blinding, which helps
    ///   mitigate sidechannel attacks.
    /// - [`Pss`][`crate::rsa::Pss`] always requires randomness. Use
    ///   [`Pss::new`][`crate::rsa::Pss::new`] for a standard RSASSA-PSS signature, or
    ///   [`Pss::new_blinded`][`crate::rsa::Pss::new_blinded`] for RSA-BSSA blind
    ///   signatures.
    pub fn sign_with_rng<R: CryptoRngCore, S: SignatureScheme>(
        &self,
        rng: &mut R,
        padding: S,
        digest_in: &[u8],
    ) -> Result<Vec<u8>> {
        padding.sign(Some(rng), self, digest_in)
    }
}

impl PrivateKeyParts for RsaPrivateKey {
    fn d(&self) -> &BigUint {
        &self.d
    }

    fn primes(&self) -> &[BigUint] {
        &self.primes
    }

    fn dp(&self) -> Option<&BigUint> {
        self.precomputed.as_ref().map(|p| &p.dp)
    }

    fn dq(&self) -> Option<&BigUint> {
        self.precomputed.as_ref().map(|p| &p.dq)
    }

    fn qinv(&self) -> Option<&BigUint> {
        self.precomputed.as_ref().map(|p| &p.qinv)
    }
}

/// Check that the public key is well formed and has an exponent within acceptable bounds.
#[inline]
pub fn check_public(public_key: &impl PublicKeyParts) -> Result<()> {
    check_public_with_max_size(public_key, None)
}

/// Check that the public key is well formed and has an exponent within acceptable bounds.
#[inline]
fn check_public_with_max_size(
    public_key: &impl PublicKeyParts,
    max_size: Option<usize>,
) -> Result<()> {
    if let Some(max_size) = max_size {
        if public_key.n().bits() > max_size {
            return Err(Error::ModulusTooLarge);
        }
    }

    let e = public_key
        .e()
        .to_u64()
        .ok_or(Error::PublicExponentTooLarge)?;

    if public_key.e() >= public_key.n() || public_key.n().is_even() {
        return Err(Error::InvalidModulus);
    }

    if public_key.e().is_even() {
        return Err(Error::InvalidExponent);
    }

    if e < RsaPublicKey::MIN_PUB_EXPONENT {
        return Err(Error::PublicExponentTooSmall);
    }

    if e > RsaPublicKey::MAX_PUB_EXPONENT {
        return Err(Error::PublicExponentTooLarge);
    }

    Ok(())
}
