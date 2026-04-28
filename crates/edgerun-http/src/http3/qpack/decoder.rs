//! QPACK decoder with dynamic table support (RFC 9204).
//!
//! Uses the full `edgerun_qpack::decoder::Decoder` for dynamic table decoding.
//! When `max_capacity > 0`, the decoder maintains a dynamic table populated from
//! encoder stream instructions received via `on_encoder_stream()`.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use edgerun_encoding::buf::Cursor;
use edgerun_qpack::decoder::Decoder as QpackInnerDecoder;
use edgerun_qpack::dynamic::DynamicTable;
use edgerun_qpack::{decode_stateless, Decoded, DecoderError, HeaderField};

/// QPACK decoder with dynamic table support.
pub struct QpackDecoder {
    max_capacity: usize,
    /// Full dynamic table decoder (only used when max_capacity > 0)
    inner: Option<QpackInnerDecoder>,
}

impl QpackDecoder {
    /// Create a new decoder (stateless by default).
    pub fn new() -> Self {
        QpackDecoder {
            max_capacity: 0,
            inner: None,
        }
    }

    /// Create decoder with dynamic table capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let mut dec = QpackDecoder {
            max_capacity: 0,
            inner: None,
        };
        dec.set_max_capacity(capacity);
        dec
    }

    /// Set the maximum dynamic table capacity.
    pub fn set_max_capacity(&mut self, capacity: usize) {
        self.max_capacity = capacity;
        if capacity > 0 {
            let mut table = DynamicTable::new();
            table.set_max_size(capacity).ok();
            self.inner = Some(QpackInnerDecoder::from(table));
        } else {
            self.inner = None;
        }
    }

    /// Decode a QPACK header block into (name, value) pairs.
    ///
    /// If dynamic table is enabled (`max_capacity > 0`), uses the dynamic table
    /// for decoding. Otherwise falls back to stateless decoding.
    pub fn decode(&mut self, data: &[u8]) -> Result<Vec<(String, String)>, DecoderError> {
        if let Some(inner) = self.inner.as_ref().filter(|_| self.max_capacity > 0) {
            // Dynamic table decoding
            let mut cursor = Cursor::new(data);
            return inner.decode_header(&mut cursor).map(header_block_to_pairs);
        }

        // Stateless decoding
        let mut cursor = Cursor::new(data);
        decode_stateless(&mut cursor, 4096).map(header_block_to_pairs)
    }

    /// Process data received on the QPACK encoder stream.
    ///
    /// This parses encoder stream instructions (Set Max Table Capacity,
    /// Insert With Name Reference, Insert Without Name Reference, Duplicate)
    /// and updates the dynamic table accordingly.
    ///
    /// Call this when receiving data on the unidirectional stream with type 0x02.
    pub fn on_encoder_stream(&mut self, data: &[u8]) -> Result<usize, DecoderError> {
        if self.max_capacity == 0 || self.inner.is_none() {
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

fn header_block_to_pairs(decoded: Decoded) -> Vec<(String, String)> {
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
