//! Generic RSA implementation

use crate::num_bigint::BigUint;
use crate::num_bigint::{One, Zero};
use crate::rand_core::CryptoRngCore;
use crate::zeroize::{Zeroize, Zeroizing};
use alloc::borrow::Cow;
use core::cmp::Ordering;

use crate::rsa::algorithms::random::random_biguint_below;
use crate::rsa::errors::{Error, Result};
use crate::rsa::traits::{PrivateKeyParts, PublicKeyParts};

/// ⚠️ Raw RSA encryption of m with the public key. No padding is performed.
///
/// # ☢️️ WARNING: HAZARDOUS API ☢️
///
/// Use this function with great care! Raw RSA should never be used without an appropriate padding
/// or signature scheme. See the [module-level documentation][crate::rsa::hazmat] for more information.
#[inline]
pub fn rsa_encrypt<K: PublicKeyParts>(key: &K, m: &BigUint) -> Result<BigUint> {
    Ok(m.modpow(key.e(), key.n()))
}

/// ⚠️ Performs raw RSA decryption with no padding or error checking.
///
/// Returns a plaintext `BigUint`. Performs RSA blinding if an `Rng` is passed.
///
/// # ☢️️ WARNING: HAZARDOUS API ☢️
///
/// Use this function with great care! Raw RSA should never be used without an appropriate padding
/// or signature scheme. See the [module-level documentation][crate::rsa::hazmat] for more information.
#[inline]
pub fn rsa_decrypt<R: CryptoRngCore + ?Sized>(
    mut rng: Option<&mut R>,
    priv_key: &impl PrivateKeyParts,
    c: &BigUint,
) -> Result<BigUint> {
    if c >= priv_key.n() {
        return Err(Error::Decryption);
    }

    if priv_key.n().is_zero() {
        return Err(Error::Decryption);
    }

    let mut ir = None;

    let c = if let Some(ref mut rng) = rng {
        let (blinded, unblinder) = blind(rng, priv_key, c);
        ir = Some(unblinder);
        Cow::Owned(blinded)
    } else {
        Cow::Borrowed(c)
    };

    let dp = priv_key.dp();
    let dq = priv_key.dq();
    let qinv = priv_key.qinv();

    let m = match (dp, dq, qinv) {
        (Some(dp), Some(dq), Some(qinv)) => {
            // We have the precalculated values needed for the CRT.

            let p = &priv_key.primes()[0];
            let q = &priv_key.primes()[1];

            let mut m1 = c.modpow(dp, p);
            let mut m2 = c.modpow(dq, q);
            let diff = if m1 >= m2 { &m1 - &m2 } else { &m1 + p - &m2 };
            let h = (qinv * diff) % p;
            let m = &m2 + q * h;

            // clear tmp values
            m1.zeroize();
            m2.zeroize();

            m
        }
        _ => c.modpow(priv_key.d(), priv_key.n()),
    };

    match ir {
        Some(ref ir) => {
            // unblind
            Ok(unblind(priv_key, &m, ir))
        }
        None => Ok(m),
    }
}

/// ⚠️ Performs raw RSA decryption with no padding.
///
/// Returns a plaintext `BigUint`. Performs RSA blinding if an `Rng` is passed.  This will also
/// check for errors in the CRT computation.
///
/// # ☢️️ WARNING: HAZARDOUS API ☢️
///
/// Use this function with great care! Raw RSA should never be used without an appropriate padding
/// or signature scheme. See the [module-level documentation][crate::rsa::hazmat] for more information.
#[inline]
pub fn rsa_decrypt_and_check<R: CryptoRngCore + ?Sized>(
    priv_key: &impl PrivateKeyParts,
    rng: Option<&mut R>,
    c: &BigUint,
) -> Result<BigUint> {
    let m = rsa_decrypt(rng, priv_key, c)?;

    // In order to defend against errors in the CRT computation, m^e is
    // calculated, which should match the original ciphertext.
    let check = rsa_encrypt(priv_key, &m)?;

    if c != &check {
        return Err(Error::Internal);
    }

    Ok(m)
}

