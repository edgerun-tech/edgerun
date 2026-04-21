//! QUIC variable-length integer encoding (RFC 9000 §16).
//!
//! QUIC uses a variable-length integer encoding where the first 2 bits
//! indicate the length (1, 2, 4, or 8 bytes), and the remaining bits
//! contain the integer value in big-endian order.
//!
//! This is distinct from protobuf varint (LEB128) encoding.

use alloc::vec::Vec;

/// Encode a u64 as a QUIC variable-length integer.
///
/// The encoding uses the first 2 bits to indicate the length:
/// - `00` → 1 byte (values 0–63)
/// - `01` → 2 bytes (values 0–16383)
/// - `10` → 4 bytes (values 0–1073741823)
/// - `11` → 8 bytes (values 0–2^62-1)
///
/// # Examples
/// ```
/// use edgerun_encoding::quic_varint::{encode_varint, decode_varint};
/// let mut out = Vec::new();
/// encode_varint(42, &mut out);
/// assert_eq!(out, vec![42]);
/// let (val, len) = decode_varint(&out).unwrap();
/// assert_eq!((val, len), (42, 1));
/// ```
pub fn encode_varint(value: u64, output: &mut Vec<u8>) {
    if value < 64 {
        output.push(value as u8);
    } else if value < 16_384 {
        output.push(((value >> 8) as u8) | 0x40);
        output.push(value as u8);
    } else if value < 1_073_741_824 {
        let bytes = (value as u32).to_be_bytes();
        output.push(bytes[0] | 0x80);
        output.push(bytes[1]);
        output.push(bytes[2]);
        output.push(bytes[3]);
    } else {
        let bytes = value.to_be_bytes();
        output.push(bytes[0] | 0xC0);
        output.extend_from_slice(&bytes[1..]);
    }
}

/// Decode a QUIC variable-length integer from bytes.
///
/// Returns `(value, bytes_consumed)`.
///
/// Returns `Err` if the input is empty or truncated.
///
/// # Examples
/// ```
/// use edgerun_encoding::quic_varint::{encode_varint, decode_varint};
/// let mut out = Vec::new();
/// encode_varint(16383, &mut out);
/// let (val, len) = decode_varint(&out).unwrap();
/// assert_eq!((val, len), (16383, 2));
/// ```
pub fn decode_varint(data: &[u8]) -> Result<(u64, usize), VarintError> {
    if data.is_empty() {
        return Err(VarintError::Empty);
    }
    let first = data[0];
    let len = match first >> 6 {
        0 => 1,
        1 => 2,
        2 => 4,
        3 => 8,
        _ => unreachable!(),
    };
    if data.len() < len {
        return Err(VarintError::Incomplete(len, data.len()));
    }
    let value = match len {
        1 => (first & 0x3F) as u64,
        2 => u16::from_be_bytes([first & 0x3F, data[1]]) as u64,
        4 => {
            let b = [first & 0x3F, data[1], data[2], data[3]];
            u32::from_be_bytes(b) as u64
        }
        8 => {
            let mut b: [u8; 8] = data[..8].try_into().unwrap();
            b[0] &= 0x3F;
            u64::from_be_bytes(b)
        }
        _ => unreachable!(),
    };
    Ok((value, len))
}

/// QUIC varint decode error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VarintError {
    /// No input bytes.
    Empty,
    /// Truncated varint: expected N bytes but got M.
    Incomplete(usize, usize),
}

impl core::fmt::Display for VarintError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            VarintError::Empty => write!(f, "empty varint"),
            VarintError::Incomplete(expected, got) => {
                write!(f, "incomplete varint: need {expected} bytes, got {got}")
            }
        }
    }
}

/// Encode a u64 as QUIC varint, returning the encoded bytes.
pub fn encode_varint_vec(value: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(8);
    encode_varint(value, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    #[test]
    fn test_encode_decode_1byte() {
        let mut out = Vec::new();
        encode_varint(0, &mut out);
        assert_eq!(out, vec![0x00]);
        let (v, n) = decode_varint(&out).unwrap();
        assert_eq!((v, n), (0, 1));

        let mut out = Vec::new();
        encode_varint(63, &mut out);
        assert_eq!(out, vec![0x3F]);
        let (v, n) = decode_varint(&out).unwrap();
        assert_eq!((v, n), (63, 1));
    }

    #[test]
    fn test_encode_decode_2byte() {
        let mut out = Vec::new();
        encode_varint(64, &mut out);
        assert_eq!(out, vec![0x40, 0x40]);
        let (v, n) = decode_varint(&out).unwrap();
        assert_eq!((v, n), (64, 2));

        let mut out = Vec::new();
        encode_varint(16_383, &mut out);
        assert_eq!(out, vec![0x7F, 0xFF]);
        let (v, n) = decode_varint(&out).unwrap();
        assert_eq!((v, n), (16_383, 2));
    }

    #[test]
    fn test_encode_decode_4byte() {
        let mut out = Vec::new();
        encode_varint(16_384, &mut out);
        assert_eq!(out.len(), 4);
        let (v, n) = decode_varint(&out).unwrap();
        assert_eq!((v, n), (16_384, 4));

        let mut out = Vec::new();
        encode_varint(1_073_741_823, &mut out);
        assert_eq!(out.len(), 4);
        let (v, n) = decode_varint(&out).unwrap();
        assert_eq!((v, n), (1_073_741_823, 4));
    }

    #[test]
    fn test_encode_decode_8byte() {
        let mut out = Vec::new();
        encode_varint(1_073_741_824, &mut out);
        assert_eq!(out.len(), 8);
        let (v, n) = decode_varint(&out).unwrap();
        assert_eq!((v, n), (1_073_741_824, 8));

        let mut out = Vec::new();
        encode_varint(u64::MAX >> 2, &mut out);
        assert_eq!(out.len(), 8);
        let (v, n) = decode_varint(&out).unwrap();
        assert_eq!((v, n), (u64::MAX >> 2, 8));
    }

    #[test]
    fn test_empty() {
        assert_eq!(decode_varint(&[]), Err(VarintError::Empty));
    }

    #[test]
    fn test_incomplete() {
        assert_eq!(
            decode_varint(&[0x40]),
            Err(VarintError::Incomplete(2, 1))
        );
    }

    #[test]
    fn test_roundtrip_all_sizes() {
        for v in [0, 1, 63, 64, 16_383, 16_384, 1_073_741_823, 1_073_741_824, u64::MAX >> 2] {
            let encoded = encode_varint_vec(v);
            let (decoded, len) = decode_varint(&encoded).unwrap();
            assert_eq!(decoded, v);
            assert_eq!(len, encoded.len());
        }
    }
}
