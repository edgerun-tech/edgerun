//! DES and two-key 3DES helpers for legacy protocols.

extern crate alloc;

use alloc::vec::Vec;
use des::cipher::generic_array::GenericArray;
use des::cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
use des::Des;

pub const DES_BLOCK_SIZE: usize = 8;

#[derive(Clone)]
pub struct Tdes2 {
    k1: Des,
    k2: Des,
}

impl Tdes2 {
    pub fn new(key: &[u8; 16]) -> Self {
        Self {
            k1: Des::new(GenericArray::from_slice(&key[..8])),
            k2: Des::new(GenericArray::from_slice(&key[8..16])),
        }
    }

    pub fn encrypt_block(&self, block: &[u8; 8]) -> [u8; 8] {
        let mut block = GenericArray::clone_from_slice(block);
        self.k1.encrypt_block(&mut block);
        self.k2.decrypt_block(&mut block);
        self.k1.encrypt_block(&mut block);
        block.into()
    }

    pub fn decrypt_block(&self, block: &[u8; 8]) -> [u8; 8] {
        let mut block = GenericArray::clone_from_slice(block);
        self.k1.decrypt_block(&mut block);
        self.k2.encrypt_block(&mut block);
        self.k1.decrypt_block(&mut block);
        block.into()
    }

    pub fn cbc_encrypt(&self, iv: &[u8; 8], plaintext: &[u8]) -> Option<Vec<u8>> {
        if plaintext.len() % DES_BLOCK_SIZE != 0 {
            return None;
        }
        let mut prev = *iv;
        let mut out = Vec::with_capacity(plaintext.len());
        for chunk in plaintext.chunks(DES_BLOCK_SIZE) {
            let mut block = [0u8; DES_BLOCK_SIZE];
            block.copy_from_slice(chunk);
            xor_in_place(&mut block, &prev);
            let encrypted = self.encrypt_block(&block);
            out.extend_from_slice(&encrypted);
            prev = encrypted;
        }
        Some(out)
    }

    pub fn cbc_decrypt(&self, iv: &[u8; 8], ciphertext: &[u8]) -> Option<Vec<u8>> {
        if ciphertext.len() % DES_BLOCK_SIZE != 0 {
            return None;
        }
        let mut prev = *iv;
        let mut out = Vec::with_capacity(ciphertext.len());
        for chunk in ciphertext.chunks(DES_BLOCK_SIZE) {
            let mut block = [0u8; DES_BLOCK_SIZE];
            block.copy_from_slice(chunk);
            let decrypted = self.decrypt_block(&block);
            let mut plaintext = decrypted;
            xor_in_place(&mut plaintext, &prev);
            out.extend_from_slice(&plaintext);
            prev = block;
        }
        Some(out)
    }
}

pub fn iso9797_method2_pad(data: &[u8], block_size: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + block_size);
    out.extend_from_slice(data);
    out.push(0x80);
    while out.len() % block_size != 0 {
        out.push(0x00);
    }
    out
}

pub fn iso9797_method2_unpad(data: &[u8]) -> Option<Vec<u8>> {
    let mut pos = data.len();
    while pos > 0 && data[pos - 1] == 0x00 {
        pos -= 1;
    }
    if pos == 0 || data[pos - 1] != 0x80 {
        return None;
    }
    Some(data[..pos - 1].to_vec())
}

pub fn iso9797_alg3_mac_3des2(key: &[u8; 16], data: &[u8]) -> [u8; 8] {
    let k1 = Des::new(GenericArray::from_slice(&key[..8]));
    let k2 = Des::new(GenericArray::from_slice(&key[8..16]));
    let padded = iso9797_method2_pad(data, DES_BLOCK_SIZE);
    let mut state = [0u8; DES_BLOCK_SIZE];
    for chunk in padded.chunks(DES_BLOCK_SIZE) {
        let mut block = [0u8; DES_BLOCK_SIZE];
        block.copy_from_slice(chunk);
        xor_in_place(&mut block, &state);
        let mut ga = GenericArray::clone_from_slice(&block);
        k1.encrypt_block(&mut ga);
        state.copy_from_slice(&ga);
    }

    let mut ga = GenericArray::clone_from_slice(&state);
    k2.decrypt_block(&mut ga);
    k1.encrypt_block(&mut ga);
    ga.into()
}

fn xor_in_place(left: &mut [u8; 8], right: &[u8; 8]) {
    for i in 0..DES_BLOCK_SIZE {
        left[i] ^= right[i];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn method2_padding_roundtrips() {
        let padded = iso9797_method2_pad(b"abc", 8);
        assert_eq!(padded, vec![b'a', b'b', b'c', 0x80, 0, 0, 0, 0]);
        assert_eq!(iso9797_method2_unpad(&padded).unwrap(), b"abc");
    }

    #[test]
    fn tdes2_cbc_roundtrips() {
        let key = [
            0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD,
            0xEF, 0x01,
        ];
        let cipher = Tdes2::new(&key);
        let iv = [0u8; 8];
        let plaintext = b"12345678ABCDEFGH";
        let encrypted = cipher.cbc_encrypt(&iv, plaintext).unwrap();
        assert_ne!(encrypted, plaintext);
        assert_eq!(cipher.cbc_decrypt(&iv, &encrypted).unwrap(), plaintext);
    }
}
