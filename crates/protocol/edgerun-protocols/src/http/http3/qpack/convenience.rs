//! Convenience QPACK encoder/decoder API for HTTP integrations.

use super::buf::Cursor;
use super::decoder::Decoder as InnerDecoder;
use super::dynamic::DynamicTable;
use super::encoder::Encoder as InnerEncoder;
use super::{decode_stateless, encode_stateless, Decoded, DecoderError, EncoderError, HeaderField};
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// QPACK encoder with optional dynamic table support.
pub struct QpackEncoder {
    max_capacity: usize,
    encoder: Option<InnerEncoder>,
    insert_count: u64,
}

impl QpackEncoder {
    /// Create a stateless encoder.
    pub fn new() -> Self {
        QpackEncoder {
            max_capacity: 0,
            encoder: None,
            insert_count: 0,
        }
    }

    /// Create an encoder with dynamic table capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let mut encoder = Self::new();
        encoder.set_max_capacity(capacity);
        encoder
    }

    /// Set the maximum dynamic table capacity.
    pub fn set_max_capacity(&mut self, capacity: usize) {
        self.max_capacity = capacity;
        if capacity > 0 {
            let mut table = DynamicTable::new();
            table.set_max_size(capacity).ok();
            self.encoder = Some(InnerEncoder::from(table));
        } else {
            self.encoder = None;
        }
    }

    /// Encode headers into `(header_block, encoder_instructions)`.
    pub fn encode(&mut self, headers: &[(&str, &str)]) -> Result<(Vec<u8>, Vec<u8>), EncoderError> {
        let fields = header_refs_to_fields(headers);

        if let Some(encoder) = self.encoder.as_mut().filter(|_| self.max_capacity > 0) {
            let mut header_block = Vec::new();
            let mut encoder_instructions = Vec::new();

            encoder.encode(1, &mut header_block, &mut encoder_instructions, &fields)?;
            self.insert_count += headers.len() as u64;

            return Ok((header_block, encoder_instructions));
        }

        let mut header_block = Vec::new();
        encode_stateless(&mut header_block, fields)?;
        Ok((header_block, Vec::new()))
    }

    /// Encode one header field.
    pub fn encode_header(
        &mut self,
        name: &str,
        value: &str,
    ) -> Result<(Vec<u8>, Vec<u8>), EncoderError> {
        self.encode(&[(name, value)])
    }

    /// Current dynamic-table insert count tracked by this convenience encoder.
    pub fn insert_count(&self) -> u64 {
        self.insert_count
    }

    /// Process decoder stream feedback.
    pub fn on_decoder_recv(&mut self, data: &[u8]) -> Result<(), EncoderError> {
        if let Some(encoder) = &mut self.encoder {
            let mut cursor = Cursor::new(data);
            encoder.on_decoder_recv(&mut cursor)?;
        }
        Ok(())
    }

    /// Set known received count from decoder stream.
    pub fn set_known_received_count(&mut self, _count: u64) {}
}

impl Default for QpackEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// QPACK decoder with optional dynamic table support.
pub struct QpackDecoder {
    max_capacity: usize,
    inner: Option<InnerDecoder>,
}

impl QpackDecoder {
    /// Create a stateless decoder.
    pub fn new() -> Self {
        QpackDecoder {
            max_capacity: 0,
            inner: None,
        }
    }

