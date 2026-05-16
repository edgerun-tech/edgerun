//! HPACK/QPACK-style prefix integer encoding.
//!
//! This is the variable-length integer scheme from RFC 7541 section 5.1,
//! reused by QPACK. It is distinct from QUIC variable-length integers.

use alloc::vec::Vec;
use core::fmt;

/// Error type for HPACK/QPACK prefix integer decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixVarintError {
    /// The prefix width must be in the range 1..=8.
    InvalidPrefixBits,
    /// The start offset is outside the input.
    OutOfBounds,
    /// The input ended before the integer was complete.
    Incomplete,
}

impl fmt::Display for PrefixVarintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrefixVarintError::InvalidPrefixBits => f.write_str("invalid prefix bit count"),
            PrefixVarintError::OutOfBounds => f.write_str("varint start offset out of bounds"),
            PrefixVarintError::Incomplete => f.write_str("incomplete varint"),
        }
    }
}

/// Encode an integer using the HPACK/QPACK prefix varint scheme.
pub fn encode_prefix_varint(value: u64, prefix_bits: u8, output: &mut Vec<u8>) {
    assert!(
        (1..=8).contains(&prefix_bits),
        "invalid prefix bit count: {prefix_bits}"
    );
    let max_prefix = (1u64 << prefix_bits) - 1;
    if value < max_prefix {
        output.push(value as u8);
    } else {
        output.push(max_prefix as u8);
        let mut remaining = value - max_prefix;
        while remaining >= 128 {
            output.push((remaining % 128 + 128) as u8);
            remaining /= 128;
        }
        output.push(remaining as u8);
    }
}

/// Decode an integer using the HPACK/QPACK prefix varint scheme.
///
/// Returns `(value, bytes_consumed)`, where `bytes_consumed` is relative to
/// `start`.
pub fn decode_prefix_varint(
    data: &[u8],
    start: usize,
    prefix_bits: u8,
) -> Result<(u64, usize), PrefixVarintError> {
    if !(1..=8).contains(&prefix_bits) {
        return Err(PrefixVarintError::InvalidPrefixBits);
    }
    if start >= data.len() {
        return Err(PrefixVarintError::OutOfBounds);
    }

    let max_prefix = (1u64 << prefix_bits) - 1;
    let mask = if prefix_bits == 8 {
        u8::MAX
    } else {
        (1u8 << prefix_bits) - 1
    };
    let mut value = (data[start] & mask) as u64;
    if value < max_prefix {
        return Ok((value, 1));
    }

    let mut pos = start + 1;
    let mut shift = 0u32;
    while pos < data.len() {
        let byte = data[pos] as u64;
        value += (byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok((value, pos - start + 1));
        }
        shift += 7;
        pos += 1;
    }

    Err(PrefixVarintError::Incomplete)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn encodes_small_value_in_prefix() {
        let mut out = Vec::new();
        encode_prefix_varint(42, 7, &mut out);
        assert_eq!(out, vec![42]);
        assert_eq!(decode_prefix_varint(&out, 0, 7).unwrap(), (42, 1));
    }

    #[test]
    fn encodes_multi_byte_value() {
        let mut out = Vec::new();
        encode_prefix_varint(1337, 5, &mut out);
        assert_eq!(decode_prefix_varint(&out, 0, 5).unwrap(), (1337, out.len()));
    }

    #[test]
    fn rejects_invalid_decode_inputs() {
        assert_eq!(
            decode_prefix_varint(&[], 0, 7),
            Err(PrefixVarintError::OutOfBounds)
        );
        assert_eq!(
            decode_prefix_varint(&[0x7f, 0x80], 0, 7),
            Err(PrefixVarintError::Incomplete)
        );
        assert_eq!(
            decode_prefix_varint(&[0], 0, 0),
            Err(PrefixVarintError::InvalidPrefixBits)
        );
    }
}
