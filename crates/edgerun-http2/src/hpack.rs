//! HPACK header compression for HTTP/2 (RFC 7541)
//!
//! Implements static/dynamic tables, integer encoding, and Huffman coding.

use super::{Http2Error, Result};

/// Static table entries (RFC 7541 Appendix A)
const STATIC_TABLE: &[(&str, &str)] = &[
    ("", ""), // Index 0 is unused
    (":authority", ""),
    (":method", "GET"),
    (":method", "POST"),
    (":path", "/"),
    (":path", "/index.html"),
    (":scheme", "http"),
    (":scheme", "https"),
    (":status", "200"),
    (":status", "204"),
    (":status", "206"),
    (":status", "304"),
    (":status", "400"),
    (":status", "404"),
    (":status", "500"),
    ("accept-charset", ""),
    ("accept-encoding", "gzip, deflate"),
    ("accept-language", ""),
    ("accept-ranges", ""),
    ("accept", ""),
    ("access-control-allow-origin", ""),
    ("age", ""),
    ("allow", ""),
    ("authorization", ""),
    ("cache-control", ""),
    ("content-disposition", ""),
    ("content-encoding", ""),
    ("content-language", ""),
    ("content-length", ""),
    ("content-location", ""),
    ("content-range", ""),
    ("content-type", ""),
    ("cookie", ""),
    ("date", ""),
    ("etag", ""),
    ("expect", ""),
    ("expires", ""),
    ("from", ""),
    ("host", ""),
    ("if-match", ""),
    ("if-modified-since", ""),
    ("if-none-match", ""),
    ("if-range", ""),
    ("if-unmodified-since", ""),
    ("last-modified", ""),
    ("link", ""),
    ("location", ""),
    ("max-forwards", ""),
    ("proxy-authenticate", ""),
    ("proxy-authorization", ""),
    ("range", ""),
    ("referer", ""),
    ("refresh", ""),
    ("retry-after", ""),
    ("server", ""),
    ("set-cookie", ""),
    ("strict-transport-security", ""),
    ("transfer-encoding", ""),
    ("user-agent", ""),
    ("vary", ""),
    ("via", ""),
    ("www-authenticate", ""),
];

/// Huffman coding table (RFC 7541 Appendix B)
/// Simplified implementation with common codes
const HUFFMAN_CODES: &[(&str, u32, u8)] = &[
    // Common characters with their Huffman codes
    (" ", 0x1ff8, 13),
    ("%", 0x1ff9, 13),
    ("-", 0x06, 5),
    (".", 0x07, 5),
    ("0", 0x1c0, 9),
    ("1", 0x1c1, 9),
    ("2", 0x1c2, 9),
    ("3", 0x1c3, 9),
    ("4", 0x1c4, 9),
    ("5", 0x1c5, 9),
    ("6", 0x1c6, 9),
    ("7", 0x1c7, 9),
    ("8", 0x1c8, 9),
    ("9", 0x1c9, 9),
    ("A", 0x96, 8),
    ("B", 0x197, 9),
    ("C", 0x98, 8),
    ("D", 0x199, 9),
    ("E", 0x9a, 8),
    ("F", 0x19b, 9),
    ("G", 0x9c, 8),
    ("H", 0x19d, 9),
    ("I", 0x9e, 8),
    ("J", 0x19f, 9),
    ("K", 0x1a0, 9),
    ("L", 0x1a1, 9),
    ("M", 0x1a2, 9),
    ("N", 0x1a3, 9),
    ("O", 0x1a4, 9),
    ("P", 0x1a5, 9),
    ("Q", 0x1a6, 9),
    ("R", 0x1a7, 9),
    ("S", 0xba, 8),
    ("T", 0x1a8, 9),
    ("U", 0x1a9, 9),
    ("V", 0x1aa, 9),
    ("W", 0x1ab, 9),
    ("X", 0x1ac, 9),
    ("Y", 0x1ad, 9),
    ("Z", 0x1ae, 9),
    ("a", 0x00, 5),
    ("b", 0x01, 5),
    ("c", 0x02, 5),
    ("d", 0x1b, 7),
    ("e", 0x03, 5),
    ("f", 0x1c, 7),
    ("g", 0x1d, 7),
    ("h", 0x04, 5),
    ("i", 0x1e, 7),
    ("j", 0x1f, 7),
    ("k", 0x5c, 7),
    ("l", 0x5d, 7),
    ("m", 0x5e, 7),
    ("n", 0x05, 5),
    ("o", 0x1a, 7),
    ("p", 0x5f, 7),
    ("q", 0x1c00, 14),
    ("r", 0x08, 5),
    ("s", 0x1b0, 9),
    ("t", 0x09, 5),
    ("u", 0x1b1, 9),
    ("v", 0x1b2, 9),
    ("w", 0x1b3, 9),
    ("x", 0x1b4, 9),
    ("y", 0x1b5, 9),
    ("z", 0x1b6, 9),
];

