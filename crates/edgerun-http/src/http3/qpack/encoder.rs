//! QPACK encoder (RFC 9204)

use super::huffman;
use super::static_table::{self, STATIC_TABLE};
use super::{QpackResult, DEFAULT_MAX_TABLE_CAPACITY};

/// Dynamic table entry
#[derive(Debug, Clone)]
struct TableEntry {
    name: String,
    value: String,
}

/// QPACK encoder
pub struct QpackEncoder {
    /// Dynamic table
    dynamic_table: Vec<TableEntry>,
    /// Maximum dynamic table capacity
    max_capacity: usize,
    /// Current dynamic table size
    current_size: usize,
    /// Insert count (total entries inserted)
    insert_count: u64,
    /// Known received count
    known_received_count: u64,
}

impl QpackEncoder {
    /// Create a new encoder
    pub fn new() -> Self {
        QpackEncoder {
            dynamic_table: Vec::new(),
            max_capacity: DEFAULT_MAX_TABLE_CAPACITY,
            current_size: 0,
            insert_count: 0,
            known_received_count: 0,
        }
    }

    /// Create encoder with custom capacity
    pub fn with_capacity(capacity: usize) -> Self {
        QpackEncoder {
            dynamic_table: Vec::new(),
            max_capacity: capacity,
            current_size: 0,
            insert_count: 0,
            known_received_count: 0,
        }
    }

    /// Encode a list of headers
    pub fn encode(&mut self, headers: &[(&str, &str)]) -> QpackResult<Vec<u8>> {
        let mut output = Vec::new();

        for &(name, value) in headers {
            output.extend(self.encode_header(name, value)?);
        }

        Ok(output)
    }

    /// Encode a single header field
    pub fn encode_header(&mut self, name: &str, value: &str) -> QpackResult<Vec<u8>> {
        // Try static table first
        if let Some(index) = static_table::find_by_name_value(name, value) {
            // Indexed Header Field with Static Name Reference
            return Ok(Self::encode_indexed(index, true));
        }

        // Try dynamic table
        if let Some((index, match_type)) = self.find_in_dynamic(name, value) {
            return match match_type {
                MatchType::NameAndValue => {
                    Ok(Self::encode_indexed(index + STATIC_TABLE.len(), false))
                }
                MatchType::NameOnly => {
                    let mut output = Vec::new();
                    Self::encode_literal_with_indexing_name_ref(
                        &mut output,
                        index + STATIC_TABLE.len(),
                        false,
                        value.as_bytes(),
                    );
                    self.add_to_dynamic(name.to_string(), value.to_string());
                    Ok(output)
                }
            };
        }

        // Literal with indexing (new name)
        let mut output = Vec::new();
        Self::encode_literal_with_indexing_new_name(&mut output, name.as_bytes(), value.as_bytes());
        self.add_to_dynamic(name.to_string(), value.to_string());
        Ok(output)
    }

    /// Encode indexed header field
    fn encode_indexed(index: usize, is_static: bool) -> Vec<u8> {
        let mut output = Vec::new();

        if is_static {
            // 1 1 S------  (6-bit prefix, type = 0b11)
            Self::encode_prefixed_varint(&mut output, index as u64, 6, 0xC0);
        } else {
            // 1 0 N------  (6-bit prefix, type = 0b10)
            Self::encode_prefixed_varint(&mut output, index as u64, 6, 0x80);
        }

        output
    }

    /// Encode a QPACK varint with a type prefix.
    ///
    /// Writes the value as a QPACK integer with `prefix_bits` data bits in the
    /// first byte, OR-ing `type_prefix` into the first byte's type bits.
    fn encode_prefixed_varint(
        output: &mut Vec<u8>,
        value: u64,
        prefix_bits: u8,
        type_prefix: u8,
    ) {
        let max_prefix = (1u64 << prefix_bits) - 1;
        if value < max_prefix {
            output.push(type_prefix | (value as u8));
        } else {
            output.push(type_prefix | max_prefix as u8);
            let mut remaining = value - max_prefix;
            while remaining >= 128 {
                output.push((remaining % 128 + 128) as u8);
                remaining /= 128;
            }
            output.push(remaining as u8);
        }
    }

    /// Encode literal header field with indexing (name reference)
    fn encode_literal_with_indexing_name_ref(
        output: &mut Vec<u8>,
        name_index: usize,
        is_static: bool,
        value: &[u8],
    ) {
        if is_static {
            // 0 1 T S-----  (4-bit prefix, type = 0b01)
            Self::encode_prefixed_varint(output, name_index as u64, 4, 0x40);
        } else {
            // 0 1 T N-----  (6-bit prefix, type = 0b01 with 6-bit dynamic)
            Self::encode_prefixed_varint(output, name_index as u64, 6, 0x20);
        }

        // Encode value as string (with optional Huffman)
        encode_string(output, value);
    }

