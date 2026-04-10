//! QPACK header compression for HTTP/3 (RFC 9204)
//!
//! QPACK is designed for HTTP/3's QUIC transport which can deliver data out of order.
//! Unlike HPACK, QPACK uses separate encoder/decoder streams to prevent head-of-line blocking.

pub mod decoder;
pub mod encoder;
pub mod instructions;
pub mod static_table;

pub use decoder::QpackDecoder;
pub use encoder::QpackEncoder;

/// QPACK static table size
pub const STATIC_TABLE_SIZE: usize = 99;

/// Default maximum dynamic table capacity
pub const DEFAULT_MAX_TABLE_CAPACITY: usize = 4096;

/// Default maximum blocked streams
pub const DEFAULT_MAX_BLOCKED_STREAMS: usize = 100;

/// QPACK error types
#[derive(Debug, Clone)]
pub enum QpackError {
    /// Encoder stream error
    EncoderStream(String),
    /// Decoder stream error
    DecoderStream(String),
    /// Dynamic table error
    DynamicTable(String),
    /// Integer overflow
    IntegerOverflow,
    /// Huffman decoding error
    HuffmanDecode(String),
}

impl std::fmt::Display for QpackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QpackError::EncoderStream(msg) => write!(f, "Encoder stream: {}", msg),
            QpackError::DecoderStream(msg) => write!(f, "Decoder stream: {}", msg),
            QpackError::DynamicTable(msg) => write!(f, "Dynamic table: {}", msg),
            QpackError::IntegerOverflow => write!(f, "Integer overflow"),
            QpackError::HuffmanDecode(msg) => write!(f, "Huffman decode: {}", msg),
        }
    }
}

/// QPACK result type
pub type QpackResult<T> = std::result::Result<T, QpackError>;

/// Encode a variable-length integer (RFC 9000 Section 16)
pub fn encode_varint(value: u64, prefix_bits: u8, output: &mut Vec<u8>) {
    let max_prefix = (1u64 << prefix_bits) - 1;

    if value < max_prefix {
        // Value fits in prefix - just output the value
        output.push(value as u8);
    } else {
        // Value doesn't fit, use multi-byte encoding
        output.push(max_prefix as u8);
        let mut remaining = value - max_prefix;

        while remaining >= 128 {
            output.push((remaining % 128 + 128) as u8);
            remaining /= 128;
        }
        output.push(remaining as u8);
    }
}

/// Decode a variable-length integer
pub fn decode_varint(data: &[u8], start: usize, prefix_bits: u8) -> QpackResult<(u64, usize)> {
    if start >= data.len() {
        return Err(QpackError::EncoderStream("Not enough data".to_string()));
    }

    let max_prefix = (1u64 << prefix_bits) - 1;
    let mut value = (data[start] & ((1u8 << prefix_bits) - 1)) as u64;

    if value < max_prefix {
        return Ok((value, 1));
    }

    let mut pos = start + 1;
    let mut m = 0;

    while pos < data.len() {
        let byte = data[pos] as u64;
        value += (byte & 127) << m;
        m += 7;

        if byte & 128 == 0 {
            return Ok((value, pos - start + 1));
        }

        pos += 1;
    }

    Err(QpackError::EncoderStream("Incomplete varint".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_varint_small_value() {
        let mut output = Vec::new();
        encode_varint(10, 7, &mut output);
        assert_eq!(output, vec![10]);

        let (value, _) = decode_varint(&output, 0, 7).unwrap();
        assert_eq!(value, 10);
    }

    #[test]
    fn test_varint_large_value() {
        let mut output = Vec::new();
        encode_varint(1337, 7, &mut output);

        let (value, _) = decode_varint(&output, 0, 7).unwrap();
        assert_eq!(value, 1337);
    }

    #[test]
    fn test_varint_max_prefix() {
        // 6-bit prefix, max = 63, values 0-62 fit
        let mut output = Vec::new();
        encode_varint(62, 6, &mut output);
        assert_eq!(output.len(), 1);

        let (value, _) = decode_varint(&output, 0, 6).unwrap();
        assert_eq!(value, 62);

        // 63 requires multi-byte
        let mut output = Vec::new();
        encode_varint(63, 6, &mut output);
        assert!(output.len() > 1);

        let (value, _) = decode_varint(&output, 0, 6).unwrap();
        assert_eq!(value, 63);
    }
}
