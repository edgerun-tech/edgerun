const X25519_BASEPOINT_BYTES: [u8; 32] = {
    let mut bytes = [0u8; 32];
    bytes[0] = 9;
    bytes
};

const MASK51: u64 = (1u64 << 51) - 1;
const A24: FieldElement = FieldElement([121665, 0, 0, 0, 0]);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PublicKey([u8; 32]);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StaticSecret([u8; 32]);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SharedSecret([u8; 32]);

impl PublicKey {
    pub fn to_bytes(self) -> [u8; 32] {
        self.0
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl StaticSecret {
    pub fn diffie_hellman(&self, their_public: &PublicKey) -> SharedSecret {
        SharedSecret(x25519(self.0, their_public.0))
    }

    pub fn to_bytes(self) -> [u8; 32] {
        self.0
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl SharedSecret {
    pub fn to_bytes(self) -> [u8; 32] {
        self.0
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl From<[u8; 32]> for PublicKey {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl From<[u8; 32]> for StaticSecret {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl From<&StaticSecret> for PublicKey {
    fn from(secret: &StaticSecret) -> Self {
        Self(x25519(secret.0, X25519_BASEPOINT_BYTES))
    }
}

pub fn x25519(scalar: [u8; 32], point: [u8; 32]) -> [u8; 32] {
    let mut scalar = scalar;
    scalar[0] &= 248;
    scalar[31] &= 127;
    scalar[31] |= 64;

    let x1 = FieldElement::from_bytes(point);
    let mut x2 = FieldElement::one();
    let mut z2 = FieldElement::zero();
    let mut x3 = x1;
    let mut z3 = FieldElement::one();
    let mut swap = 0u64;

    for bit_index in (0..255).rev() {
        let bit = ((scalar[bit_index / 8] >> (bit_index & 7)) & 1) as u64;
        swap ^= bit;
        FieldElement::conditional_swap(&mut x2, &mut x3, swap);
        FieldElement::conditional_swap(&mut z2, &mut z3, swap);
        swap = bit;

        let a = x2.add(&z2);
        let aa = a.square();
        let b = x2.sub(&z2);
        let bb = b.square();
        let e = aa.sub(&bb);
        let c = x3.add(&z3);
        let d = x3.sub(&z3);
        let da = d.mul(&a);
        let cb = c.mul(&b);
        let da_plus_cb = da.add(&cb);
        let da_minus_cb = da.sub(&cb);

        x3 = da_plus_cb.square();
        z3 = x1.mul(&da_minus_cb.square());
        x2 = aa.mul(&bb);
        z2 = e.mul(&aa.add(&A24.mul(&e)));
    }

    FieldElement::conditional_swap(&mut x2, &mut x3, swap);
    FieldElement::conditional_swap(&mut z2, &mut z3, swap);
    x2.mul(&z2.invert()).to_bytes()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FieldElement([u64; 5]);

impl FieldElement {
    const fn zero() -> Self {
        Self([0, 0, 0, 0, 0])
    }

    const fn one() -> Self {
        Self([1, 0, 0, 0, 0])
    }

    fn from_bytes(mut bytes: [u8; 32]) -> Self {
        bytes[31] &= 127;
        Self([
            load_64(&bytes, 0) & MASK51,
            (load_64(&bytes, 6) >> 3) & MASK51,
            (load_64(&bytes, 12) >> 6) & MASK51,
            (load_64(&bytes, 19) >> 1) & MASK51,
            (load_64(&bytes, 24) >> 12) & MASK51,
        ])
    }

    fn to_bytes(self) -> [u8; 32] {
        let reduced = self.final_reduce();
        let limbs = reduced.0;
        let mut bytes = [0u8; 32];
        store_bits(&mut bytes, limbs[0] as u128, 0, 51);
        store_bits(&mut bytes, limbs[1] as u128, 51, 51);
        store_bits(&mut bytes, limbs[2] as u128, 102, 51);
        store_bits(&mut bytes, limbs[3] as u128, 153, 51);
        store_bits(&mut bytes, limbs[4] as u128, 204, 51);
        bytes
    }

    fn add(&self, rhs: &Self) -> Self {
        Self([
            self.0[0] + rhs.0[0],
            self.0[1] + rhs.0[1],
            self.0[2] + rhs.0[2],
            self.0[3] + rhs.0[3],
            self.0[4] + rhs.0[4],
        ])
        .carry()
    }

    fn sub(&self, rhs: &Self) -> Self {
        let base = 1u64 << 51;
        Self([
            self.0[0] + (base * 2 - 38) - rhs.0[0],
            self.0[1] + (base * 2 - 2) - rhs.0[1],
            self.0[2] + (base * 2 - 2) - rhs.0[2],
            self.0[3] + (base * 2 - 2) - rhs.0[3],
            self.0[4] + (base * 2 - 2) - rhs.0[4],
        ])
        .carry()
    }

    fn mul(&self, rhs: &Self) -> Self {
        let a = self.0;
        let b = rhs.0;
        let c0 = a[0] as u128 * b[0] as u128
            + 19u128
                * (a[1] as u128 * b[4] as u128
                    + a[2] as u128 * b[3] as u128
                    + a[3] as u128 * b[2] as u128
                    + a[4] as u128 * b[1] as u128);
        let c1 = a[0] as u128 * b[1] as u128
            + a[1] as u128 * b[0] as u128
            + 19u128
                * (a[2] as u128 * b[4] as u128
                    + a[3] as u128 * b[3] as u128
                    + a[4] as u128 * b[2] as u128);
        let c2 = a[0] as u128 * b[2] as u128
            + a[1] as u128 * b[1] as u128
            + a[2] as u128 * b[0] as u128
            + 19u128 * (a[3] as u128 * b[4] as u128 + a[4] as u128 * b[3] as u128);
        let c3 = a[0] as u128 * b[3] as u128
            + a[1] as u128 * b[2] as u128
            + a[2] as u128 * b[1] as u128
            + a[3] as u128 * b[0] as u128
            + 19u128 * (a[4] as u128 * b[4] as u128);
        let c4 = a[0] as u128 * b[4] as u128
            + a[1] as u128 * b[3] as u128
            + a[2] as u128 * b[2] as u128
            + a[3] as u128 * b[1] as u128
            + a[4] as u128 * b[0] as u128;
        Self::carry_wide([c0, c1, c2, c3, c4])
    }

    fn square(&self) -> Self {
        self.mul(self)
    }

    fn invert(&self) -> Self {
        let exponent = {
            let mut bytes = [0xffu8; 32];
            bytes[0] = 0xeb;
            bytes[31] = 0x7f;
            bytes
        };
        let mut result = Self::one();
        for bit_index in (0..255).rev() {
            result = result.square();
            if ((exponent[bit_index / 8] >> (bit_index & 7)) & 1) != 0 {
                result = result.mul(self);
            }
        }
        result
    }

    fn carry(self) -> Self {
        Self::carry_wide([
            self.0[0] as u128,
            self.0[1] as u128,
            self.0[2] as u128,
            self.0[3] as u128,
            self.0[4] as u128,
        ])
    }

    fn carry_wide(mut limbs: [u128; 5]) -> Self {
        for _ in 0..2 {
            let carry0 = limbs[0] >> 51;
            limbs[0] &= MASK51 as u128;
            limbs[1] += carry0;
            let carry1 = limbs[1] >> 51;
            limbs[1] &= MASK51 as u128;
            limbs[2] += carry1;
            let carry2 = limbs[2] >> 51;
            limbs[2] &= MASK51 as u128;
            limbs[3] += carry2;
            let carry3 = limbs[3] >> 51;
            limbs[3] &= MASK51 as u128;
            limbs[4] += carry3;
            let carry4 = limbs[4] >> 51;
            limbs[4] &= MASK51 as u128;
            limbs[0] += carry4 * 19;
        }
        Self([
            limbs[0] as u64,
            limbs[1] as u64,
            limbs[2] as u64,
            limbs[3] as u64,
            limbs[4] as u64,
        ])
    }

    fn final_reduce(self) -> Self {
        let reduced = self.carry().carry();
        let p = [MASK51 - 18, MASK51, MASK51, MASK51, MASK51];
        let mut out = [0u64; 5];
        let mut borrow = 0i128;
        for index in 0..5 {
            let value = reduced.0[index] as i128 - p[index] as i128 - borrow;
            if value < 0 {
                out[index] = (value + (1i128 << 51)) as u64;
                borrow = 1;
            } else {
                out[index] = value as u64;
                borrow = 0;
            }
        }
        if borrow == 0 {
            Self(out)
        } else {
            reduced
        }
    }

    fn conditional_swap(lhs: &mut Self, rhs: &mut Self, choice: u64) {
        let mask = 0u64.wrapping_sub(choice);
        for index in 0..5 {
            let swap = mask & (lhs.0[index] ^ rhs.0[index]);
            lhs.0[index] ^= swap;
            rhs.0[index] ^= swap;
        }
    }
}

fn load_64(bytes: &[u8; 32], offset: usize) -> u64 {
    let mut out = 0u64;
    for index in 0..8 {
        out |= (bytes[offset + index] as u64) << (8 * index);
    }
    out
}

fn store_bits(bytes: &mut [u8; 32], mut value: u128, bit_offset: usize, bit_len: usize) {
    for bit in 0..bit_len {
        if (value & 1) != 0 {
            let absolute = bit_offset + bit;
            bytes[absolute / 8] |= 1 << (absolute & 7);
        }
        value >>= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_secret_agrees_from_both_sides() {
        let alice = StaticSecret::from([1u8; 32]);
        let bob = StaticSecret::from([2u8; 32]);
        let alice_public = PublicKey::from(&alice);
        let bob_public = PublicKey::from(&bob);
        assert_eq!(
            alice.diffie_hellman(&bob_public).to_bytes(),
            bob.diffie_hellman(&alice_public).to_bytes()
        );
    }

    #[test]
    fn x25519_matches_rfc7748_vector() {
        let scalar = [
            0x77, 0x07, 0x6d, 0x0a, 0x73, 0x18, 0xa5, 0x7d, 0x3c, 0x16, 0xc1, 0x72, 0x51, 0xb2,
            0x66, 0x45, 0xdf, 0x4c, 0x2f, 0x87, 0xeb, 0xc0, 0x99, 0x2a, 0xb1, 0x77, 0xfb, 0xa5,
            0x1d, 0xb9, 0x2c, 0x2a,
        ];
        let point = X25519_BASEPOINT_BYTES;
        let expected = [
            0x85, 0x20, 0xf0, 0x09, 0x89, 0x30, 0xa7, 0x54, 0x74, 0x8b, 0x7d, 0xdc, 0xb4, 0x3e,
            0xf7, 0x5a, 0x0d, 0xbf, 0x3a, 0x0d, 0x26, 0x38, 0x1a, 0xf4, 0xeb, 0xa4, 0xa9, 0x8e,
            0xaa, 0x9b, 0x4e, 0x6a,
        ];
        assert_eq!(x25519(scalar, point), expected);
    }
}
