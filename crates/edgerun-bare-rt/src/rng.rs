//! Random number generation for bare-metal

#![allow(dead_code)]

pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn new_from_entropy() -> Self {
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