    /// Create decoder with dynamic table capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let mut decoder = Self::new();
        decoder.set_max_capacity(capacity);
        decoder
    }

    /// Set the maximum dynamic table capacity.
    pub fn set_max_capacity(&mut self, capacity: usize) {
        self.max_capacity = capacity;
        if capacity > 0 {
            let mut table = DynamicTable::new();
            table.set_max_size(capacity).ok();
            self.inner = Some(InnerDecoder::from(table));
        } else {
            self.inner = None;
        }
    }

    /// Decode a QPACK header block into `(name, value)` pairs.
    pub fn decode(&mut self, data: &[u8]) -> Result<Vec<(String, String)>, DecoderError> {
        if let Some(inner) = self.inner.as_ref().filter(|_| self.max_capacity > 0) {
            let mut cursor = Cursor::new(data);
            return inner.decode_header(&mut cursor).map(decoded_to_pairs);
        }

        let mut cursor = Cursor::new(data);
        decode_stateless(&mut cursor, 4096).map(decoded_to_pairs)
    }

    /// Process data received on the QPACK encoder stream.
    pub fn on_encoder_stream(&mut self, data: &[u8]) -> Result<usize, DecoderError> {
        if self.max_capacity == 0 {
            return Ok(0);
        }

        let Some(inner) = self.inner.as_mut() else {
            return Ok(0);
        };

        let mut cursor = Cursor::new(data);
        let mut output = Vec::new();
        inner.on_encoder_recv(&mut cursor, &mut output)
    }
}

impl Default for QpackDecoder {
    fn default() -> Self {
        Self::new()
    }
}

/// QPACK convenience error type.
#[derive(Debug, Clone)]
pub enum QpackError {
    /// QPACK encoding error.
    Encode(String),
    /// QPACK decoding error.
    Decode(String),
}

impl core::fmt::Display for QpackError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            QpackError::Encode(msg) => write!(f, "QPACK encode error: {msg}"),
            QpackError::Decode(msg) => write!(f, "QPACK decode error: {msg}"),
        }
    }
}

impl From<EncoderError> for QpackError {
    fn from(error: EncoderError) -> Self {
        QpackError::Encode(error.to_string())
    }
}

impl From<DecoderError> for QpackError {
    fn from(error: DecoderError) -> Self {
        QpackError::Decode(error.to_string())
    }
}

fn header_refs_to_fields(headers: &[(&str, &str)]) -> Vec<HeaderField> {
    headers
        .iter()
        .map(|(name, value)| HeaderField::new(name.as_bytes(), value.as_bytes()))
        .collect()
}

fn decoded_to_pairs(decoded: Decoded) -> Vec<(String, String)> {
    decoded
        .fields
        .into_iter()
        .map(header_field_to_pair)
        .collect()
}

fn header_field_to_pair(field: HeaderField) -> (String, String) {
    (
        String::from_utf8_lossy(field.name.as_ref()).to_string(),
        String::from_utf8_lossy(field.value.as_ref()).to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn qpack_roundtrip_static_only() {
        let headers = vec![(":method", "GET"), (":scheme", "https"), (":path", "/")];

        let mut encoder = QpackEncoder::new();
        let mut decoder = QpackDecoder::new();
        let (encoded, instructions) = encoder.encode(&headers).expect("encode failed");
        let decoded = decoder.decode(&encoded).expect("decode failed");

        assert!(instructions.is_empty());
        assert_eq!(decoded.len(), 3);
        assert_eq!(decoded[0].0, ":method");
        assert_eq!(decoded[0].1, "GET");
        assert_eq!(decoded[1].0, ":scheme");
        assert_eq!(decoded[1].1, "https");
        assert_eq!(decoded[2].0, ":path");
        assert_eq!(decoded[2].1, "/");
    }

    #[test]
    fn qpack_roundtrip_with_custom_header() {
        let headers = vec![
            (":method", "GET"),
            (":scheme", "https"),
            (":path", "/api/v1/users"),
            ("x-custom-header", "my-value"),
        ];

        let mut encoder = QpackEncoder::new();
        let mut decoder = QpackDecoder::new();
        let (encoded, _) = encoder.encode(&headers).expect("encode failed");
        let decoded = decoder.decode(&encoded).expect("decode failed");

        assert_eq!(decoded.len(), 4);
        assert_eq!(decoded[3].0, "x-custom-header");
        assert_eq!(decoded[3].1, "my-value");
    }
}
