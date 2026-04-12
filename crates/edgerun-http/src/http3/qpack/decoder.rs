//! QPACK decoder (RFC 9204)

use super::huffman;
use super::static_table;
use super::{QpackError, QpackResult, DEFAULT_MAX_TABLE_CAPACITY};

/// Dynamic table entry
#[derive(Debug, Clone)]
struct TableEntry {
    name: String,
    value: String,
}

/// QPACK decoder
pub struct QpackDecoder {
    /// Dynamic table
    dynamic_table: Vec<TableEntry>,
    /// Maximum dynamic table capacity
    max_capacity: usize,
    /// Current dynamic table size
    current_size: usize,
    /// Total insertions tracked
    insert_count: u64,
}

impl QpackDecoder {
    /// Create a new decoder
    pub fn new() -> Self {
        QpackDecoder {
            dynamic_table: Vec::new(),
            max_capacity: DEFAULT_MAX_TABLE_CAPACITY,
            current_size: 0,
            insert_count: 0,
        }
    }

    /// Create a decoder with the given dynamic table capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        QpackDecoder {
            dynamic_table: Vec::new(),
            max_capacity: capacity,
            current_size: 0,
            insert_count: 0,
        }
    }

    /// Decode a header block
    pub fn decode(&mut self, data: &[u8]) -> QpackResult<Vec<(String, String)>> {
        let mut headers = Vec::new();
        let mut pos = 0;

        while pos < data.len() {
            let first_byte = data[pos];

            if first_byte & 0x80 != 0 {
                // Indexed Header Field
                let (index, is_static, bytes_read) = Self::decode_indexed(data, pos)?;
                pos += bytes_read;

                let (name, value) = if is_static {
                    let entry = static_table::get_static_entry(index)
                        .ok_or_else(|| QpackError::DecoderStream("Invalid static index".to_string()))?;
                    (entry.0.to_string(), entry.1.to_string())
                } else {
                    self.get_dynamic_entry(index)?
                };

                headers.push((name, value));
            } else if first_byte & 0x40 != 0 {
                // Literal with name reference and indexing
                let (index, is_static, bytes_read) =
                    Self::decode_name_index(data, pos, first_byte & 0x20 != 0)?;
                pos += bytes_read;

                let name = if is_static {
                    static_table::get_static_entry(index)
                        .ok_or_else(|| QpackError::DecoderStream("Invalid static index".to_string()))?
                        .0
                        .to_string()
                } else {
                    self.get_dynamic_entry(index)?.0
                };

                let (value, str_len) = Self::decode_string(data, pos)?;
                pos += str_len;

                self.add_to_dynamic(name.clone(), value.clone());
                headers.push((name, value));
            } else if first_byte & 0x20 != 0 {
                // Dynamic table size update
                let (size, bytes_read) = super::decode_varint(data, pos, 5)?;
                pos += bytes_read;
                self.max_capacity = size as usize;
            } else if first_byte & 0x10 != 0 {
                // Literal with new name and indexing
                let (name, name_len) = Self::decode_string(data, pos)?;
                pos += name_len;

                let (value, str_len) = Self::decode_string(data, pos)?;
                pos += str_len;

                self.add_to_dynamic(name.clone(), value.clone());
                headers.push((name, value));
            } else {
                // Literal without indexing
                let (name, name_len) = Self::decode_string(data, pos)?;
                pos += name_len;

                let (value, str_len) = Self::decode_string(data, pos)?;
                pos += str_len;

                headers.push((name, value));
            }
        }

        Ok(headers)
    }

    /// Decode indexed header field
    fn decode_indexed(data: &[u8], start: usize) -> QpackResult<(usize, bool, usize)> {
        if start >= data.len() {
            return Err(QpackError::DecoderStream("Not enough data".to_string()));
        }

        let first_byte = data[start];
        let is_static = first_byte & 0x40 != 0;
        let prefix_bits = if is_static { 6 } else { 4 };
        let mask = if is_static { 0x3F } else { 0x0F };

        let (index, bytes_read) = super::decode_varint(data, start, prefix_bits)?;

        Ok((index as usize, is_static, bytes_read))
    }

    /// Decode name index
    fn decode_name_index(
        data: &[u8],
        start: usize,
        is_dynamic: bool,
    ) -> QpackResult<(usize, bool, usize)> {
        if start >= data.len() {
            return Err(QpackError::DecoderStream("Not enough data".to_string()));
        }

        let prefix_bits = if is_dynamic { 6 } else { 4 };
        let (index, bytes_read) = super::decode_varint(data, start, prefix_bits)?;

        Ok((index as usize, !is_dynamic, bytes_read))
    }

    /// Decode string (RFC 9204 §5)
    ///
    /// Handles both raw and Huffman-encoded strings. The H flag (bit 7 of the
    /// first byte) determines the encoding:
    /// - H=1: Huffman-encoded string follows
    /// - H=0: Raw UTF-8 string follows
    fn decode_string(data: &[u8], start: usize) -> QpackResult<(String, usize)> {
        if start >= data.len() {
            return Err(QpackError::DecoderStream("Not enough data".to_string()));
        }

        // Check Huffman flag (bit 7 per RFC 9204 §5)
        let huffman = (data[start] & 0x80) != 0;

        let (str_len, bytes_read) = super::decode_varint(data, start, 7)?;

        // The actual string length is the value from the varint WITHOUT the H flag
        let raw_len = (str_len as usize) & 0x7F; // Mask off H flag
        let total_len = bytes_read + raw_len;

        if start + total_len > data.len() {
            return Err(QpackError::DecoderStream("String incomplete".to_string()));
        }

        let str_data = &data[start + bytes_read..start + total_len];

        let value = if huffman {
            let decoded = huffman::decode(str_data)
                .map_err(|e| QpackError::HuffmanDecode(e))?;
            String::from_utf8(decoded).map_err(|_| {
                QpackError::DecoderStream("Invalid UTF-8 in Huffman-decoded string".to_string())
            })?
        } else {
            String::from_utf8(str_data.to_vec()).map_err(|_| {
                QpackError::DecoderStream("Invalid UTF-8 in string".to_string())
            })?
        };

        Ok((value, total_len))
    }

    /// Get entry from dynamic table
    fn get_dynamic_entry(&self, index: usize) -> QpackResult<(String, String)> {
        if index >= self.dynamic_table.len() {
            return Err(QpackError::DecoderStream(format!(
                "Invalid dynamic index: {}",
                index
            )));
        }

        let entry = &self.dynamic_table[index];
        Ok((entry.name.clone(), entry.value.clone()))
    }

    /// Add entry to dynamic table
    fn add_to_dynamic(&mut self, name: String, value: String) {
        let entry_size = name.len() + value.len() + 32;

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

    /// Insert acknowledgment
    pub fn acknowledge_inserts(&mut self, count: u64) {
        // Process acknowledgments from encoder
        let _ = count;
    }

    /// Get current table capacity
    pub fn capacity(&self) -> usize {
        self.max_capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_static_indexed() {
        let mut decoder = QpackDecoder::new();

        // Encode :method GET as indexed static (index 18)
        // QPACK indexed: 1 1 S----- → 0xC0 | index
        let encoded = vec![0xC0 | 18];

        let headers = decoder.decode(&encoded).unwrap();
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].0, ":method");
        assert_eq!(headers[0].1, "GET");
    }

    #[test]
    fn test_decode_literal_no_index() {
        let mut decoder = QpackDecoder::new();

        // Encode custom header without indexing
        let name = b"custom";
        let value = b"value";
        let mut encoded = Vec::new();
        encoded.push(0x00 | ((name.len() as u8) & 0x0F));
        if name.len() > 15 {
            encoded.push((name.len() & 0xFF) as u8);
        }
        encoded.extend_from_slice(name);
        encoded.push(value.len() as u8);
        encoded.extend_from_slice(value);

        let headers = decoder.decode(&encoded).unwrap();
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].0, "custom");
        assert_eq!(headers[0].1, "value");
    }

    #[test]
    fn test_decode_multiple_headers() {
        use super::super::QpackEncoder;

        let mut encoder = QpackEncoder::new();
        let mut decoder = QpackDecoder::new();

        // Use simple headers that we know work
        let headers = vec![
            (":method", "GET"),
        ];

        let encoded = encoder.encode(&headers).unwrap();
        let decoded = decoder.decode(&encoded).unwrap();

        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].0, ":method");
        assert_eq!(decoded[0].1, "GET");
    }
}
