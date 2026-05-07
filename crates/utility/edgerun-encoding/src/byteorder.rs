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

/// Read a 24-bit big-endian unsigned integer from `input[offset..]`.
#[inline]
pub fn read_u24_be(input: &[u8], offset: usize) -> u32 {
    (u32::from(input[offset]) << 16)
        | (u32::from(input[offset + 1]) << 8)
        | u32::from(input[offset + 2])
}

/// Read a big-endian `i32` from `input[offset..]`.
#[inline]
pub fn read_i32_be(input: &[u8], offset: usize) -> i32 {
    i32::from_be_bytes([
        input[offset],
        input[offset + 1],
        input[offset + 2],
        input[offset + 3],
    ])
}

/// Read a big-endian `u64` from `input[offset..]`.
#[inline]
pub fn read_u64_be(input: &[u8], offset: usize) -> u64 {
    u64::from_be_bytes([
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

/// Read a big-endian `i64` from `input[offset..]`.
#[inline]
pub fn read_i64_be(input: &[u8], offset: usize) -> i64 {
    i64::from_be_bytes([
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

/// Checked 24-bit big-endian unsigned integer read from `input[offset..]`.
#[inline]
pub fn try_read_u24_be(input: &[u8], offset: usize) -> Option<u32> {
    if input.len().saturating_sub(offset) < 3 {
        return None;
    }
    Some(read_u24_be(input, offset))
}

/// Checked big-endian `i32` read from `input[offset..]`.
#[inline]
pub fn try_read_i32_be(input: &[u8], offset: usize) -> Option<i32> {
    if input.len().saturating_sub(offset) < 4 {
        return None;
    }
    Some(read_i32_be(input, offset))
}

/// Checked big-endian `u64` read from `input[offset..]`.
#[inline]
pub fn try_read_u64_be(input: &[u8], offset: usize) -> Option<u64> {
    if input.len().saturating_sub(offset) < 8 {
        return None;
    }
    Some(read_u64_be(input, offset))
}

/// Checked big-endian `i64` read from `input[offset..]`.
#[inline]
pub fn try_read_i64_be(input: &[u8], offset: usize) -> Option<i64> {
    if input.len().saturating_sub(offset) < 8 {
        return None;
    }
    Some(read_i64_be(input, offset))
}

/// Append a big-endian `u16` to an output buffer.
#[inline]
pub fn push_u16_be(out: &mut alloc::vec::Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}

/// Append a 24-bit big-endian unsigned integer to an output buffer.
///
/// Panics if `value` does not fit in 24 bits.
#[inline]
pub fn push_u24_be(out: &mut alloc::vec::Vec<u8>, value: u32) {
    assert!(value <= 0x00ff_ffff, "value too large for u24: {value}");
    out.push((value >> 16) as u8);
    out.push((value >> 8) as u8);
    out.push(value as u8);
}

/// Append a big-endian `u32` to an output buffer.
#[inline]
pub fn push_u32_be(out: &mut alloc::vec::Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

/// Append a big-endian `u64` to an output buffer.
#[inline]
pub fn push_u64_be(out: &mut alloc::vec::Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_be_bytes());
}

/// Append a little-endian `u16` to an output buffer.
#[inline]
pub fn push_u16_le(out: &mut alloc::vec::Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// Append a little-endian `u32` to an output buffer.
#[inline]
pub fn push_u32_le(out: &mut alloc::vec::Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// Append a little-endian `u64` to an output buffer.
#[inline]
pub fn push_u64_le(out: &mut alloc::vec::Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// Append a little-endian `i32` to an output buffer.
#[inline]
pub fn push_i32_le(out: &mut alloc::vec::Vec<u8>, value: i32) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// Write a big-endian `u16` into `out[offset..]`.
#[inline]
pub fn write_u16_be(out: &mut [u8], offset: usize, value: u16) {
    out[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
}

/// Write a 24-bit big-endian unsigned integer into `out[offset..]`.
///
/// Panics if `value` does not fit in 24 bits.
#[inline]
pub fn write_u24_be(out: &mut [u8], offset: usize, value: u32) {
    assert!(value <= 0x00ff_ffff, "value too large for u24: {value}");
    out[offset] = (value >> 16) as u8;
    out[offset + 1] = (value >> 8) as u8;
    out[offset + 2] = value as u8;
}

/// Write a big-endian `u32` into `out[offset..]`.
#[inline]
pub fn write_u32_be(out: &mut [u8], offset: usize, value: u32) {
    out[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

/// Write a little-endian `u16` into `out[offset..]`.
#[inline]
pub fn write_u16_le(out: &mut [u8], offset: usize, value: u16) {
    out[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

/// Write a little-endian `u32` into `out[offset..]`.
#[inline]
pub fn write_u32_le(out: &mut [u8], offset: usize, value: u32) {
    out[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

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
        let data = [0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef];
        assert_eq!(read_u16_be(&data, 0), 0x1234);
        assert_eq!(read_u24_be(&data, 0), 0x1234_56);
        assert_eq!(read_u32_be(&data, 0), 0x1234_5678);
        assert_eq!(read_u64_be(&data, 0), 0x1234_5678_90ab_cdef);
        assert_eq!(read_i32_be(&[0xff, 0xff, 0xff, 0xff], 0), -1);
        assert_eq!(
            read_i64_be(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff], 0),
            -1
        );
    }

    #[test]
    fn checked_reads_return_none_when_short() {
        let data = [0x12, 0x34, 0x56, 0x78];
        assert_eq!(try_read_u16_le(&data, 3), None);
        assert_eq!(try_read_u32_le(&data, 1), None);
        assert_eq!(try_read_u64_le(&data, 0), None);
        assert_eq!(try_read_u16_be(&data, 3), None);
        assert_eq!(try_read_u24_be(&data, 2), None);
        assert_eq!(try_read_u32_be(&data, 1), None);
        assert_eq!(try_read_i32_be(&data, 1), None);
        assert_eq!(try_read_u64_be(&data, 0), None);
        assert_eq!(try_read_i64_be(&data, 0), None);
    }

    #[test]
    fn checked_reads_return_values_when_present() {
        let data = [0x78, 0x56, 0x34, 0x12, 0xef, 0xcd, 0xab, 0x90];
        assert_eq!(try_read_u16_le(&data, 0), Some(0x5678));
        assert_eq!(try_read_u32_le(&data, 0), Some(0x1234_5678));
        assert_eq!(try_read_u64_le(&data, 0), Some(0x90ab_cdef_1234_5678));
        assert_eq!(try_read_u16_be(&data, 0), Some(0x7856));
        assert_eq!(try_read_u24_be(&data, 0), Some(0x7856_34));
        assert_eq!(try_read_u32_be(&data, 0), Some(0x7856_3412));
        assert_eq!(try_read_i32_be(&data, 0), Some(0x7856_3412));
        assert_eq!(try_read_u64_be(&data, 0), Some(0x7856_3412_efcd_ab90));
        assert_eq!(
            try_read_i64_be(&data, 0),
            Some(0x7856_3412_efcd_ab90_u64 as i64)
        );
    }

    #[test]
    fn writes_little_endian_values() {
        let mut out = Vec::new();
        push_u16_le(&mut out, 0xabcd);
        push_u32_le(&mut out, 0x1234_5678);
        push_u64_le(&mut out, 0x0123_4567_89ab_cdef);
        push_i32_le(&mut out, -2);
        assert_eq!(
            out,
            [
                0xcd, 0xab, 0x78, 0x56, 0x34, 0x12, 0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01,
                0xfe, 0xff, 0xff, 0xff
            ]
        );

        let mut data = [0; 6];
        write_u16_le(&mut data, 1, 0xabcd);
        write_u32_le(&mut data, 2, 0x1234_5678);
        assert_eq!(data, [0x00, 0xcd, 0x78, 0x56, 0x34, 0x12]);
    }
}