/// Returns the blinded c, along with the unblinding factor.
fn blind<R: CryptoRngCore, K: PublicKeyParts>(
    rng: &mut R,
    key: &K,
    c: &BigUint,
) -> (BigUint, BigUint) {
    // Blinding involves multiplying c by r^e.
    // Then the decryption operation performs (m^e * r^e)^d mod n
    // which equals mr mod n. The factor of r can then be removed
    // by multiplying by the multiplicative inverse of r.

    let mut r: BigUint;
    let mut ir;
    let unblinder;
    loop {
        r = random_biguint_below(rng, key.n());
        if r.is_zero() {
            r = BigUint::one();
        }
        ir = mod_inverse_uint(&r, key.n());
        if let Some(ub) = ir {
            unblinder = ub;
            break;
        }
    }

    let c = {
        let mut rpowe = r.modpow(key.e(), key.n()); // N != 0
        let mut c = c * &rpowe;
        c %= key.n();

        rpowe.zeroize();

        c
    };

    (c, unblinder)
}

/// Given an m and and unblinding factor, unblind the m.
fn unblind(key: &impl PublicKeyParts, m: &BigUint, unblinder: &BigUint) -> BigUint {
    (m * unblinder) % key.n()
}

/// Compute the modulus of a key from its primes.
pub(crate) fn compute_modulus(primes: &[BigUint]) -> BigUint {
    primes.iter().product()
}

/// Compute the private exponent from its primes (p and q) and public exponent
/// This uses Euler's totient function
#[inline]
pub(crate) fn compute_private_exponent_euler_totient(
    primes: &[BigUint],
    exp: &BigUint,
) -> Result<BigUint> {
    if primes.len() < 2 {
        return Err(Error::InvalidPrime);
    }

    let mut totient = BigUint::one();

    for prime in primes {
        totient *= prime - BigUint::one();
    }

    // NOTE: `mod_inverse` checks if `exp` evenly divides `totient` and returns `None` if so.
    // This ensures that `exp` is not a factor of any `(prime - 1)`.
    if let Some(d) = mod_inverse_uint(exp, &totient) {
        Ok(d)
    } else {
        // `exp` evenly divides `totient`
        Err(Error::InvalidPrime)
    }
}

pub(crate) fn mod_inverse_uint(value: &BigUint, modulus: &BigUint) -> Option<BigUint> {
    if modulus.is_zero() {
        return None;
    }

    let mut old_r = value % modulus;
    let mut r = modulus.clone();
    let mut old_t = SignedBigUint::positive(BigUint::one());
    let mut t = SignedBigUint::zero();

    while !r.is_zero() {
        let q = &old_r / &r;
        let q_r = &q * &r;
        let next_r = &old_r - &q_r;
        let next_t = old_t.sub_mul(&q, &t);

        old_r = r;
        r = next_r;
        old_t = t;
        t = next_t;
    }

    if !old_r.is_one() {
        return None;
    }

    Some(old_t.mod_floor(modulus))
}

#[derive(Clone)]
struct SignedBigUint {
    negative: bool,
    magnitude: BigUint,
}

impl SignedBigUint {
    fn zero() -> Self {
        Self {
            negative: false,
            magnitude: BigUint::zero(),
        }
    }

    fn positive(magnitude: BigUint) -> Self {
        Self {
            negative: false,
            magnitude,
        }
    }

    fn sub_mul(&self, factor: &BigUint, rhs: &Self) -> Self {
        let product = factor * &rhs.magnitude;
        self.sub(&Self {
            negative: rhs.negative,
            magnitude: product,
        })
    }

    fn sub(&self, rhs: &Self) -> Self {
        if rhs.magnitude.is_zero() {
            return self.clone();
        }
        if self.magnitude.is_zero() {
            return Self {
                negative: !rhs.negative,
                magnitude: rhs.magnitude.clone(),
            };
        }

        if self.negative != rhs.negative {
            return Self {
                negative: self.negative,
                magnitude: &self.magnitude + &rhs.magnitude,
            };
        }

        match self.magnitude.cmp(&rhs.magnitude) {
            Ordering::Greater => Self {
                negative: self.negative,
                magnitude: &self.magnitude - &rhs.magnitude,
            },
            Ordering::Less => Self {
                negative: !self.negative,
                magnitude: &rhs.magnitude - &self.magnitude,
            },
            Ordering::Equal => Self::zero(),
        }
    }

    fn mod_floor(self, modulus: &BigUint) -> BigUint {
        let residue = self.magnitude % modulus;
        if !self.negative || residue.is_zero() {
            residue
        } else {
            modulus - residue
        }
    }
}