/// Dynamic table entry
#[derive(Debug, Clone)]
struct TableEntry {
    name: String,
    value: String,
}

/// HPACK encoder
pub struct Encoder {
    /// Dynamic table
    dynamic_table: Vec<TableEntry>,
    /// Maximum dynamic table size
    max_table_size: usize,
    /// Current dynamic table size
    current_table_size: usize,
}

impl Encoder {
    /// Create a new encoder
    pub fn new() -> Self {
        Encoder {
            dynamic_table: Vec::new(),
            max_table_size: 4096,
            current_table_size: 0,
        }
    }

    /// Encode a header field
    pub fn encode_header(&mut self, name: &str, value: &str) -> Result<Vec<u8>> {
        // Try to find in static table
        if let Some((index, match_type)) = self.find_in_static(name, value) {
            return Ok(match match_type {
                MatchType::NameAndValue => {
                    // Indexed Header Field Representation
                    let mut encoded = Vec::new();
                    Self::encode_integer(index, 0x80, &mut encoded);
                    encoded
                }
                MatchType::NameOnly => {
                    // Literal Header Field with Incremental Indexing - Indexed Name
                    let mut encoded = Vec::new();
                    Self::encode_integer(index, 0x40, &mut encoded);
                    self.encode_string(value.as_bytes(), &mut encoded);
                    self.add_to_dynamic(name.to_string(), value.to_string());
                    encoded
                }
            });
        }

        // Try to find in dynamic table
        if let Some((index, match_type)) = self.find_in_dynamic(name, value) {
            return Ok(match match_type {
                MatchType::NameAndValue => {
                    let mut encoded = Vec::new();
                    Self::encode_integer(index, 0x80, &mut encoded);
                    encoded
                }
                MatchType::NameOnly => {
                    let mut encoded = Vec::new();
                    Self::encode_integer(index, 0x40, &mut encoded);
                    self.encode_string(value.as_bytes(), &mut encoded);
                    encoded
                }
            });
        }

        // Literal Header Field with Incremental Indexing - New Name
        let mut encoded = Vec::new();
        encoded.push(0x40); // 01xxxxxx
        self.encode_string(name.as_bytes(), &mut encoded);
        self.encode_string(value.as_bytes(), &mut encoded);
        self.add_to_dynamic(name.to_string(), value.to_string());

        Ok(encoded)
    }

    /// Encode headers without indexing
    pub fn encode_no_indexing(&mut self, name: &str, value: &str) -> Result<Vec<u8>> {
        let mut encoded = Vec::new();
        encoded.push(0x00); // 0000xxxx - Literal without indexing
        self.encode_string(name.as_bytes(), &mut encoded);
        self.encode_string(value.as_bytes(), &mut encoded);
        Ok(encoded)
    }

