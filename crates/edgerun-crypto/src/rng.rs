//! Random number generation using hardware RDRAND (x86_64) or software fallback.

use crate::error::Result;

#[cfg(target_arch = "x86_64")]
pub mod rdrand {
    use core::arch::x86_64::_rdrand32_step;
    use core::arch::x86_64::_rdrand64_step;

    #[inline]
    pub fn u32() -> Option<u32> {
        let mut val: u32 = 0;
        unsafe {
            if _rdrand32_step(&mut val) == 1 {
                Some(val)
            } else {
                None
            }
        }
    }

    #[inline]
    pub fn u64() -> Option<u64> {
        let mut val: u64 = 0;
        unsafe {
            if _rdrand64_step(&mut val) == 1 {
                Some(val)
            } else {
                None
            }
        }
    }

    pub fn fill_bytes(buf: &mut [u8]) -> bool {
        let mut i = 0usize;
        while i < buf.len() {
            let chunk = &mut buf[i..core::cmp::min(i + 4, buf.len())];
            if chunk.len() == 4 {
                if let Some(v) = u32() {
                    chunk.copy_from_slice(&v.to_le_bytes());
                    i += 4;
                } else {
                    return false;
                }
            } else if chunk.len() > 0 {
                if let Some(v) = u32() {
                    chunk.copy_from_slice(&v.to_le_bytes()[..chunk.len()]);
                    i += chunk.len();
                } else {
                    return false;
                }
            }
        }
        true
    }
}

#[cfg(not(target_arch = "x86_64"))]
pub mod rdrand {
    use crate::error::CryptoError;

    static mut STATE: u64 = 0x1234567890ABCDEF;

    #[inline(always)]
    fn xorshift64(state: u64) -> u64 {
        let mut x = state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    }

    #[inline]
    pub fn u32() -> Option<u32> {
        unsafe {
            STATE = xorshift64(STATE);
            Some((STATE >> 32) as u32)
        }
    }

    #[inline]
    pub fn u64() -> Option<u64> {
        unsafe {
            STATE = xorshift64(STATE);
            Some(STATE)
        }
    }

    pub fn fill_bytes(buf: &mut [u8]) -> bool {
        for chunk in buf.chunks_mut(4) {
            if let Some(v) = u32() {
                chunk.copy_from_slice(&v.to_le_bytes()[..chunk.len()]);
            } else {
                return false;
            }
        }
        true
    }
}

pub fn random_bytes(len: usize) -> Result<alloc::vec::Vec<u8>> {
    let mut buf = alloc::vec::Vec::with_capacity(len);
    buf.resize(len, 0);
    if !rdrand::fill_bytes(&mut buf) {
        return Err(CryptoError::RandomGenerationFailed);
    }
    Ok(buf)
}

pub fn random_u32() -> Result<u32> {
    rdrand::u32().ok_or(CryptoError::RandomGenerationFailed)
}

pub fn random_u64() -> Result<u64> {
    rdrand::u64().ok_or(CryptoError::RandomGenerationFailed)
}

#[inline]
pub fn fill_random(buf: &mut [u8]) -> Result<()> {
    if !rdrand::fill_bytes(buf) {
        return Err(CryptoError::RandomGenerationFailed);
    }
    Ok(())
}