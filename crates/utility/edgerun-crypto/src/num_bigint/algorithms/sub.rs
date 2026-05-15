use core::cmp;
use core::cmp::Ordering::*;

use crate::num_bigint::SmallVec;
use crate::num_bigint::Zero;

use crate::num_bigint::algorithms::cmp_slice;
use crate::num_bigint::big_digit::{BITS, BigDigit, SignedDoubleBigDigit};
use crate::num_bigint::{BigUint, VEC_SIZE};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Sign {
    Minus,
    NoSign,
    Plus,
}

impl core::ops::Mul for Sign {
    type Output = Sign;

    fn mul(self, other: Sign) -> Sign {
        match (self, other) {
            (Sign::NoSign, _) | (_, Sign::NoSign) => Sign::NoSign,
            (Sign::Plus, Sign::Plus) | (Sign::Minus, Sign::Minus) => Sign::Plus,
            (Sign::Plus, Sign::Minus) | (Sign::Minus, Sign::Plus) => Sign::Minus,
        }
    }
}

// Subtract with borrow:
#[inline]
pub fn sbb(a: BigDigit, b: BigDigit, acc: &mut SignedDoubleBigDigit) -> BigDigit {
    *acc += a as SignedDoubleBigDigit;
    *acc -= b as SignedDoubleBigDigit;
    let lo = *acc as BigDigit;
    *acc >>= BITS;
    lo
}

pub fn sub2(a: &mut [BigDigit], b: &[BigDigit]) {
    let mut borrow = 0;

    let len = cmp::min(a.len(), b.len());
    let (a_lo, a_hi) = a.split_at_mut(len);
    let (b_lo, b_hi) = b.split_at(len);

    for (a, b) in a_lo.iter_mut().zip(b_lo) {
        *a = sbb(*a, *b, &mut borrow);
    }

    if borrow != 0 {
        for a in a_hi {
            *a = sbb(*a, 0, &mut borrow);
            if borrow == 0 {
                break;
            }
        }
    }

    // note: we're _required_ to fail on underflow
    assert!(
        borrow == 0 && b_hi.iter().all(|x| *x == 0),
        "Cannot subtract b from a because b is larger than a."
    );
}

// Only for the Sub impl. `a` and `b` must have same length.
#[inline]
pub fn __sub2rev(a: &[BigDigit], b: &mut [BigDigit]) -> BigDigit {
    debug_assert!(b.len() == a.len());

    let mut borrow = 0;

    for (ai, bi) in a.iter().zip(b) {
        *bi = sbb(*ai, *bi, &mut borrow);
    }

    borrow as BigDigit
}

pub fn sub2rev(a: &[BigDigit], b: &mut [BigDigit]) {
    debug_assert!(b.len() >= a.len());

    let len = cmp::min(a.len(), b.len());
    let (a_lo, a_hi) = a.split_at(len);
    let (b_lo, b_hi) = b.split_at_mut(len);

    let borrow = __sub2rev(a_lo, b_lo);

    assert!(a_hi.is_empty());

    // note: we're _required_ to fail on underflow
    assert!(
        borrow == 0 && b_hi.iter().all(|x| *x == 0),
        "Cannot subtract b from a because b is larger than a."
    );
}

pub fn sub_sign(a: &[BigDigit], b: &[BigDigit]) -> (Sign, BigUint) {
    // Normalize:
    let a = &a[..a.iter().rposition(|&x| x != 0).map_or(0, |i| i + 1)];
    let b = &b[..b.iter().rposition(|&x| x != 0).map_or(0, |i| i + 1)];

    match cmp_slice(a, b) {
        Greater => {
            let mut a: SmallVec<[BigDigit; VEC_SIZE]> = a.into();
            sub2(&mut a, b);
            (Sign::Plus, BigUint::new_native(a))
        }
        Less => {
            let mut b: SmallVec<[BigDigit; VEC_SIZE]> = b.into();
            sub2(&mut b, a);
            (Sign::Minus, BigUint::new_native(b))
        }
        _ => (Sign::NoSign, Zero::zero()),
    }
}
