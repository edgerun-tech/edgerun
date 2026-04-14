//! Varint encoding/decoding (protobuf-style LEB128).
//!
//! Supports encoding `u64` values as variable-length integers where each byte
//! uses 7 bits for data and the MSB as a continuation flag.
//!
//! # Examples
//! ```
//! use edgerun_encoding::varint::encode_varint;
//! assert_eq!(encode_varint(0), vec![0x00]);
//! assert_eq!(encode_varint(1), vec![0x01]);
//! assert_eq!(encode_varint(127), vec![0x7F]);
//! assert_eq!(encode_varint(128), vec![0x80, 0x01]);
//! ```

use alloc::vec::Vec;

/// Encodes a `u64` as a varint.
///
/// Returns a `Vec<u8>` containing the variable-length encoded integer.
/// Values 0–127 produce 1 byte; larger values produce up to 10 bytes.
pub fn encode_varint(mut v: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(10);
    loop {
        let mut byte = (v & 0x7F) as u8;
        v >>= 7;
        if v != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if v == 0 {
            break;
        }
    }
    out
}

/// Varint decode error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VarintError {
    /// No bytes were available (EOF before any data).
    UnexpectedEof,
    /// Incomplete varint at EOF.
    TruncatedVarint,
    /// Varint encoding exceeded 64 bits.
    TooLong,
}

impl core::fmt::Display for VarintError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            VarintError::UnexpectedEof => f.write_str("unexpected end of data"),
            VarintError::TruncatedVarint => f.write_str("truncated varint"),
            VarintError::TooLong => f.write_str("varint too long"),
        }
    }
}

/// Decodes a varint from a byte slice.
///
/// Returns `(value, bytes_consumed)` on success.
/// Returns `Err(VarintError::UnexpectedEof)` if the slice is empty.
/// Returns `Err(VarintError::TruncatedVarint)` if the data ends mid-varint
/// (all bytes have continuation bit set).
///
/// # Examples
/// ```
/// use edgerun_encoding::varint::decode_varint_slice;
/// let (v, n) = decode_varint_slice(&[0x80, 0x01]).unwrap();
/// assert_eq!(v, 128);
/// assert_eq!(n, 2);
/// ```
pub fn decode_varint_slice(data: &[u8]) -> Result<(u64, usize), VarintError> {
    let mut result: u64 = 0;
    let mut shift: u32 = 0;
    let mut i = 0;

    if data.is_empty() {
        return Err(VarintError::UnexpectedEof);
    }

    loop {
        if i >= data.len() {
            return Err(VarintError::TruncatedVarint);
        }
        let b = data[i];
        result |= ((b & 0x7F) as u64) << shift;
        i += 1;
        if b & 0x80 == 0 {
            return Ok((result, i));
        }
        shift += 7;
        if shift >= 64 {
            return Err(VarintError::TooLong);
        }
    }
}

/// Decodes a varint from a byte iterator, returning the value and remaining iterator.
///
/// Useful when decoding from a `&mut impl Iterator<Item = u8>`.
pub fn decode_varint_iter<I>(iter: &mut I) -> Result<u64, VarintError>
where
    I: Iterator<Item = u8>,
{
    let mut result: u64 = 0;
    let mut shift: u32 = 0;

    loop {
        let b = iter.next().ok_or_else(|| {
            if shift == 0 {
                VarintError::UnexpectedEof
            } else {
                VarintError::TruncatedVarint
            }
        })?;
        result |= ((b & 0x7F) as u64) << shift;
        if b & 0x80 == 0 {
            return Ok(result);
        }
        shift += 7;
        if shift >= 64 {
            return Err(VarintError::TooLong);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn encode_zero() {
        assert_eq!(encode_varint(0), vec![0x00]);
    }

    #[test]
    fn encode_one() {
        assert_eq!(encode_varint(1), vec![0x01]);
    }

    #[test]
    fn encode_127() {
        assert_eq!(encode_varint(127), vec![0x7F]);
    }

    #[test]
    fn encode_128() {
        assert_eq!(encode_varint(128), vec![0x80, 0x01]);
    }

    #[test]
    fn encode_300() {
        assert_eq!(encode_varint(300), vec![0xAC, 0x02]);
    }

    #[test]
    fn encode_max_u64() {
        let encoded = encode_varint(u64::MAX);
        assert_eq!(encoded.len(), 10);
    }

    #[test]
    fn roundtrip_small() {
        for v in 0..=255 {
            let encoded = encode_varint(v);
            let (decoded, _) = decode_varint_slice(&encoded).unwrap();
            assert_eq!(decoded, v);
        }
    }

    #[test]
    fn roundtrip_various() {
        for v in [0, 1, 127, 128, 255, 256, 16383, 16384, u32::MAX as u64, u64::MAX] {
            let encoded = encode_varint(v);
            let (decoded, _) = decode_varint_slice(&encoded).unwrap();
            assert_eq!(decoded, v);
        }
    }

    #[test]
    fn decode_empty() {
        assert_eq!(decode_varint_slice(&[]), Err(VarintError::UnexpectedEof));
    }

    #[test]
    fn decode_truncated() {
        // All bytes have continuation bit set — never terminates
        assert_eq!(
            decode_varint_slice(&[0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80]),
            Err(VarintError::TooLong)
        );
    }

    #[test]
    fn decode_iter_roundtrip() {
        let data = vec![0x80, 0x01, 0x7F, 0x00];
        let mut iter = data.into_iter();
        assert_eq!(decode_varint_iter(&mut iter).unwrap(), 128);
        assert_eq!(decode_varint_iter(&mut iter).unwrap(), 127);
        assert_eq!(decode_varint_iter(&mut iter).unwrap(), 0);
        assert_eq!(decode_varint_iter(&mut iter), Err(VarintError::UnexpectedEof));
    }
}
