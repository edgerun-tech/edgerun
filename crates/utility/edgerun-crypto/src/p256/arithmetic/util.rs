//! Helper functions.
// TODO(tarcieri): replace these with `crypto-bigint`

/// Computes `a + b + carry`, returning the result along with the new carry. 64-bit version.
#[inline(always)]
#[cfg(not(target_os = "none"))]
pub const fn adc(a: u64, b: u64, carry: u64) -> (u64, u64) {
    let ret = (a as u128) + (b as u128) + (carry as u128);
    (ret as u64, (ret >> 64) as u64)
}

/// Computes `a + b + carry`, returning the result along with the new carry.
#[inline(always)]
#[cfg(target_os = "none")]
pub const fn adc(a: u64, b: u64, carry: u64) -> (u64, u64) {
    let (sum, carry_a) = a.overflowing_add(b);
    let (sum, carry_b) = sum.overflowing_add(carry);
    (sum, (carry_a as u64) + (carry_b as u64))
}

/// Computes `a - (b + borrow)`, returning the result along with the new borrow. 64-bit version.
#[inline(always)]
#[cfg(not(target_os = "none"))]
pub const fn sbb(a: u64, b: u64, borrow: u64) -> (u64, u64) {
    let ret = (a as u128).wrapping_sub((b as u128) + ((borrow >> 63) as u128));
    (ret as u64, (ret >> 64) as u64)
}

/// Computes `a - (b + borrow)`, returning the result along with the new borrow.
#[inline(always)]
#[cfg(target_os = "none")]
pub const fn sbb(a: u64, b: u64, borrow: u64) -> (u64, u64) {
    let borrow = borrow >> 63;
    let (diff, borrow_a) = a.overflowing_sub(b);
    let (diff, borrow_b) = diff.overflowing_sub(borrow);
    let borrow = ((borrow_a as u64) | (borrow_b as u64)).wrapping_neg();
    (diff, borrow)
}

/// Computes `a + (b * c) + carry`, returning the result along with the new carry.
#[inline(always)]
#[cfg(not(target_os = "none"))]
pub const fn mac(a: u64, b: u64, c: u64, carry: u64) -> (u64, u64) {
    let ret = (a as u128) + ((b as u128) * (c as u128)) + (carry as u128);
    (ret as u64, (ret >> 64) as u64)
}

/// Computes `a + (b * c) + carry`, returning the result along with the new carry.
#[inline(always)]
#[cfg(target_os = "none")]
pub const fn mac(a: u64, b: u64, c: u64, carry: u64) -> (u64, u64) {
    let (lo, hi) = mul_64x64_128(b, c);
    let (lo, carry_a) = lo.overflowing_add(a);
    let (lo, carry_b) = lo.overflowing_add(carry);
    (lo, hi + (carry_a as u64) + (carry_b as u64))
}

#[inline(always)]
#[cfg(target_os = "none")]
const fn mul_64x64_128(a: u64, b: u64) -> (u64, u64) {
    const MASK: u64 = 0xffff_ffff;

    let a0 = a & MASK;
    let a1 = a >> 32;
    let b0 = b & MASK;
    let b1 = b >> 32;

    let p0 = a0 * b0;
    let p1 = a0 * b1;
    let p2 = a1 * b0;
    let p3 = a1 * b1;

    let middle = (p0 >> 32) + (p1 & MASK) + (p2 & MASK);
    let lo = (p0 & MASK) | (middle << 32);
    let hi = p3 + (p1 >> 32) + (p2 >> 32) + (middle >> 32);

    (lo, hi)
}
