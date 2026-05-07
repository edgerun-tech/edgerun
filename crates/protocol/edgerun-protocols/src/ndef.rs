//! NFC Forum NDEF message encoding and decoding.

use crate::prelude::*;
use alloc::vec;

/// An NDEF record within an NDEF message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NdefRecord {
    pub tnf: NdefTnf,
    pub type_name: Vec<u8>,
    pub id: Vec<u8>,
    pub payload: Vec<u8>,
}

impl NdefRecord {
    /// Creates a well-known text record (TNF=1, type="T").
    pub fn text_record(text: &str, language_code: &str) -> Self {
        let mut payload = Vec::new();
        payload.push(language_code.len() as u8);
        payload.extend_from_slice(language_code.as_bytes());
        payload.extend_from_slice(text.as_bytes());
        NdefRecord {
            tnf: NdefTnf::NfcWellKnown,
            type_name: b"T".to_vec(),
            id: Vec::new(),
            payload,
        }
    }

    /// Creates a URI record (TNF=1, type="U").
    pub fn uri_record(uri: &str) -> Self {
        NdefRecord {
            tnf: NdefTnf::NfcWellKnown,
            type_name: b"U".to_vec(),
            id: Vec::new(),
            payload: uri.as_bytes().to_vec(),
        }
    }

    /// Creates a MIME media record with arbitrary payload.
    pub fn mime_record(mime_type: &str, data: &[u8]) -> Self {
        NdefRecord {
            tnf: NdefTnf::Media,
            type_name: mime_type.as_bytes().to_vec(),
            id: Vec::new(),
            payload: data.to_vec(),
        }
    }

    /// If this record is a well-known text record, decode it.
    pub fn as_text(&self) -> Option<String> {
        if self.tnf != NdefTnf::NfcWellKnown || self.type_name != b"T" {
            return None;
        }
        if self.payload.is_empty() {
            return None;
        }
        let lang_len = self.payload[0] as usize & 0x3F;
        if lang_len + 1 >= self.payload.len() {
            return None;
        }
        let text = &self.payload[lang_len + 1..];
        String::from_utf8(text.to_vec()).ok()
    }

    /// If this record is a well-known URI record, decode it.
    pub fn as_uri(&self) -> Option<String> {
        if self.tnf != NdefTnf::NfcWellKnown || self.type_name != b"U" {
            return None;
        }
        String::from_utf8(self.payload.clone()).ok()
    }
}

/// NDEF Type Name Format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NdefTnf {
    Empty = 0x00,
    NfcWellKnown = 0x01,
    Media = 0x02,
    AbsoluteUri = 0x03,
    NfcExternal = 0x04,
    Unknown = 0x05,
    Unchanged = 0x06,
    Reserved = 0x07,
}

impl NdefTnf {
    pub fn from_u8(v: u8) -> Self {
        match v & 0x07 {
            0x00 => NdefTnf::Empty,
            0x01 => NdefTnf::NfcWellKnown,
            0x02 => NdefTnf::Media,
            0x03 => NdefTnf::AbsoluteUri,
            0x04 => NdefTnf::NfcExternal,
            0x05 => NdefTnf::Unknown,
            0x06 => NdefTnf::Unchanged,
            _ => NdefTnf::Reserved,
        }
    }
}

/// An NDEF message containing one or more records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NdefMessage {
    pub records: Vec<NdefRecord>,
}

impl NdefMessage {
    pub fn new(records: Vec<NdefRecord>) -> Self {
        Self { records }
    }

    pub fn single_text(text: &str) -> Self {
        Self {
            records: vec![NdefRecord::text_record(text, "en")],
        }
    }

