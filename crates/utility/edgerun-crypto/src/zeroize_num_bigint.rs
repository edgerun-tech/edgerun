use crate::zeroize::Zeroize;
use crate::num_bigint::{BigInt, BigUint};

impl Zeroize for BigUint {
    fn zeroize(&mut self) {
        BigUint::zeroize(self);
    }
}

impl Zeroize for BigInt {
    fn zeroize(&mut self) {
        BigInt::zeroize(self);
    }
}