    /// Encode literal header field with indexing (new name)
    fn encode_literal_with_indexing_new_name(output: &mut Vec<u8>, name: &[u8], value: &[u8]) {
        // Name: 5-bit prefix for length (type = 0b001)
        // With Huffman: H flag on the length byte
        encode_string_with_prefix(output, name, 5, 0x10);
        // Value: 7-bit prefix for length
        encode_string(output, value);
    }

    /// Add entry to dynamic table
    fn add_to_dynamic(&mut self, name: String, value: String) {
        let entry_size = name.len() + value.len() + 32;

        // Evict entries if needed
        while self.current_size + entry_size > self.max_capacity {
            if let Some(entry) = self.dynamic_table.pop() {
                self.current_size -= entry.name.len() + entry.value.len() + 32;
            } else {
                break;
            }
        }

        self.dynamic_table
            .insert(0, TableEntry { name, value });
        self.current_size += entry_size;
        self.insert_count += 1;
    }

    /// Find in dynamic table
    fn find_in_dynamic(&self, name: &str, value: &str) -> Option<(usize, MatchType)> {
        for (i, entry) in self.dynamic_table.iter().enumerate() {
            if entry.name.eq_ignore_ascii_case(name) && entry.value == value {
                return Some((i, MatchType::NameAndValue));
            }
        }

        for (i, entry) in self.dynamic_table.iter().enumerate() {
            if entry.name.eq_ignore_ascii_case(name) {
                return Some((i, MatchType::NameOnly));
            }
        }

        None
    }

    /// Get current insert count
    pub fn insert_count(&self) -> u64 {
        self.insert_count
    }

    /// Set known received count
    pub fn set_known_received_count(&mut self, count: u64) {
        self.known_received_count = count;
    }
}

/// Match type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MatchType {
    NameAndValue,
    NameOnly,
}

/// Encode a string with Huffman if it reduces size, otherwise as raw bytes.
///
/// QPACK string representation (RFC 9204 §5):
///   7-bit prefix integer for length, with H flag (bit 7 of first byte).
///   If H=1, the string is Huffman-encoded. If H=0, it's raw UTF-8.
fn encode_string(output: &mut Vec<u8>, data: &[u8]) {
    encode_string_with_prefix(output, data, 7, 0x00);
}

/// Encode a string with a configurable prefix length and type.
///
/// `prefix_bits`: number of data bits in the first byte for the length (e.g., 5 or 7).
/// `type_prefix`: the type bits to OR into the first byte (e.g., 0x10 for literal new name).
fn encode_string_with_prefix(output: &mut Vec<u8>, data: &[u8], prefix_bits: u8, type_prefix: u8) {
    let huffman_encoded = huffman::encode(data);
    let use_huffman = huffman_encoded.len() < data.len() && !data.is_empty();

    let (str_data, len) = if use_huffman {
        (&huffman_encoded[..], huffman_encoded.len())
    } else {
        (data, data.len())
    };

    let start_pos = output.len();

    // Encode length with the given prefix (same algorithm as super::encode_varint)
    super::encode_varint(len as u64, prefix_bits, output);

    // Set H flag on the first byte of the length encoding
    if use_huffman {
        output[start_pos] |= 0x80 | type_prefix;
    } else {
        output[start_pos] |= type_prefix;
    }

    output.extend_from_slice(str_data);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_static_header() {
        let mut encoder = QpackEncoder::new();
        let encoded = encoder.encode_header(":method", "GET").unwrap();

        // Should be indexed static reference
        assert!((encoded[0] & 0x80) != 0);
    }

    #[test]
    fn test_encode_literal_new_name() {
        let mut encoder = QpackEncoder::new();
        let encoded = encoder.encode_header("x-custom", "value").unwrap();

        // Should be literal with indexing (type bits 0001 in lower 4 bits of first byte)
        // With Huffman encoding, the H flag (bit 7) may also be set
        // Check the type bits (bit 4) are set, and bits 5-6 are clear (new name format)
        let first_byte = encoded[0];
        // Lower 5 bits: type (1 bit) + length prefix (4 bits for 5-bit prefix)
        // Type bit (bit 0 of the 5-bit field) should be 1 for "literal with indexing, new name"
        assert!((first_byte & 0x10) != 0, "type bit not set: {:02x}", first_byte);
    }

    #[test]
    fn test_encode_multiple_headers() {
        let mut encoder = QpackEncoder::new();
        let headers = vec![
            (":method", "GET"),
            (":scheme", "https"),
            (":path", "/"),
            (":authority", "example.com"),
        ];

        let encoded = encoder.encode(&headers).unwrap();
        assert!(!encoded.is_empty());
        assert!(encoded.len() > 4);
    }

    #[test]
    fn test_dynamic_table_eviction() {
        let mut encoder = QpackEncoder::with_capacity(100);

        for i in 0..10 {
            let _ = encoder
                .encode_header(&format!("h{}", i), &format!("v{}", i))
                .unwrap();
        }

        assert!(encoder.current_size <= encoder.max_capacity);
    }
}
