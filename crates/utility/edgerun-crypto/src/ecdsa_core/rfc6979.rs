use crate::elliptic_curve::generic_array::{ArrayLength, GenericArray};
use crate::digest::{
    core_api::BlockSizeUser, generic_array::typenum::Unsigned, Digest, FixedOutput,
    FixedOutputReset, Output,
};
use crate::subtle::{Choice, ConditionallySelectable, ConstantTimeEq};

pub type ByteArray<Size> = GenericArray<u8, Size>;

#[inline]
pub fn generate_k<D, N>(
    x: &ByteArray<N>,
    n: &ByteArray<N>,
    h: &ByteArray<N>,
    data: &[u8],
) -> ByteArray<N>
where
    D: Digest + BlockSizeUser + FixedOutput<OutputSize = N> + FixedOutputReset,
    N: ArrayLength<u8>,
{
    let mut hmac_drbg = HmacDrbg::<D>::new(x, h, data);

    loop {
        let mut k = ByteArray::<N>::default();
        hmac_drbg.fill_bytes(&mut k);

        let k_is_zero = ct_eq(&k, &ByteArray::default());
        if (!k_is_zero & ct_lt(&k, n)).into() {
            return k;
        }
    }
}

struct HmacDrbg<D>
where
    D: Digest + BlockSizeUser + FixedOutputReset,
{
    k: Output<D>,
    v: Output<D>,
}

impl<D> HmacDrbg<D>
where
    D: Digest + BlockSizeUser + FixedOutputReset,
{
    fn new(entropy_input: &[u8], nonce: &[u8], additional_data: &[u8]) -> Self {
        let mut k = Output::<D>::default();
        let mut v = Output::<D>::default();

        for b in &mut v {
            *b = 0x01;
        }

        for i in 0..=1 {
            let mut input = alloc::vec::Vec::new();
            input.extend_from_slice(&v);
            input.push(i);
            input.extend_from_slice(entropy_input);
            input.extend_from_slice(nonce);
            input.extend_from_slice(additional_data);
            k = hmac::<D>(&k, &input);
            v = hmac::<D>(&k, &v);
        }

        Self { k, v }
    }

    fn fill_bytes(&mut self, out: &mut [u8]) {
        for out_chunk in out.chunks_mut(self.v.len()) {
            self.v = hmac::<D>(&self.k, &self.v);
            out_chunk.copy_from_slice(&self.v[..out_chunk.len()]);
        }

        let mut input = alloc::vec::Vec::new();
        input.extend_from_slice(&self.v);
        input.push(0x00);
        self.k = hmac::<D>(&self.k, &input);
        self.v = hmac::<D>(&self.k, &self.v);
    }
}

fn hmac<D>(key: &[u8], data: &[u8]) -> Output<D>
where
    D: Digest + BlockSizeUser + FixedOutputReset,
{
    let mut key_block = GenericArray::<u8, <D as BlockSizeUser>::BlockSize>::default();
    if key.len() <= key_block.len() {
        key_block[..key.len()].copy_from_slice(key);
    } else {
        let hash = D::digest(key);
        let n = core::cmp::min(hash.len(), key_block.len());
        key_block[..n].copy_from_slice(&hash[..n]);
    }

    let mut ipad = key_block.clone();
    let mut opad = key_block;
    for byte in ipad.iter_mut() {
        *byte ^= 0x36;
    }
    for byte in opad.iter_mut() {
        *byte ^= 0x5c;
    }

    let mut inner = D::new();
    Digest::update(&mut inner, &ipad);
    Digest::update(&mut inner, data);
    let inner = inner.finalize();

    let mut outer = D::new();
    Digest::update(&mut outer, &opad);
    Digest::update(&mut outer, inner);
    outer.finalize()
}

fn ct_eq<N: ArrayLength<u8>>(a: &ByteArray<N>, b: &ByteArray<N>) -> Choice {
    let mut ret = Choice::from(1);

    for (a, b) in a.iter().zip(b.iter()) {
        ret.conditional_assign(&Choice::from(0), !a.ct_eq(b));
    }

    ret
}

fn ct_lt<N: ArrayLength<u8>>(a: &ByteArray<N>, b: &ByteArray<N>) -> Choice {
    let mut borrow = 0;

    for (&a, &b) in a.iter().zip(b.iter()).rev() {
        let c = (b as u16).wrapping_add(borrow >> (u8::BITS - 1));
        borrow = (a as u16).wrapping_sub(c) >> u8::BITS as u8;
    }

    !borrow.ct_eq(&0)
}