    /// Serialize this NDEF message to raw bytes per NFC Forum NDEF spec.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let last = self.records.len().saturating_sub(1);
        for (i, record) in self.records.iter().enumerate() {
            let is_first = i == 0;
            let is_last = i == last;
            let tnf = record.tnf as u8 & 0x07;

            let is_short = record.payload.len() <= 255;
            let has_id = !record.id.is_empty() && is_first;

            let mut flags = tnf;
            if is_first {
                flags |= 0x80;
            }
            if is_last {
                flags |= 0x40;
            }
            if is_short {
                flags |= 0x10;
            }
            if has_id {
                flags |= 0x08;
            }

            out.push(flags);
            out.push(record.type_name.len() as u8);

            if is_short {
                out.push(record.payload.len() as u8);
            } else {
                let len = record.payload.len() as u32;
                out.extend_from_slice(&len.to_be_bytes());
            }

            if has_id {
                out.push(record.id.len() as u8);
            }

            out.extend_from_slice(&record.type_name);
            out.extend_from_slice(&record.id);
            out.extend_from_slice(&record.payload);
        }
        out
    }

    /// Parse an NDEF message from raw bytes.
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        let mut records = Vec::new();
        let mut pos = 0;
        let mut message_end = false;

        while pos < data.len() && !message_end {
            let flags = data[pos];
            pos += 1;

            let tnf = NdefTnf::from_u8(flags);
            message_end = (flags & 0x40) != 0;
            let is_short = (flags & 0x10) != 0;
            let has_id = (flags & 0x08) != 0;

            if pos >= data.len() {
                return None;
            }
            let type_len = data[pos] as usize;
            pos += 1;

            let payload_len = if is_short {
                if pos >= data.len() {
                    return None;
                }
                let len = data[pos] as usize;
                pos += 1;
                len
            } else {
                if pos + 4 > data.len() {
                    return None;
                }
                let len =
                    u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
                        as usize;
                pos += 4;
                len
            };

            let id_len = if has_id {
                if pos >= data.len() {
                    return None;
                }
                let len = data[pos] as usize;
                pos += 1;
                len
            } else {
                0
            };

            if pos + type_len > data.len() {
                return None;
            }
            let type_name = data[pos..pos + type_len].to_vec();
            pos += type_len;

            if pos + id_len > data.len() {
                return None;
            }
            let id = data[pos..pos + id_len].to_vec();
            pos += id_len;

            if pos + payload_len > data.len() {
                return None;
            }
            let payload = data[pos..pos + payload_len].to_vec();
            pos += payload_len;

            records.push(NdefRecord {
                tnf,
                type_name,
                id,
                payload,
            });
        }

        if records.is_empty() {
            None
        } else {
            Some(NdefMessage { records })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn text_roundtrip() {
        let msg = NdefMessage::single_text("Hello World");
        let bytes = msg.to_bytes();
        let parsed = NdefMessage::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.records.len(), 1);
        assert_eq!(parsed.records[0].as_text().as_deref(), Some("Hello World"));
    }

    #[test]
    fn uri_roundtrip() {
        let msg = NdefMessage {
            records: vec![NdefRecord::uri_record("https://example.com")],
        };
        let bytes = msg.to_bytes();
        let parsed = NdefMessage::from_bytes(&bytes).unwrap();
        assert_eq!(
            parsed.records[0].as_uri().as_deref(),
            Some("https://example.com")
        );
    }

    #[test]
    fn mime_roundtrip() {
        let msg = NdefMessage {
            records: vec![NdefRecord::mime_record(
                "application/json",
                b"{\"key\":\"value\"}",
            )],
        };
        let bytes = msg.to_bytes();
        let parsed = NdefMessage::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.records[0].tnf, NdefTnf::Media);
        assert_eq!(parsed.records[0].type_name, b"application/json");
        assert_eq!(parsed.records[0].payload, b"{\"key\":\"value\"}");
    }

    #[test]
    fn multi_record_roundtrip() {
        let msg = NdefMessage {
            records: vec![
                NdefRecord::text_record("Hello", "en"),
                NdefRecord::uri_record("https://example.com"),
            ],
        };
        let bytes = msg.to_bytes();
        let parsed = NdefMessage::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.records.len(), 2);
    }

    #[test]
    fn rejects_empty_and_corrupted_input() {
        assert!(NdefMessage::from_bytes(&[]).is_none());
        assert!(NdefMessage::from_bytes(&[0xFF, 0xFF]).is_none());
    }
}
