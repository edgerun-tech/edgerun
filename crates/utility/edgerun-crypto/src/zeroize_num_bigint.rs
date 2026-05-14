use crate::num_bigint::BigUint;
use crate::zeroize::Zeroize;

impl Zeroize for BigUint {
    fn zeroize(&mut self) {
        BigUint::zeroize(self);
    }
}
