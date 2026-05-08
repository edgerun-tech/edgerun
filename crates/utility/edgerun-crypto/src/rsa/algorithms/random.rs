use alloc::vec::Vec;

use crate::num_bigint::BigUint;
use crate::num_bigint::{FromPrimitive, ToPrimitive, Zero};

use crate::rand_core::RngCore;
use crate::rsa::algorithms::prime::probably_prime;

const SMALL_PRIMES: [u8; 15] = [3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53];
const SMALL_PRIMES_PRODUCT: u64 = 16_294_579_238_595_022_365;

pub(crate) fn random_biguint(rng: &mut (impl RngCore + ?Sized), bit_size: usize) -> BigUint {
    let bytes_len = bit_size.div_ceil(8);
    if bytes_len == 0 {
        return BigUint::zero();
    }

    let mut bytes = Vec::with_capacity(bytes_len);
    bytes.resize(bytes_len, 0);
    rng.fill_bytes(&mut bytes);

    let excess_bits = bytes_len * 8 - bit_size;
    if excess_bits > 0 {
        bytes[0] >>= excess_bits;
    }

    BigUint::from_bytes_be(&bytes)
}

pub(crate) fn random_biguint_below(rng: &mut (impl RngCore + ?Sized), bound: &BigUint) -> BigUint {
    assert!(!bound.is_zero());

    loop {
        let n = random_biguint(rng, bound.bits() as usize);
        if n < *bound {
            return n;
        }
    }
}

pub(crate) fn random_prime(rng: &mut (impl RngCore + ?Sized), bit_size: usize) -> BigUint {
    assert!(bit_size >= 2, "prime size must be at least 2-bit");

    let mut top_bits = bit_size % 8;
    if top_bits == 0 {
        top_bits = 8;
    }

    let bytes_len = bit_size.div_ceil(8);
    let mut bytes = Vec::with_capacity(bytes_len);
    bytes.resize(bytes_len, 0);

    loop {
        rng.fill_bytes(&mut bytes);
        bytes[0] &= ((1u32 << top_bits as u32) - 1) as u8;

        if top_bits >= 2 {
            bytes[0] |= 3u8.wrapping_shl(top_bits as u32 - 2);
        } else {
            bytes[0] |= 1;
            if bytes_len > 1 {
                bytes[1] |= 0x80;
            }
        }

        bytes[bytes_len - 1] |= 1;

        let mut candidate = BigUint::from_bytes_be(&bytes);
        let rem = (&candidate % SMALL_PRIMES_PRODUCT).to_u64().unwrap();

        'next_delta: for delta in (0..(1u64 << 20)).step_by(2) {
            let maybe_divisible = rem + delta;

            for prime in SMALL_PRIMES {
                if maybe_divisible % u64::from(prime) == 0
                    && (bit_size > 6 || maybe_divisible != u64::from(prime))
                {
                    continue 'next_delta;
                }
            }

            if delta > 0 {
                candidate += BigUint::from_u64(delta).unwrap();
            }

            break;
        }

        if candidate.bits() == bit_size && probably_prime(&candidate, 20) {
            return candidate;
        }
    }
}
