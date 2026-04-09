//! QPACK encoder (RFC 9204)

use super::static_table::{self, STATIC_TABLE};
use super::{QpackError, QpackResult, DEFAULT_MAX_TABLE_CAPACITY};

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
            // 1 1 S------  (6-bit prefix for index)
            if index < 64 {
                output.push(0xC0 | (index as u8));
            } else {
                output.push(0xFF);
                let mut remaining = index - 63;
                while remaining >= 128 {
                    output.push((remaining % 128 + 128) as u8);
                    remaining /= 128;
                }
                output.push(remaining as u8);
            }
        } else {
            // 1 0 N----  (6-bit prefix for dynamic index)
            if index < 64 {
                output.push(0x80 | (index as u8));
            } else {
                output.push(0xBF);
                let mut remaining = index - 63;
                while remaining >= 128 {
                    output.push((remaining % 128 + 128) as u8);
                    remaining /= 128;
                }
                output.push(remaining as u8);
            }
        }

        output
    }

    /// Encode literal header field with indexing (name reference)
    fn encode_literal_with_indexing_name_ref(
        output: &mut Vec<u8>,
        name_index: usize,
        is_static: bool,
        value: &[u8],
    ) {
        let prefix_bits = if is_static { 4 } else { 6 };

        if is_static {
            // 0 1 T N----
            output.push(0x40 | ((name_index >> 8) & 0x0F) as u8);
            if name_index > 15 {
                output.push((name_index & 0xFF) as u8);
            }
        } else {
            // 0 1 T N-----
            output.push(0x20 | ((name_index >> 8) & 0x3F) as u8);
            if name_index > 63 {
                output.push((name_index & 0xFF) as u8);
            }
        }

        // Encode value as string
        output.push(value.len() as u8);
        output.extend_from_slice(value);
    }

    /// Encode literal header field with indexing (new name)
    fn encode_literal_with_indexing_new_name(output: &mut Vec<u8>, name: &[u8], value: &[u8]) {
        // 0 0 1 T N----
        output.push(0x10 | ((name.len() >> 8) & 0x0F) as u8);
        if name.len() > 15 {
            output.push((name.len() & 0xFF) as u8);
        }
        output.extend_from_slice(name);

        // Value
        output.push(value.len() as u8);
        output.extend_from_slice(value);
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

        // Should be literal with indexing (0001xxxx)
        assert!((encoded[0] & 0xF0) == 0x10);
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
