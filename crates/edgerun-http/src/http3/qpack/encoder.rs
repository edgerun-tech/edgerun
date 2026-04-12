//! QPACK encoder — thin wrapper around the `qpack` crate (RFC 9204).

use qpack::{HeaderField, encode_stateless, EncoderError};

/// QPACK encoder.
///
/// Wraps `qpack::encode_stateless` for compatibility with the existing API.
/// Dynamic table is not used (stateless encoding only).
pub struct QpackEncoder {
    #[allow(dead_code)]
    max_capacity: usize,
}

impl QpackEncoder {
    /// Create a new encoder
    pub fn new() -> Self {
        QpackEncoder {
            max_capacity: 0, // Stateless — no dynamic table
        }
    }

    /// Create encoder with custom capacity
    pub fn with_capacity(capacity: usize) -> Self {
        QpackEncoder {
            max_capacity: capacity,
        }
    }

    /// Set the maximum dynamic table capacity.
    /// Setting this to 0 disables dynamic table usage (stateless encoding).
    pub fn set_max_capacity(&mut self, capacity: usize) {
        self.max_capacity = capacity;
    }

    /// Encode a list of headers into a QPACK header block.
    ///
    /// Uses stateless encoding (no dynamic table).
    pub fn encode(&mut self, headers: &[(&str, &str)]) -> Result<Vec<u8>, EncoderError> {
        let fields: Vec<HeaderField> = headers
            .iter()
            .map(|(n, v)| HeaderField::new(n.as_bytes(), v.as_bytes()))
            .collect();

        let mut output = Vec::new();
        encode_stateless(&mut output, fields)?;
        Ok(output)
    }

    /// Encode a single header field — not used in stateless mode,
    /// but kept for API compatibility.
    pub fn encode_header(&mut self, name: &str, value: &str) -> Result<Vec<u8>, EncoderError> {
        self.encode(&[(name, value)])
    }

    /// Get current insert count (always 0 for stateless encoding).
    pub fn insert_count(&self) -> u64 {
        0
    }

    /// Set known received count (no-op for stateless encoding).
    pub fn set_known_received_count(&mut self, _count: u64) {}
}