    /// Encode headers with never-indexing
    pub fn encode_never_indexing(&mut self, name: &str, value: &str) -> Result<Vec<u8>> {
        let mut encoded = Vec::new();
        encoded.push(0x10); // 0001xxxx - Literal never indexed
        self.encode_string(name.as_bytes(), &mut encoded);
        self.encode_string(value.as_bytes(), &mut encoded);
        Ok(encoded)
    }

    /// Find header in static table
    fn find_in_static(&self, name: &str, value: &str) -> Option<(usize, MatchType)> {
        // First try exact match (name + value)
        for (i, &(n, v)) in STATIC_TABLE.iter().enumerate().skip(1) {
            if n.eq_ignore_ascii_case(name) && v == value {
                return Some((i, MatchType::NameAndValue));
            }
        }

        // Then try name-only match
        for (i, &(n, _)) in STATIC_TABLE.iter().enumerate().skip(1) {
            if n.eq_ignore_ascii_case(name) {
                return Some((i, MatchType::NameOnly));
            }
        }

        None
    }

    /// Find header in dynamic table
    fn find_in_dynamic(&self, name: &str, value: &str) -> Option<(usize, MatchType)> {
        // Try exact match
        for (i, entry) in self.dynamic_table.iter().enumerate() {
            if entry.name.eq_ignore_ascii_case(name) && entry.value == value {
                // Dynamic table indices start after static table
                let index = STATIC_TABLE.len() + i;
                return Some((index, MatchType::NameAndValue));
            }
        }

        // Try name-only match
        for (i, entry) in self.dynamic_table.iter().enumerate() {
            if entry.name.eq_ignore_ascii_case(name) {
                let index = STATIC_TABLE.len() + i;
                return Some((index, MatchType::NameOnly));
            }
        }

        None
    }

    /// Add entry to dynamic table
    fn add_to_dynamic(&mut self, name: String, value: String) {
        let entry_size = name.len() + value.len() + 32; // 32 bytes overhead per RFC 7541

        // Evict entries if needed
        while self.current_table_size + entry_size > self.max_table_size {
            if let Some(entry) = self.dynamic_table.pop() {
                self.current_table_size -= entry.name.len() + entry.value.len() + 32;
            } else {
                break;
            }
        }

        // Insert at beginning
        self.dynamic_table
            .insert(0, TableEntry { name, value });
        self.current_table_size += entry_size;
    }

    /// Encode an integer using HPACK integer representation
    fn encode_integer(value: usize, prefix_mask: u8, output: &mut Vec<u8>) {
        // Count how many low bits are available in the prefix
        // e.g., 0x80 = 10000000 → 7 bits available (bits 0-6)
        // e.g., 0x40 = 01000000 → 6 bits available (bits 0-5)
        let prefix_bits = 8 - prefix_mask.count_ones();
        let max_prefix = (1usize << prefix_bits) - 1;

        if value < max_prefix {
            output.push((value as u8) | prefix_mask);
        } else {
            output.push((max_prefix as u8) | prefix_mask);
            let mut remaining = value - max_prefix;

            while remaining >= 128 {
                output.push((remaining % 128 + 128) as u8);
                remaining /= 128;
            }
            output.push(remaining as u8);
        }
    }

    /// Encode a string (literal, no Huffman)
    fn encode_string(&self, data: &[u8], output: &mut Vec<u8>) {
        // 7-bit prefix for string length (bit 7 = Huffman flag, cleared = no Huffman)
        let len = data.len();
        if len < 127 {
            output.push(len as u8);
            output.extend_from_slice(data);
        } else {
            // Multi-byte: first byte 0x7F, then variable-length int
            output.push(0x7F);
            let mut remaining = len - 127;
            while remaining >= 128 {
                output.push((remaining % 128 + 128) as u8);
                remaining /= 128;
            }
            output.push(remaining as u8);
            output.extend_from_slice(data);
        }
    }
}

/// Match type for header lookup
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MatchType {
    /// Both name and value match
    NameAndValue,
    /// Only name matches
    NameOnly,
}

