//! Shared varint implementations.
//!
//! Two distinct schemes are used in HTTP/3 and QUIC:
//!
//! 1. **QUIC varint** (RFC 9000 §16) — 2 prefix bits determine encoding length
//!    (1/2/4/8 bytes total). Used for stream IDs, offsets, frame lengths, etc.
//! 2. **QPACK integer** (RFC 9204 §5, derived from RFC 7541 §5.1) — configurable
//!    prefix bits with continuation bytes. Used for header table indices.

// ─── QUIC varint (RFC 9000 §16) ─────────────────────────────────────────────

/// Encode a value using QUIC varint encoding (RFC 9000 §16).
///
/// The 2 most-significant bits of the first byte encode the length:
/// - `00` = 1 byte total (6-bit value, max 63)
/// - `01` = 2 bytes total (14-bit value, max 16383)
/// - `10` = 4 bytes total (30-bit value, max 1073741823)
/// - `11` = 8 bytes total (62-bit value, max 2^62-1)
pub fn quic_encode_varint(value: u64, output: &mut Vec<u8>) {
    if value < 64 {
        output.push(value as u8);
    } else if value < 16384 {
        output.push(((value >> 8) as u8) | 0x40);
        output.push(value as u8);
    } else if value < 1073741824 {
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

/// Decode a QUIC varint (RFC 9000 §16).
///
/// Returns `(value, bytes_consumed)`.
pub fn quic_decode_varint(data: &[u8]) -> Result<(u64, usize), String> {
    if data.is_empty() {
        return Err("Empty varint".to_string());
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
        return Err("Incomplete varint".to_string());
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

// ─── QPACK integer (RFC 9204 §5 / RFC 7541 §5.1) ───────────────────────────

/// Encode an integer using the QPACK/HPACK generic varint scheme.
///
/// `prefix_bits` specifies how many low bits of the first byte are available
/// for the value. If the value fits in the prefix, it's stored directly.
/// Otherwise the prefix is set to all-1s and the remainder is encoded in
/// continuation bytes (MSB = more bytes follow, lower 7 bits = data).
pub fn qpack_encode_varint(value: u64, prefix_bits: u8, output: &mut Vec<u8>) {
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

/// Decode an integer using the QPACK/HPACK generic varint scheme.
///
/// Returns `(value, bytes_consumed)`.
pub fn qpack_decode_varint(data: &[u8], start: usize, prefix_bits: u8) -> Result<(u64, usize), String> {
    if start >= data.len() {
        return Err("Not enough data".to_string());
    }
    let max_prefix = (1u64 << prefix_bits) - 1;
    let mut value = (data[start] & ((1u8 << prefix_bits) - 1)) as u64;
    if value < max_prefix {
        return Ok((value, 1));
    }
    let mut pos = start + 1;
    let mut m = 0u32;
    while pos < data.len() {
        let byte = data[pos] as u64;
        value += (byte & 127) << m;
        m += 7;
        if byte & 128 == 0 {
            return Ok((value, pos - start + 1));
        }
        pos += 1;
    }
    Err("Incomplete varint".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

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
