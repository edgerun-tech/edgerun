//! Random number generation for bare-metal

#![allow(dead_code)]

pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn mix_seed(&mut self, seed: u64) {
        self.state ^= seed;
        self.state = self
            .state
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            .wrapping_add(0xD1B5_4A32_D192_ED03);
    }

    pub fn mix_entropy(&mut self, bytes: &[u8]) {
        let mut acc = 0xA076_1D64_78BD_642Fu64;
        for (idx, byte) in bytes.iter().enumerate() {
            acc ^= (*byte as u64) << ((idx & 7) * 8);
            acc = acc.rotate_left(9).wrapping_mul(0xE703_7ED1_A0B4_28DB);
        }
        self.mix_seed(acc);
    }

    pub fn new_from_entropy() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            let mut seed: u64 = 0;
            unsafe {
                core::arch::asm!(
                    "rdtsc",
                    out("rax") seed,
                    out("rdx") _,
                );
            }
            Self { state: seed }
        }

        #[cfg(not(target_arch = "x86_64"))]
        {
            Self {
                state: 0xD1B5_4A32_D192_ED03,
            }
        }
    }

    pub fn next(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.state >> 33) as u32
    }

    pub fn next_u64(&mut self) -> u64 {
        ((self.next() as u64) << 32) | (self.next() as u64)
    }

    pub fn range(&mut self, min: u32, max: u32) -> u32 {
        if max <= min {
            return min;
        }
        let range = max - min + 1;
        min + (self.next() % range)
    }

    pub fn bytes(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.next() as u8;
        }
    }
}

impl Default for Rng {
    fn default() -> Self {
        Self::new_from_entropy()
    }
}