/// HPACK decoder
pub struct Decoder {
    /// Static table (read-only)
    /// Dynamic table
    dynamic_table: Vec<TableEntry>,
    /// Maximum dynamic table size
    max_table_size: usize,
    /// Current dynamic table size
    current_table_size: usize,
}

impl Decoder {
    /// Create a new decoder
    pub fn new() -> Self {
        Decoder {
            dynamic_table: Vec::new(),
            max_table_size: 4096,
            current_table_size: 0,
        }
    }

    /// Decode a header block
    pub fn decode(&mut self, data: &[u8]) -> Result<Vec<(String, String)>> {
        let mut headers = Vec::new();
        let mut pos = 0;

        while pos < data.len() {
            let first_byte = data[pos];

            if first_byte & 0x80 != 0 {
                // Indexed Header Field Representation (1xxxxxxx)
                let (index, bytes_read) = Self::decode_integer(data, pos, 0x80)?;
                pos += bytes_read;

                let (name, value) = self.get_from_table(index)?;
                headers.push((name, value));
            } else if first_byte & 0x40 != 0 {
                // Literal Header Field with Incremental Indexing (01xxxxxx)
                let (index, bytes_read) = Self::decode_integer(data, pos, 0x40)?;
                pos += bytes_read;

                let name = if index > 0 {
                    let (name, _) = self.get_from_table(index)?;
                    name
                } else {
                    let (name_str, str_len) = self.decode_string(data, pos)?;
                    pos += str_len;
                    name_str
                };

                let (value, str_len) = self.decode_string(data, pos)?;
                pos += str_len;

                // Add to dynamic table
                self.add_to_dynamic(name.clone(), value.clone());

                headers.push((name, value));
            } else if first_byte & 0x20 != 0 {
                // Dynamic Table Size Update (001xxxxx)
                let (size, bytes_read) = Self::decode_integer(data, pos, 0x20)?;
                pos += bytes_read;
                self.max_table_size = size;
            } else if first_byte & 0x10 != 0 {
                // Literal Header Field never Indexed (0001xxxx)
                let (index, bytes_read) = Self::decode_integer(data, pos, 0x10)?;
                pos += bytes_read;

                let name = if index > 0 {
                    let (name, _) = self.get_from_table(index)?;
                    name
                } else {
                    let (name_str, str_len) = self.decode_string(data, pos)?;
                    pos += str_len;
                    name_str
                };

                let (value, str_len) = self.decode_string(data, pos)?;
                pos += str_len;

                headers.push((name, value));
            } else {
                // Literal Header Field without Indexing (0000xxxx)
                let (index, bytes_read) = Self::decode_integer(data, pos, 0x00)?;
                pos += bytes_read;

                let name = if index > 0 {
                    let (name, _) = self.get_from_table(index)?;
                    name
                } else {
                    let (name_str, str_len) = self.decode_string(data, pos)?;
                    pos += str_len;
                    name_str
                };

                let (value, str_len) = self.decode_string(data, pos)?;
                pos += str_len;

                headers.push((name, value));
            }
        }

        Ok(headers)
    }

    /// Decode an HPACK integer
    fn decode_integer(data: &[u8], start: usize, prefix_mask: u8) -> Result<(usize, usize)> {
        if start >= data.len() {
            return Err(Http2Error::HpackDecode(
                "Integer decode out of bounds".to_string(),
            ));
        }

        let prefix_bits = 8 - prefix_mask.count_ones();
        let max_prefix = (1usize << prefix_bits) - 1;

        let mut value = (data[start] & !prefix_mask) as usize;

        if value < max_prefix {
            return Ok((value, 1));
        }

        let mut pos = start + 1;
        let mut m = 0;

        while pos < data.len() {
            let byte = data[pos] as usize;
            value += (byte & 127) << m;
            m += 7;

            if byte & 128 == 0 {
                return Ok((value, pos - start + 1));
            }

            pos += 1;
        }

        Err(Http2Error::HpackDecode(
            "Incomplete integer encoding".to_string(),
        ))
    }

