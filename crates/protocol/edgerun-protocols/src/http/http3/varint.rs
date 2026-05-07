//! Shared varint implementations.
//!
//! Two distinct schemes are used in HTTP/3 and QUIC:
//!
//! 1. **QUIC varint** (RFC 9000 §16) — 2 prefix bits determine encoding length
//!    (1/2/4/8 bytes total). Used for stream IDs, offsets, frame lengths, etc.
//! 2. **QPACK integer** (RFC 9204 §5, derived from RFC 7541 §5.1) — configurable
//!    prefix bits with continuation bytes. Used for header table indices.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use edgerun_encoding::prefix_varint::{decode_prefix_varint, encode_prefix_varint};

// ─── QUIC varint (RFC 9000 §16) ─────────────────────────────────────────────

/// Encode a value using QUIC varint encoding (RFC 9000 §16).
///
/// The 2 most-significant bits of the first byte encode the length:
/// - `00` = 1 byte total (6-bit value, max 63)
/// - `01` = 2 bytes total (14-bit value, max 16383)
/// - `10` = 4 bytes total (30-bit value, max 1073741823)
/// - `11` = 8 bytes total (62-bit value, max 2^62-1)
pub fn quic_encode_varint(value: u64, output: &mut Vec<u8>) {
    edgerun_encoding::quic_varint::encode_varint(value, output)
}

/// Decode a QUIC varint (RFC 9000 §16).
///
/// Returns `(value, bytes_consumed)`.
pub fn quic_decode_varint(data: &[u8]) -> Result<(u64, usize), String> {
    edgerun_encoding::quic_varint::decode_varint(data).map_err(|e| e.to_string())
}

/// Decode a QUIC varint starting at `pos`.
pub fn quic_decode_varint_at(data: &[u8], pos: usize) -> Result<(u64, usize), String> {
    if pos >= data.len() {
        return Err("Out of bounds".to_string());
    }
    quic_decode_varint(&data[pos..])
}

// ─── QPACK integer (RFC 9204 §5 / RFC 7541 §5.1) ───────────────────────────

/// Encode an integer using the QPACK/HPACK generic varint scheme.
///
/// `prefix_bits` specifies how many low bits of the first byte are available
/// for the value. If the value fits in the prefix, it's stored directly.
/// Otherwise the prefix is set to all-1s and the remainder is encoded in
/// continuation bytes (MSB = more bytes follow, lower 7 bits = data).
pub fn qpack_encode_varint(value: u64, prefix_bits: u8, output: &mut Vec<u8>) {
    encode_prefix_varint(value, prefix_bits, output)
}

/// Decode an integer using the QPACK/HPACK generic varint scheme.
///
/// Returns `(value, bytes_consumed)`.
pub fn qpack_decode_varint(
    data: &[u8],
    start: usize,
    prefix_bits: u8,
) -> Result<(u64, usize), String> {
    decode_prefix_varint(data, start, prefix_bits).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_quic_varint_1byte() {
        let mut out = Vec::new();
        quic_encode_varint(0, &mut out);
        assert_eq!(out, vec![0x00]);
        out.clear();
        quic_encode_varint(63, &mut out);
        assert_eq!(out, vec![0x3F]);
    }

    #[test]
    fn test_quic_varint_2byte() {
        let mut out = Vec::new();
        quic_encode_varint(64, &mut out);
        assert_eq!(out, vec![0x40, 0x40]);
        out.clear();
        quic_encode_varint(16383, &mut out);
        assert_eq!(out, vec![0x7F, 0xFF]);
    }

    #[test]
    fn test_quic_varint_4byte() {
        let mut out = Vec::new();
        quic_encode_varint(16384, &mut out);
        assert_eq!(out.len(), 4, "4-byte varint must be exactly 4 bytes");
        let (val, consumed) = quic_decode_varint(&out).unwrap();
        assert_eq!(val, 16384);
        assert_eq!(consumed, 4);
    }

    #[test]
    fn test_quic_varint_8byte() {
        let mut out = Vec::new();
        quic_encode_varint(1073741824, &mut out);
        assert_eq!(out.len(), 8, "8-byte varint must be exactly 8 bytes");
        let (val, consumed) = quic_decode_varint(&out).unwrap();
        assert_eq!(val, 1073741824);
        assert_eq!(consumed, 8);
    }

    #[test]
    fn test_quic_varint_roundtrip_max() {
        let mut out = Vec::new();
        // Max value representable in a QUIC varint is 2^62 - 1 (62 data bits)
        let max_val = (1u64 << 62) - 1;
        quic_encode_varint(max_val, &mut out);
        assert_eq!(out.len(), 8);
        let (val, consumed) = quic_decode_varint(&out).unwrap();
        assert_eq!(val, max_val);
        assert_eq!(consumed, 8);
    }

    #[test]
    fn test_quic_decode_too_short() {
        assert!(quic_decode_varint(&[]).is_err());
        assert!(quic_decode_varint(&[0x40]).is_err()); // 2-byte prefix, only 1 byte
    }

    #[test]
    fn test_quic_decode_at_offset() {
        let data = [0xff, 0x40, 0x40];
        assert_eq!(quic_decode_varint_at(&data, 1).unwrap(), (64, 2));
        assert!(quic_decode_varint_at(&data, data.len()).is_err());
    }

    #[test]
    fn test_qpack_varint_small() {
        let mut out = Vec::new();
        qpack_encode_varint(42, 7, &mut out);
        assert_eq!(out, vec![42]);
        let (val, consumed) = qpack_decode_varint(&out, 0, 7).unwrap();
        assert_eq!(val, 42);
        assert_eq!(consumed, 1);
    }

    #[test]
    fn test_qpack_varint_multi_byte() {
        let mut out = Vec::new();
        qpack_encode_varint(1337, 5, &mut out);
        assert!(out.len() > 1);
        let (val, consumed) = qpack_decode_varint(&out, 0, 5).unwrap();
        assert_eq!(val, 1337);
        assert_eq!(consumed, out.len());
    }
}
