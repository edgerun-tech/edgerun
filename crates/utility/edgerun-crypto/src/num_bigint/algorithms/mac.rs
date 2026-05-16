use core::cmp;
use core::iter::repeat;

use crate::num_bigint::BigUint;
use crate::num_bigint::algorithms::{Sign, adc, add2, sub_sign, sub2};
use crate::num_bigint::big_digit::{BITS, BigDigit, DoubleBigDigit};
use crate::smallvec;

#[inline]
pub fn mac_with_carry(a: BigDigit, b: BigDigit, c: BigDigit, acc: &mut DoubleBigDigit) -> BigDigit {
    *acc += a as DoubleBigDigit;
    *acc += (b as DoubleBigDigit) * (c as DoubleBigDigit);
    let lo = *acc as BigDigit;
    *acc >>= BITS;
    lo
}

// Three argument multiply accumulate:
// acc += b * c
pub fn mac_digit(acc: &mut [BigDigit], b: &[BigDigit], c: BigDigit) {
    if c == 0 {
        return;
    }

    let mut carry = 0;
    let (a_lo, a_hi) = acc.split_at_mut(b.len());

    for (a, &b) in a_lo.iter_mut().zip(b) {
        *a = mac_with_carry(*a, b, c, &mut carry);
    }

    let mut a = a_hi.iter_mut();
    while carry != 0 {
        let a = a.next().expect("carry overflow during multiplication!");
        *a = adc(*a, 0, &mut carry);
    }
}

// Three argument multiply accumulate:
// acc += b * c
pub fn mac3(acc: &mut [BigDigit], b: &[BigDigit], c: &[BigDigit]) {
    let (x, y) = if b.len() < c.len() { (b, c) } else { (c, b) };

    // We use three algorithms for different input sizes.
    //
    // - For small inputs, long multiplication is fastest.
    // - Next we use Karatsuba multiplication (Toom-2), which we have optimized
    //   to avoid unnecessary allocations for intermediate values.
    // - For the largest inputs we use Toom-3, which better optimizes the
    //   number of operations, but uses more temporary allocations.
    //
    // The thresholds are somewhat arbitrary, chosen by evaluating the results
    // of `cargo bench --bench bigint multiply`.

    if x.len() <= 32 {
        long(acc, x, y)
    } else if x.len() <= 256 {
        karatsuba(acc, x, y)
    } else {
        karatsuba(acc, x, y)
    }
}

// Long multiplication:
fn long(acc: &mut [BigDigit], x: &[BigDigit], y: &[BigDigit]) {
    for (i, xi) in x.iter().enumerate() {
        mac_digit(&mut acc[i..], y, *xi);
    }
}

// Karatsuba multiplication:
//
// The idea is that we break x and y up into two smaller numbers that each have about half
// as many digits, like so (note that multiplying by b is just a shift):
//
// x = x0 + x1 * b
// y = y0 + y1 * b
//
// With some algebra, we can compute x * y with three smaller products, where the inputs to
// each of the smaller products have only about half as many digits as x and y:
//
// x * y = (x0 + x1 * b) * (y0 + y1 * b)
//
// x * y = x0 * y0
//       + x0 * y1 * b
//       + x1 * y0 * b       + x1 * y1 * b^2
//
// Let p0 = x0 * y0 and p2 = x1 * y1:
//
// x * y = p0
//       + (x0 * y1 + x1 * y0) * b
//       + p2 * b^2
//
// The real trick is that middle term:
//
//   x0 * y1 + x1 * y0
// = x0 * y1 + x1 * y0 - p0 + p0 - p2 + p2
// = x0 * y1 + x1 * y0 - x0 * y0 - x1 * y1 + p0 + p2
//
// Now we complete the square:
//
// = -(x0 * y0 - x0 * y1 - x1 * y0 + x1 * y1) + p0 + p2
// = -((x1 - x0) * (y1 - y0)) + p0 + p2
//
// Let p1 = (x1 - x0) * (y1 - y0), and substitute back into our original formula:
//
// x * y = p0
//       + (p0 + p2 - p1) * b
//       + p2 * b^2
//
// Where the three intermediate products are:
//
// p0 = x0 * y0
// p1 = (x1 - x0) * (y1 - y0)
// p2 = x1 * y1
//
// In doing the computation, we take great care to avoid unnecessary temporary variables
// (since creating a BigUint requires a heap allocation): thus, we rearrange the formula a
// bit so we can use the same temporary variable for all the intermediate products:
//
// x * y = p2 * b^2 + p2 * b
//       + p0 * b + p0
//       - p1 * b
//
// The other trick we use is instead of doing explicit shifts, we slice acc at the
// appropriate offset when doing the add.
fn karatsuba(acc: &mut [BigDigit], x: &[BigDigit], y: &[BigDigit]) {
    /*
     * When x is smaller than y, it's significantly faster to pick b such that x is split in
     * half, not y:
     */
    let b = x.len() / 2;
    let (x0, x1) = x.split_at(b);
    let (y0, y1) = y.split_at(b);

    /*
     * We reuse the same BigUint for all the intermediate multiplies and have to size p
     * appropriately here: x1.len() >= x0.len and y1.len() >= y0.len():
     */
    let len = x1.len() + y1.len() + 1;
    let mut p = BigUint {
        data: smallvec![0; len],
    };

    // p2 = x1 * y1
    mac3(&mut p.data[..], x1, y1);

    // Not required, but the adds go faster if we drop any unneeded 0s from the end:
    p.normalize();

    add2(&mut acc[b..], &p.data[..]);
    add2(&mut acc[b * 2..], &p.data[..]);

    // Zero out p before the next multiply:
    p.data.truncate(0);
    p.data.extend(repeat(0).take(len));

    // p0 = x0 * y0
    mac3(&mut p.data[..], x0, y0);
    p.normalize();

    add2(&mut acc[..], &p.data[..]);
    add2(&mut acc[b..], &p.data[..]);

    // p1 = (x1 - x0) * (y1 - y0)
    // We do this one last, since it may be negative and acc can't ever be negative:
    let (j0_sign, j0) = sub_sign(x1, x0);
    let (j1_sign, j1) = sub_sign(y1, y0);

    match j0_sign * j1_sign {
        Sign::Plus => {
            p.data.truncate(0);
            p.data.extend(repeat(0).take(len));

            mac3(&mut p.data[..], &j0.data[..], &j1.data[..]);
            p.normalize();

            sub2(&mut acc[b..], &p.data[..]);
        }
        Sign::Minus => {
            mac3(&mut acc[b..], &j0.data[..], &j1.data[..]);
        }
        Sign::NoSign => (),
    }
}
