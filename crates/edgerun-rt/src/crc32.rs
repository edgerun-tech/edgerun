//! CRC32 implementation

pub struct Crc32(u32);

impl Crc32 {
    pub const CRC_INIT: u32 = 0xFFFFFFFF;

    pub fn new() -> Self {
        Self(0xFFFFFFFF)
    }

    pub fn update(&mut self, data: &[u8]) -> u32 {
        let mut crc = self.0;
        for byte in data {
            crc ^= *byte as u32;
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0xEDB88320
                } else {
                    crc >> 1
                };
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

pub fn crc32(data: &[u8]) -> u32 {
    Crc32::new().update(data)
}
