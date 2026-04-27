//! Fixed-offset byte-order helpers for binary formats.

/// Read a little-endian `u16` from `input[offset..]`.
#[inline]
pub fn read_u16_le(input: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([input[offset], input[offset + 1]])
}

/// Read a little-endian `u32` from `input[offset..]`.
#[inline]
pub fn read_u32_le(input: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        input[offset],
        input[offset + 1],
        input[offset + 2],
        input[offset + 3],
    ])
}

/// Read a little-endian `u64` from `input[offset..]`.
#[inline]
pub fn read_u64_le(input: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        input[offset],
        input[offset + 1],
        input[offset + 2],
        input[offset + 3],
        input[offset + 4],
        input[offset + 5],
        input[offset + 6],
        input[offset + 7],
    ])
}

/// Read a big-endian `u16` from `input[offset..]`.
#[inline]
pub fn read_u16_be(input: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes([input[offset], input[offset + 1]])
}

/// Read a big-endian `u32` from `input[offset..]`.
#[inline]
pub fn read_u32_be(input: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([
        input[offset],
        input[offset + 1],
        input[offset + 2],
        input[offset + 3],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_little_endian_values() {
        let data = [0x78, 0x56, 0x34, 0x12, 0xef, 0xcd, 0xab, 0x90];
        assert_eq!(read_u16_le(&data, 0), 0x5678);
        assert_eq!(read_u32_le(&data, 0), 0x1234_5678);
        assert_eq!(read_u64_le(&data, 0), 0x90ab_cdef_1234_5678);
    }

    #[test]
    fn reads_big_endian_values() {
        let data = [0x12, 0x34, 0x56, 0x78];
        assert_eq!(read_u16_be(&data, 0), 0x1234);
        assert_eq!(read_u32_be(&data, 0), 0x1234_5678);
    }
}
