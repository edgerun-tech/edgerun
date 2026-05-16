//! Probabilistic primality checks used by RSA key generation.

use crate::num_bigint::BigUint;
use crate::num_bigint::Integer;
use crate::num_bigint::{FromPrimitive, One, ToPrimitive, Zero};

const PRIMES_A: u64 = 3 * 5 * 7 * 11 * 13 * 17 * 19 * 23 * 37;
const PRIMES_B: u64 = 29 * 31 * 41 * 43 * 47 * 53;
const PRIME_BIT_MASK: u64 = 1 << 2
    | 1 << 3
    | 1 << 5
    | 1 << 7
    | 1 << 11
    | 1 << 13
    | 1 << 17
    | 1 << 19
    | 1 << 23
    | 1 << 29
    | 1 << 31
    | 1 << 37
    | 1 << 41
    | 1 << 43
    | 1 << 47
    | 1 << 53
    | 1 << 59
    | 1 << 61;

pub(crate) fn probably_prime(x: &BigUint, n: usize) -> bool {
    if x.is_zero() {
        return false;
    }

    if x < &big(64) {
        return (PRIME_BIT_MASK & (1 << x.to_u64().unwrap())) != 0;
    }

    if x.is_even() {
        return false;
    }

    let r_a = x % PRIMES_A;
    let r_b = x % PRIMES_B;

    if (&r_a % 3u32).is_zero()
        || (&r_a % 5u32).is_zero()
        || (&r_a % 7u32).is_zero()
        || (&r_a % 11u32).is_zero()
        || (&r_a % 13u32).is_zero()
        || (&r_a % 17u32).is_zero()
        || (&r_a % 19u32).is_zero()
        || (&r_a % 23u32).is_zero()
        || (&r_a % 37u32).is_zero()
        || (&r_b % 29u32).is_zero()
        || (&r_b % 31u32).is_zero()
        || (&r_b % 41u32).is_zero()
        || (&r_b % 43u32).is_zero()
        || (&r_b % 47u32).is_zero()
        || (&r_b % 53u32).is_zero()
    {
        return false;
    }

    probably_prime_miller_rabin(x, n + 1, true) && probably_prime_lucas(x)
}

fn probably_prime_miller_rabin(n: &BigUint, reps: usize, force2: bool) -> bool {
    let one = BigUint::one();
    let two = big(2);
    let nm1 = n - &one;
    let k = nm1.trailing_zeros().unwrap() as usize;
    let q = &nm1 >> k;
    let nm3 = n - &big(3);
    let mut rng = WitnessRng::new(n.get_limb(0) as u64);

    'nextrandom: for i in 0..reps {
        let x = if i == reps - 1 && force2 {
            two.clone()
        } else {
            rng.biguint_below(&nm3) + &two
        };

        let mut y = x.modpow(&q, n);
        if y.is_one() || y == nm1 {
            continue;
        }

        for _ in 1..k {
            y = y.modpow(&two, n);
            if y == nm1 {
                continue 'nextrandom;
            }
            if y.is_one() {
                return false;
            }
        }
        return false;
    }

    true
}

fn probably_prime_lucas(n: &BigUint) -> bool {
    if n.is_zero() || n.is_one() {
        return false;
    }

    if n.to_u64() == Some(2) {
        return false;
    }

    let one = BigUint::one();
    let two = big(2);
    let mut p = 3u64;

    loop {
        if p > 10000 {
            panic!("internal error: cannot find (D/n) = -1 for {:?}", n)
        }

        let d = BigUint::from_u64(p * p - 4).unwrap();
        let j = jacobi_positive(&d, n);

        if j == -1 {
            break;
        }
        if j == 0 {
            return n.to_u64() == Some(p + 2);
        }
        if p == 40 {
            let t1 = n.sqrt();
            let t1 = &t1 * &t1;
            if &t1 == n {
                return false;
            }
        }

        p += 1;
    }

    let mut s = n + &one;
    let r = s.trailing_zeros().unwrap() as usize;
    s = &s >> r;
    let nm2 = n - &two;

    let mut vk = two.clone();
    let mut vk1 = BigUint::from_u64(p).unwrap();

    for i in (0..s.bits()).rev() {
        if is_bit_set(&s, i) {
            let t1 = (&vk * &vk1) + n - p;
            vk = &t1 % n;
            let t1 = (&vk1 * &vk1) + &nm2;
            vk1 = &t1 % n;
        } else {
            let t1 = (&vk * &vk1) + n - p;
            vk1 = &t1 % n;
            let t1 = (&vk * &vk) + &nm2;
            vk = &t1 % n;
        }
    }

    if vk.to_u64() == Some(2) || vk == nm2 {
        let mut t1 = &vk * p;
        let mut t2 = &vk1 << 1;

        if t1 < t2 {
            core::mem::swap(&mut t1, &mut t2);
        }

        t1 -= t2;

        if (t1 % n).is_zero() {
            return true;
        }
    }

    for _ in 0..r - 1 {
        if vk.is_zero() {
            return true;
        }

        if vk.to_u64() == Some(2) {
            return false;
        }

        let t1 = (&vk * &vk) - &two;
        vk = &t1 % n;
    }

    false
}

fn jacobi_positive(x: &BigUint, y: &BigUint) -> isize {
    if !y.is_odd() {
        panic!(
            "invalid arguments, y must be an odd positive integer, but got {:?}",
            y
        );
    }

    let mut a = x % y;
    let mut b = y.clone();
    let mut j = 1;

    loop {
        if b.is_one() {
            return j;
        }
        if a.is_zero() {
            return 0;
        }

        let s = a.trailing_zeros().unwrap();
        if s & 1 != 0 {
            let bmod8 = b.get_limb(0) & 7;
            if bmod8 == 3 || bmod8 == 5 {
                j = -j;
            }
        }

        let c = &a >> s;
        if b.get_limb(0) & 3 == 3 && c.get_limb(0) & 3 == 3 {
            j = -j
        }

        a = b % &c;
        b = c;
    }
}

#[inline]
fn is_bit_set(x: &BigUint, i: usize) -> bool {
    let limb = i / 32;
    let shift = i % 32;
    ((x.get_limb(limb) >> shift) & 1) == 1
}

#[inline]
fn big(n: u64) -> BigUint {
    BigUint::from_u64(n).unwrap()
}

struct WitnessRng(u64);

impl WitnessRng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_add(0x9e37_79b9_7f4a_7c15))
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn biguint_below(&mut self, bound: &BigUint) -> BigUint {
        assert!(!bound.is_zero());

        let bytes_len = bound.bits().div_ceil(8) as usize;
        let mut bytes = alloc::vec![0u8; bytes_len];
        let excess_bits = bytes_len * 8 - bound.bits() as usize;

        loop {
            for chunk in bytes.chunks_mut(8) {
                let word = self.next_u64().to_le_bytes();
                chunk.copy_from_slice(&word[..chunk.len()]);
            }
            if excess_bits > 0 {
                bytes[bytes_len - 1] &= (1u8 << (8 - excess_bits)) - 1;
            }

            let n = BigUint::from_bytes_le(&bytes);
            if n < *bound {
                return n;
            }
        }
    }
}
