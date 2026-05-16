//! Generate prime components for the RSA Private Key

use crate::num_bigint::BigUint;
use crate::num_bigint::Zero;
use crate::rand_core::CryptoRngCore;
use alloc::vec::Vec;

use crate::rsa::{
    algorithms::{
        random::random_prime,
        rsa::{compute_modulus, compute_private_exponent_euler_totient},
    },
    errors::{Error, Result},
};

pub struct RsaPrivateKeyComponents {
    pub n: BigUint,
    pub e: BigUint,
    pub d: BigUint,
    pub primes: Vec<BigUint>,
}

/// Generates a multi-prime RSA keypair of the given bit size, public exponent,
/// and the given random source, as suggested in [1]. Although the public
/// keys are compatible (actually, indistinguishable) from the 2-prime case,
/// the private keys are not. Thus it may not be possible to export multi-prime
/// private keys in certain formats or to subsequently import them into other
/// code.
///
/// Table 1 in [2] suggests maximum numbers of primes for a given size.
///
/// [1]: https://patents.google.com/patent/US4405829A/en
/// [2]: http://www.cacr.math.uwaterloo.ca/techreports/2006/cacr2006-16.pdf
pub(crate) fn generate_multi_prime_key_with_exp<R: CryptoRngCore + ?Sized>(
    rng: &mut R,
    nprimes: usize,
    bit_size: usize,
    exp: &BigUint,
) -> Result<RsaPrivateKeyComponents> {
    if nprimes < 2 {
        return Err(Error::NprimesTooSmall);
    }

    if bit_size < 64 {
        let bits_per_prime = bit_size / nprimes;
        if available_top_bit_primes(bits_per_prime) < nprimes {
            return Err(Error::TooFewPrimes);
        }
    }

    let mut primes = alloc::vec![BigUint::zero(); nprimes];
    let n_final: BigUint;
    let d_final: BigUint;

    'next: loop {
        let mut todo = bit_size;
        // `gen_prime` should set the top two bits in each prime.
        // Thus each prime has the form
        //   p_i = 2^bitlen(p_i) × 0.11... (in base 2).
        // And the product is:
        //   P = 2^todo × α
        // where α is the product of nprimes numbers of the form 0.11...
        //
        // If α < 1/2 (which can happen for nprimes > 2), we need to
        // shift todo to compensate for lost bits: the mean value of 0.11...
        // is 7/8, so todo + shift - nprimes * log2(7/8) ~= bits - 1/2
        // will give good results.
        if nprimes >= 7 {
            todo += (nprimes - 2) / 5;
        }

        for (i, prime) in primes.iter_mut().enumerate() {
            *prime = random_prime(rng, todo / (nprimes - i));
            todo -= prime.bits();
        }

        // Makes sure that primes is pairwise unequal.
        for (i, prime1) in primes.iter().enumerate() {
            for prime2 in primes.iter().take(i) {
                if prime1 == prime2 {
                    continue 'next;
                }
            }
        }

        let n = compute_modulus(&primes);

        if n.bits() != bit_size {
            // This should never happen for nprimes == 2 because
            // gen_prime should set the top two bits in each prime.
            // For nprimes > 2 we hope it does not happen often.
            continue 'next;
        }

        if let Ok(d) = compute_private_exponent_euler_totient(&primes, exp) {
            n_final = n;
            d_final = d;
            break;
        }
    }

    Ok(RsaPrivateKeyComponents {
        n: n_final,
        e: exp.clone(),
        d: d_final,
        primes,
    })
}

fn available_top_bit_primes(bits: usize) -> usize {
    if bits < 2 {
        return 0;
    }

    let start = 3u64 << (bits - 2);
    let end = 1u64 << bits;
    ((start | 1)..end)
        .step_by(2)
        .filter(|n| is_prime(*n))
        .count()
}

fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    if n == 2 {
        return true;
    }
    if n % 2 == 0 {
        return false;
    }

    let mut d = 3u64;
    while d <= n / d {
        if n % d == 0 {
            return false;
        }
        d += 2;
    }
    true
}
