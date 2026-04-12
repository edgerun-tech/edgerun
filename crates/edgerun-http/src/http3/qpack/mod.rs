//! QPACK header compression for HTTP/3 (RFC 9204)
//!
//! QPACK is designed for HTTP/3's QUIC transport which can deliver data out of order.
//! Unlike HPACK, QPACK uses separate encoder/decoder streams to prevent head-of-line blocking.
//!
//! This implementation uses the `qpack` crate for encoding/decoding.

pub mod decoder;
pub mod encoder;
pub mod huffman;

pub use decoder::QpackDecoder;
pub use encoder::QpackEncoder;

/// QPACK error types
#[derive(Debug, Clone)]
pub enum QpackError {
    /// QPACK encoding error
    Encode(String),
    /// QPACK decoding error
    Decode(String),
}

impl std::fmt::Display for QpackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QpackError::Encode(msg) => write!(f, "QPACK encode error: {}", msg),
            QpackError::Decode(msg) => write!(f, "QPACK decode error: {}", msg),
        }
    }
}

impl From<edgerun_qpack::EncoderError> for QpackError {
    fn from(e: edgerun_qpack::EncoderError) -> Self {
        QpackError::Encode(e.to_string())
    }
}

impl From<edgerun_qpack::DecoderError> for QpackError {
    fn from(e: edgerun_qpack::DecoderError) -> Self {
        QpackError::Decode(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qpack_roundtrip_static_only() {
        // Encode headers with only static table entries
        let headers = vec![
            (":method", "GET"),
            (":scheme", "https"),
            (":path", "/"),
        ];

        let mut encoder = QpackEncoder::new();
        let mut decoder = QpackDecoder::new();

        let encoded = encoder
            .encode(&headers)
            .expect("encode failed");

        let decoded = decoder
            .decode(&encoded)
            .expect("decode failed");

        assert_eq!(decoded.len(), 3);
        assert_eq!(decoded[0].0, ":method");
        assert_eq!(decoded[0].1, "GET");
        assert_eq!(decoded[1].0, ":scheme");
        assert_eq!(decoded[1].1, "https");
        assert_eq!(decoded[2].0, ":path");
        assert_eq!(decoded[2].1, "/");
    }

    #[test]
    fn test_qpack_with_dynamic_headers() {
        // Encode headers that aren't in the static table
        let headers = vec![
            (":method", "GET"),
            (":scheme", "https"),
            (":path", "/api/v1/users"),
            ("x-custom-header", "my-value"),
        ];

        let mut encoder = QpackEncoder::new();
        let mut decoder = QpackDecoder::new();

        let encoded = encoder
            .encode(&headers)
            .expect("encode failed");

        let decoded = decoder
            .decode(&encoded)
            .expect("decode failed");

        assert_eq!(decoded.len(), 4);
        assert_eq!(decoded[3].0, "x-custom-header");
        assert_eq!(decoded[3].1, "my-value");
    }
}
