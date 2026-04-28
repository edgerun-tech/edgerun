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

/// Read a little-endian `i16` from `input[offset..]`.
#[inline]
pub fn read_i16_le(input: &[u8], offset: usize) -> i16 {
    i16::from_le_bytes([input[offset], input[offset + 1]])
}

/// Read a little-endian `i32` from `input[offset..]`.
#[inline]
pub fn read_i32_le(input: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        input[offset],
        input[offset + 1],
        input[offset + 2],
        input[offset + 3],
    ])
}

/// Read a little-endian `i64` from `input[offset..]`.
#[inline]
pub fn read_i64_le(input: &[u8], offset: usize) -> i64 {
    i64::from_le_bytes([
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

/// Checked little-endian `u16` read from `input[offset..]`.
#[inline]
pub fn try_read_u16_le(input: &[u8], offset: usize) -> Option<u16> {
    if input.len().saturating_sub(offset) < 2 {
        return None;
    }
    Some(read_u16_le(input, offset))
}

/// Checked little-endian `u32` read from `input[offset..]`.
#[inline]
pub fn try_read_u32_le(input: &[u8], offset: usize) -> Option<u32> {
    if input.len().saturating_sub(offset) < 4 {
        return None;
    }
    Some(read_u32_le(input, offset))
}

/// Checked little-endian `u64` read from `input[offset..]`.
#[inline]
pub fn try_read_u64_le(input: &[u8], offset: usize) -> Option<u64> {
    if input.len().saturating_sub(offset) < 8 {
        return None;
    }
    Some(read_u64_le(input, offset))
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

/// Checked big-endian `u16` read from `input[offset..]`.
#[inline]
pub fn try_read_u16_be(input: &[u8], offset: usize) -> Option<u16> {
    if input.len().saturating_sub(offset) < 2 {
        return None;
    }
    Some(read_u16_be(input, offset))
}

/// Checked big-endian `u32` read from `input[offset..]`.
#[inline]
pub fn try_read_u32_be(input: &[u8], offset: usize) -> Option<u32> {
    if input.len().saturating_sub(offset) < 4 {
        return None;
    }
    Some(read_u32_be(input, offset))
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
        assert_eq!(read_i16_le(&[0xff, 0xff], 0), -1);
        assert_eq!(read_i32_le(&[0xff, 0xff, 0xff, 0xff], 0), -1);
        assert_eq!(
            read_i64_le(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff], 0),
            -1
        );
    }

    #[test]
    fn reads_big_endian_values() {
        let data = [0x12, 0x34, 0x56, 0x78];
        assert_eq!(read_u16_be(&data, 0), 0x1234);
        assert_eq!(read_u32_be(&data, 0), 0x1234_5678);
    }

    #[test]
    fn checked_reads_return_none_when_short() {
        let data = [0x12, 0x34, 0x56, 0x78];
        assert_eq!(try_read_u16_le(&data, 3), None);
        assert_eq!(try_read_u32_le(&data, 1), None);
        assert_eq!(try_read_u64_le(&data, 0), None);
        assert_eq!(try_read_u16_be(&data, 3), None);
        assert_eq!(try_read_u32_be(&data, 1), None);
    }

    #[test]
    fn checked_reads_return_values_when_present() {
        let data = [0x78, 0x56, 0x34, 0x12, 0xef, 0xcd, 0xab, 0x90];
        assert_eq!(try_read_u16_le(&data, 0), Some(0x5678));
        assert_eq!(try_read_u32_le(&data, 0), Some(0x1234_5678));
        assert_eq!(try_read_u64_le(&data, 0), Some(0x90ab_cdef_1234_5678));
        assert_eq!(try_read_u16_be(&data, 0), Some(0x7856));
        assert_eq!(try_read_u32_be(&data, 0), Some(0x7856_3412));
    }
}
