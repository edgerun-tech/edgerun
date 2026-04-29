//! CRC-32/IEEE utilities.

/// Incremental CRC-32/IEEE accumulator.
pub struct Crc32(u32);

impl Crc32 {
    pub const CRC_INIT: u32 = 0xffff_ffff;

    pub fn new() -> Self {
        Self(Self::CRC_INIT)
    }

    pub fn update(&mut self, data: &[u8]) -> u32 {
        let mut crc = self.0;
        for &byte in data {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                let mask = 0u32.wrapping_sub(crc & 1);
                crc = (crc >> 1) ^ (0xedb8_8320 & mask);
            }
        }
        self.0 = crc;
        !crc
    }

    pub fn crc(&self) -> u32 {
        !self.0
    }
}

impl Default for Crc32 {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute CRC-32/IEEE for `data`.
pub fn crc32(data: &[u8]) -> u32 {
    Crc32::new().update(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_known_value() {
        assert_eq!(crc32(b"hello"), 0x3610_a686);
    }

    #[test]
    fn incremental_matches_single_pass() {
        let mut crc = Crc32::new();
        crc.update(b"hel");
        crc.update(b"lo");
        assert_eq!(crc.crc(), crc32(b"hello"));
    }
}
