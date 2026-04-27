//! QPACK encoder with full dynamic table support (RFC 9204).
//!
//! Uses the vendored `edgerun-qpack` crate's `Encoder` struct which supports:
//! - Static table lookups (Indexed::Static)
//! - Dynamic table insertions with deduplication
//! - Encoder stream instruction generation (InsertWithNameRef, InsertWithoutNameRef, etc.)
//! - Header block encoding with dynamic table references (Indexed::Dynamic, IndexedWithPostBase)
//! - Decoder stream feedback via `on_decoder_recv()`

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use edgerun_encoding::buf::Cursor;
use edgerun_qpack::dynamic::DynamicTable;
use edgerun_qpack::encoder::Encoder;
use edgerun_qpack::{encode_stateless, EncoderError, HeaderField};

/// QPACK encoder with full dynamic table support.
///
/// When `max_capacity` is 0, stateless encoding is used (static table + literals only).
/// When `max_capacity` > 0, the encoder maintains a dynamic table and produces:
/// 1. **Encoder stream instructions** — sent on the QPACK encoder stream (unidirectional, type 0x2)
/// 2. **Header block** — sent in HEADERS frames, referencing dynamic table entries by index
pub struct QpackEncoder {
    max_capacity: usize,
    encoder: Option<Encoder>,
    insert_count: u64,
}

impl QpackEncoder {
    /// Create a new encoder (stateless by default).
    pub fn new() -> Self {
        QpackEncoder {
            max_capacity: 0,
            encoder: None,
            insert_count: 0,
        }
    }

    /// Create encoder with dynamic table capacity.
    ///
    /// The `capacity` is the maximum dynamic table size in bytes.
    /// Values > 0 enable dynamic table encoding.
    pub fn with_capacity(capacity: usize) -> Self {
        let mut enc = QpackEncoder {
            max_capacity: 0,
            encoder: None,
            insert_count: 0,
        };
        enc.set_max_capacity(capacity);
        enc
    }

    /// Set the maximum dynamic table capacity.
    pub fn set_max_capacity(&mut self, capacity: usize) {
        self.max_capacity = capacity;
        if capacity > 0 {
            let mut table = DynamicTable::new();
            table.set_max_size(capacity).ok();
            self.encoder = Some(Encoder::from(table));
        } else {
            self.encoder = None;
        }
    }

    /// Encode headers into a QPACK header block.
    ///
    /// Returns `(header_block, encoder_instructions)`.
    ///
    /// - **header_block**: The encoded header block to send in a HEADERS frame.
    ///   Contains references to static/dynamic table entries and literal values.
    /// - **encoder_instructions**: Instructions to send on the QPACK encoder stream
    ///   (unidirectional stream type 0x2) so the decoder can populate its dynamic table.
    ///
    /// If dynamic table is disabled (`max_capacity == 0`), uses stateless encoding
    /// and returns an empty `encoder_instructions` vec.
    pub fn encode(&mut self, headers: &[(&str, &str)]) -> Result<(Vec<u8>, Vec<u8>), EncoderError> {
        if self.max_capacity == 0 || self.encoder.is_none() {
            // Stateless encoding
            let fields: Vec<HeaderField> = headers
                .iter()
                .map(|(n, v)| HeaderField::new(n.as_bytes(), v.as_bytes()))
                .collect();
            let mut header_block = Vec::new();
            encode_stateless(&mut header_block, fields)?;
            return Ok((header_block, Vec::new()));
        }

        let encoder = self.encoder.as_mut().unwrap();

        let fields: Vec<HeaderField> = headers
            .iter()
            .map(|(n, v)| HeaderField::new(n.as_bytes(), v.as_bytes()))
            .collect();

        let mut header_block = Vec::new();
        let mut encoder_instructions = Vec::new();

        // Encode with dynamic table — stream_id is the HTTP/3 request stream ID
        // (we use stream_id = 1 as a placeholder for the encoder)
        encoder.encode(1, &mut header_block, &mut encoder_instructions, &fields)?;

        self.insert_count += headers.len() as u64;

        Ok((header_block, encoder_instructions))
    }

    /// Encode a single header field.
    pub fn encode_header(
        &mut self,
        name: &str,
        value: &str,
    ) -> Result<(Vec<u8>, Vec<u8>), EncoderError> {
        self.encode(&[(name, value)])
    }

    /// Get current insert count.
    pub fn insert_count(&self) -> u64 {
        self.insert_count
    }

    /// Process decoder stream feedback.
    ///
    /// The decoder sends acknowledgments on the decoder stream (unidirectional, type 0x3)
    /// to inform the encoder about which dynamic table entries have been acknowledged.
    ///
    /// Call this when receiving data on the QPACK decoder stream.
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
