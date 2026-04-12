//! QPACK decoder — thin wrapper around the `qpack` crate (RFC 9204).

use qpack::{HeaderField, decode_stateless, DecoderError};

/// QPACK decoder.
///
/// Wraps `qpack::decode_stateless` for compatibility with the existing API.
pub struct QpackDecoder {
    #[allow(dead_code)]
    max_capacity: usize,
}

impl QpackDecoder {
    /// Create a new decoder
    pub fn new() -> Self {
        QpackDecoder {
            max_capacity: 0, // Stateless — no dynamic table
        }
    }

    /// Create decoder with custom capacity
    pub fn with_capacity(capacity: usize) -> Self {
        QpackDecoder {
            max_capacity: capacity,
        }
    }

    /// Decode a QPACK header block into a list of (name, value) pairs.
    ///
    /// Uses stateless decoding (no dynamic table).
    pub fn decode(&mut self, data: &[u8]) -> Result<Vec<(String, String)>, DecoderError> {
        let mut cursor = std::io::Cursor::new(data);
        let decoded = decode_stateless(&mut cursor, 4096)?;

        Ok(decoded
            .fields
            .into_iter()
            .map(|f| {
                (
                    String::from_utf8_lossy(f.name.as_ref()).to_string(),
                    String::from_utf8_lossy(f.value.as_ref()).to_string(),
                )
            })
            .collect())
    }

    /// Set the maximum dynamic table capacity.
    /// Setting this to 0 disables dynamic table usage (stateless decoding).
    pub fn set_max_capacity(&mut self, capacity: usize) {
        self.max_capacity = capacity;
    }
}