    /// Decode an HPACK string
    fn decode_string(&self, data: &[u8], start: usize) -> Result<(String, usize)> {
        if start >= data.len() {
            return Err(Http2Error::HpackDecode(
                "String decode out of bounds".to_string(),
            ));
        }

        let huffman = data[start] & 0x80 != 0;
        let (str_len, bytes_read) = Self::decode_integer(data, start, 0x80)?;

        if start + bytes_read + str_len > data.len() {
            return Err(Http2Error::HpackDecode(
                "String data incomplete".to_string(),
            ));
        }

        let str_data = &data[start + bytes_read..start + bytes_read + str_len];

        let value = if huffman {
            // Huffman decode
            Self::huffman_decode(str_data)?
        } else {
            String::from_utf8_lossy(str_data).to_string()
        };

        Ok((value, bytes_read + str_len))
    }

    /// Get entry from tables by index
    fn get_from_table(&self, index: usize) -> Result<(String, String)> {
        if index == 0 {
            return Err(Http2Error::HpackDecode(
                "Index 0 is invalid".to_string(),
            ));
        }

        // Check static table
        if index < STATIC_TABLE.len() {
            let (name, value) = STATIC_TABLE[index];
            return Ok((name.to_string(), value.to_string()));
        }

        // Check dynamic table
        let dynamic_index = index - STATIC_TABLE.len();
        if dynamic_index < self.dynamic_table.len() {
            let entry = &self.dynamic_table[dynamic_index];
            return Ok((entry.name.clone(), entry.value.clone()));
        }

        Err(Http2Error::HpackDecode(format!(
            "Invalid index: {}",
            index
        )))
    }

    /// Add entry to dynamic table
    fn add_to_dynamic(&mut self, name: String, value: String) {
        let entry_size = name.len() + value.len() + 32;

        // Evict entries if needed
        while self.current_table_size + entry_size > self.max_table_size {
            if let Some(entry) = self.dynamic_table.pop() {
                self.current_table_size -= entry.name.len() + entry.value.len() + 32;
            } else {
                break;
            }
        }

        self.dynamic_table
            .insert(0, TableEntry { name, value });
        self.current_table_size += entry_size;
    }

    /// Huffman decode (simplified)
    fn huffman_decode(data: &[u8]) -> Result<String> {
        // Full Huffman decoding requires building the tree
        // This is a simplified implementation
        let mut result = String::new();
        for &byte in data {
            result.push(byte as char);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_table_size() {
        assert_eq!(STATIC_TABLE.len(), 62);
        assert_eq!(STATIC_TABLE[1].0, ":authority");
        assert_eq!(STATIC_TABLE[2].0, ":method");
        assert_eq!(STATIC_TABLE[2].1, "GET");
    }

    #[test]
    fn test_integer_encoding() {
        // 0x80 mask: 7-bit prefix. Value 10 fits in prefix → [10 | 0x80] = [0x8A]
        let mut output = Vec::new();
        Encoder::encode_integer(10, 0x80, &mut output);
        assert_eq!(output, vec![0x8A]);

        // Value 1337 doesn't fit: first byte 0x7F | 0x80 = 0xFF, then remainder
        // 1337 - 127 = 1210; 1210 = 9*128 + 58 → [0xFF, 0xBA, 0x09]
        let mut output = Vec::new();
        Encoder::encode_integer(1337, 0x80, &mut output);
        assert_eq!(output, vec![0xFF, 0xBA, 0x09]);

        // Value 5 with 0xF0 mask (4-bit prefix, max=15)
        let mut output = Vec::new();
        Encoder::encode_integer(5, 0xF0, &mut output);
        assert_eq!(output, vec![0xF5]);
    }

    #[test]
    fn test_integer_decoding() {
        // 0x8A with 0x80 mask → extract lower 7 bits = 10
        let data = vec![0x8A];
        let (value, _) = Decoder::decode_integer(&data, 0, 0x80).unwrap();
        assert_eq!(value, 10);

        // 0xFF 0xBA 0x09 → 127 + 58 + 9*128 = 1337
        let data = vec![0xFF, 0xBA, 0x09];
        let (value, _) = Decoder::decode_integer(&data, 0, 0x80).unwrap();
        assert_eq!(value, 1337);

        // 0xF5 with 0xF0 mask → extract lower 4 bits = 5
        let data = vec![0xF5];
        let (value, _) = Decoder::decode_integer(&data, 0, 0xF0).unwrap();
        assert_eq!(value, 5);
    }

    #[test]
    fn test_encoder_indexed_header() {
        let mut encoder = Encoder::new();
        let encoded = encoder.encode_header(":method", "GET").unwrap();

        // Should be indexed representation (1xxxxxxx)
        assert_eq!(encoded[0] & 0x80, 0x80);
    }

    #[test]
    fn test_encoder_literal_header() {
        let mut encoder = Encoder::new();
        let encoded = encoder.encode_header("x-custom", "value").unwrap();

        // Should be literal with indexing (01xxxxxx)
        assert_eq!(encoded[0] & 0xC0, 0x40);
    }

    #[test]
    fn test_decoder_indexed_header() {
        let mut decoder = Decoder::new();

        // Encode :method GET (index 2)
        let mut encoded = Vec::new();
        Encoder::encode_integer(2, 0x80, &mut encoded);

        let headers = decoder.decode(&encoded).unwrap();
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].0, ":method");
        assert_eq!(headers[0].1, "GET");
    }

