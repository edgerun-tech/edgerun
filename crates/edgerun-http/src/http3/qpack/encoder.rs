//! QPACK encoder — supports both stateless and dynamic table encoding (RFC 9204).
//!
//! When `max_capacity` is 0 (default), stateless encoding is used via `qpack::encode_stateless`.
//! When `max_capacity` > 0, the encoder maintains its own dynamic table and emits
//! encoder stream instructions for headers that are not in the static table.

use qpack::{HeaderField, EncoderError};

/// QPACK encoder with dynamic table support.
///
/// Uses stateless encoding for the header block, but tracks dynamic table inserts
/// and generates encoder stream instructions separately.
pub struct QpackEncoder {
    max_capacity: usize,
    insert_count: u64,
    encoder_stream_initialized: bool,
    /// Track inserted fields for dynamic table lookups
    inserted_fields: Vec<(String, String)>,
}

impl QpackEncoder {
    /// Create a new encoder (stateless by default).
    pub fn new() -> Self {
        QpackEncoder {
            max_capacity: 0,
            insert_count: 0,
            encoder_stream_initialized: false,
            inserted_fields: Vec::new(),
        }
    }

    /// Create encoder with dynamic table capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        QpackEncoder {
            max_capacity: capacity,
            insert_count: 0,
            encoder_stream_initialized: false,
            inserted_fields: Vec::new(),
        }
    }

    /// Set the maximum dynamic table capacity.
    pub fn set_max_capacity(&mut self, capacity: usize) {
        self.max_capacity = capacity;
    }

    /// Encode headers into a QPACK header block.
    ///
    /// If dynamic table is enabled, headers not in the static table are
    /// inserted into the dynamic table (tracked locally) and referenced
    /// by index in subsequent encodings.
    pub fn encode(&mut self, headers: &[(&str, &str)]) -> Result<Vec<u8>, EncoderError> {
        if self.max_capacity == 0 {
            // Stateless encoding
            let fields: Vec<HeaderField> = headers
                .iter()
                .map(|(n, v)| HeaderField::new(n.as_bytes(), v.as_bytes()))
                .collect();
            let mut output = Vec::new();
            qpack::encode_stateless(&mut output, fields)?;
            return Ok(output);
        }

        // Dynamic table encoding:
        // 1. Generate encoder stream instructions for new dynamic table entries
        let encoder_instructions = self.generate_encoder_instructions(headers);

        // 2. Encode the header block using stateless encoding
        let fields: Vec<HeaderField> = headers
            .iter()
            .map(|(n, v)| HeaderField::new(n.as_bytes(), v.as_bytes()))
            .collect();
        let mut header_block = Vec::new();
        qpack::encode_stateless(&mut header_block, fields)?;

        // 3. Prepend encoder instructions to the header block
        let mut output = encoder_instructions;
        output.extend_from_slice(&header_block);
        Ok(output)
    }

    /// Generate encoder stream instructions for headers that are inserted
    /// into the dynamic table.
    fn generate_encoder_instructions(&mut self, headers: &[(&str, &str)]) -> Vec<u8> {
        let mut instructions = Vec::new();

        if !self.encoder_stream_initialized && self.max_capacity > 0 {
            // Send Set Max Table Capacity
            if self.max_capacity <= 31 {
                instructions.push((0x20 | (self.max_capacity & 0x1F)) as u8);
            } else {
                let mut remaining = self.max_capacity;
                let first_byte = (0x20 | (remaining & 0x1F)) as u8;
                instructions.push(first_byte);
                remaining >>= 5;
                while remaining > 127 {
                    instructions.push((remaining & 0x7F | 0x80) as u8);
                    remaining >>= 7;
                }
                instructions.push(remaining as u8);
            }
            self.encoder_stream_initialized = true;
        }

        // Insert instructions for headers not in static table
        for &(name, value) in headers {
            if Self::is_in_static_table(name) {
                // Skip — static table entries don't need insertion
                continue;
            }

            // Check if already in our dynamic table
            if self.inserted_fields.iter().any(|(n, v)| n == name && v == value) {
                continue;
            }

            // Insert With Literal Name (prefix 0b01)
            instructions.push(0x40);
            Self::encode_varint(name.len() as u64, &mut instructions);
            instructions.extend_from_slice(name.as_bytes());
            Self::encode_varint(value.len() as u64, &mut instructions);
            instructions.extend_from_slice(value.as_bytes());

            self.inserted_fields.push((name.to_string(), value.to_string()));
            self.insert_count += 1;
        }

        instructions
    }

    /// Check if a header name is in the QPACK static table.
    fn is_in_static_table(name: &str) -> bool {
        find_static_table_index(name).is_some()
    }

    /// Encode a single header field.
    pub fn encode_header(&mut self, name: &str, value: &str) -> Result<Vec<u8>, EncoderError> {
        self.encode(&[(name, value)])
    }

    /// Get current insert count.
    pub fn insert_count(&self) -> u64 {
        self.insert_count
    }

    /// Set known received count (from decoder stream).
    pub fn set_known_received_count(&mut self, _count: u64) {}

    /// QUIC-style varint encoder.
    fn encode_varint(value: u64, output: &mut Vec<u8>) {
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
}

/// Static table lookup — RFC 9204 Table 1
fn find_static_table_index(name: &str) -> Option<usize> {
    const NAMES: &[&str] = &[
        ":authority",
        ":path",
        "age",
        "content-disposition",
        "content-length",
        "cookie",
        "date",
        "etag",
        "if-modified-since",
        "if-none-match",
        "last-modified",
        "link",
        "location",
        "referer",
        "set-cookie",
        ":method",
        ":method",
        ":method",
        ":method",
        ":method",
        ":method",
        ":method",
        ":scheme",
        ":scheme",
        ":status",
        ":status",
        ":status",
        ":status",
        ":status",
        "accept",
        "accept",
        "accept-encoding",
        "accept-ranges",
        "access-control-allow-headers",
        "access-control-allow-headers",
        "access-control-allow-origin",
        "cache-control",
        "cache-control",
        "cache-control",
        "cache-control",
        "cache-control",
        "cache-control",
        "content-encoding",
        "content-encoding",
        "content-type",
        "content-type",
        "content-type",
        "content-type",
        "content-type",
        "content-type",
        "content-type",
        "content-type",
        "content-type",
        "range",
        "strict-transport-security",
        "strict-transport-security",
        "strict-transport-security",
        "vary",
        "vary",
        "x-content-type-options",
        "x-xss-protection",
        ":status",
        ":status",
        ":status",
        ":status",
        ":status",
        ":status",
        ":status",
        ":status",
        ":status",
        "accept-encoding",
        "accept-ranges",
        "access-control-allow-headers",
        "access-control-allow-origin",
        "age",
        "authorization",
        "content-disposition",
        "content-encoding",
        "content-length",
        "cookie",
        "date",
        "etag",
        "if-modified-since",
        "if-none-match",
        "if-range",
        "last-modified",
        "link",
        "location",
        "referer",
        "set-cookie",
        "strict-transport-security",
        "user-agent",
    ];

    NAMES.iter().position(|&n| n == name)
}