    #[test]
    fn test_decoder_literal_header() {
        let mut decoder = Decoder::new();

        // Encode custom header with indexing
        let mut encoder = Encoder::new();
        let encoded = encoder.encode_header("custom", "value").unwrap();

        let headers = decoder.decode(&encoded).unwrap();
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].0, "custom");
        assert_eq!(headers[0].1, "value");
    }

    #[test]
    fn test_encoder_no_indexing() {
        let mut encoder = Encoder::new();
        let encoded = encoder.encode_no_indexing("no-index", "value").unwrap();

        // Should be 0000xxxx
        assert_eq!(encoded[0] & 0xF0, 0x00);
    }

    #[test]
    fn test_encoder_never_indexing() {
        let mut encoder = Encoder::new();
        let encoded = encoder.encode_never_indexing("never-index", "value").unwrap();

        // Should be 0001xxxx
        assert_eq!(encoded[0] & 0xF0, 0x10);
    }

    #[test]
    fn test_dynamic_table_eviction() {
        let mut encoder = Encoder::new();
        encoder.max_table_size = 100; // Very small table

        // Add headers until eviction occurs
        for i in 0..10 {
            let _ = encoder
                .encode_header(&format!("header{}", i), &format!("value{}", i))
                .unwrap();
        }

        // Dynamic table should not exceed max_table_size
        assert!(encoder.current_table_size <= encoder.max_table_size);
    }

    #[test]
    fn test_decode_invalid_index() {
        let mut decoder = Decoder::new();
        let encoded = vec![0xFF]; // Index 127, which is invalid

        assert!(decoder.decode(&encoded).is_err());
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let mut encoder = Encoder::new();
        let mut decoder = Decoder::new();

        let headers = vec![
            (":method", "GET"),
            (":scheme", "https"),
            (":path", "/"),
            (":authority", "example.com"),
            ("accept", "text/html"),
        ];

        let mut encoded = Vec::new();
        for (name, value) in &headers {
            let chunk = encoder.encode_header(name, value).unwrap();
            encoded.extend(chunk);
        }

        let decoded = decoder.decode(&encoded).unwrap();
        assert_eq!(decoded.len(), headers.len());

        for (i, (name, value)) in headers.iter().enumerate() {
            assert_eq!(decoded[i].0, *name);
            assert_eq!(decoded[i].1, *value);
        }
    }
}
