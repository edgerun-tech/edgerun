//! Matter TLV codec.

use crate::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnonymousTag;

impl AnonymousTag {
    pub const fn new() -> Self {
        AnonymousTag
    }
}

impl Default for AnonymousTag {
    fn default() -> Self {
        Self::new()
    }
}

pub const TAG_ANONYMOUS: u8 = 0xFF;

#[derive(Debug, Clone)]
pub struct TlvWriter {
    buf: Vec<u8>,
}

impl TlvWriter {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn write_bool(&mut self, value: bool) {
        self.buf.push(TAG_ANONYMOUS);
        self.buf.push(0x01);
        self.buf.push(if value { 0x01 } else { 0x00 });
    }

    pub fn write_i8(&mut self, value: i8) {
        self.buf.push(TAG_ANONYMOUS);
        self.buf.push(0x04);
        self.buf.push(value as u8);
    }

    pub fn write_i16(&mut self, value: i16) {
        self.buf.push(TAG_ANONYMOUS);
        self.buf.push(0x05);
        self.buf.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_i32(&mut self, value: i32) {
        self.buf.push(TAG_ANONYMOUS);
        self.buf.push(0x06);
        self.buf.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_i64(&mut self, value: i64) {
        self.buf.push(TAG_ANONYMOUS);
        self.buf.push(0x07);
        self.buf.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_u8(&mut self, value: u8) {
        self.buf.push(TAG_ANONYMOUS);
        self.buf.push(0x08);
        self.buf.push(value);
    }

    pub fn write_u16(&mut self, value: u16) {
        self.buf.push(TAG_ANONYMOUS);
        self.buf.push(0x09);
        self.buf.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_u32(&mut self, value: u32) {
        self.buf.push(TAG_ANONYMOUS);
        self.buf.push(0x0A);
        self.buf.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_u64(&mut self, value: u64) {
        self.buf.push(TAG_ANONYMOUS);
        self.buf.push(0x0B);
        self.buf.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_f32(&mut self, value: f32) {
        self.buf.push(TAG_ANONYMOUS);
        self.buf.push(0x0C);
        self.buf.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_f64(&mut self, value: f64) {
        self.buf.push(TAG_ANONYMOUS);
        self.buf.push(0x0D);
        self.buf.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_str(&mut self, value: &str) {
        self.buf.push(TAG_ANONYMOUS);
        let bytes = value.as_bytes();
        let len = bytes.len() as u32;
        self.buf.push(0x10);
        self.buf.extend_from_slice(&len.to_be_bytes());
        self.buf.extend_from_slice(bytes);
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }
}

impl Default for TlvWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct TlvReader {
    buf: Vec<u8>,
    pos: usize,
}

impl TlvReader {
    pub fn new(data: Vec<u8>) -> Self {
        Self { buf: data, pos: 0 }
    }

    fn read_byte(&mut self) -> Option<u8> {
        if self.pos < self.buf.len() {
            let b = self.buf[self.pos];
            self.pos += 1;
            Some(b)
        } else {
            None
        }
    }

    fn read_bytes(&mut self, n: usize) -> Option<&[u8]> {
        if self.pos + n <= self.buf.len() {
            let start = self.pos;
            self.pos += n;
            Some(&self.buf[start..start + n])
        } else {
            None
        }
    }

    pub fn read_tag(&mut self) -> Option<u8> {
        self.read_byte()
    }

    pub fn read_type(&mut self) -> Option<u8> {
        self.read_byte()
    }

    pub fn read_bool(&mut self) -> bool {
        if let (Some(TAG_ANONYMOUS), Some(0x01 | 0x00)) = (self.read_byte(), self.read_byte()) {
            self.read_byte() == Some(0x01)
        } else {
            false
        }
    }

    pub fn read_i32(&mut self) -> i32 {
        if let (Some(TAG_ANONYMOUS), Some(0x06)) = (self.read_byte(), self.read_byte()) {
            if let Some(bytes) = self.read_bytes(4) {
                return i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            }
        }
        0
    }

    pub fn read_u32(&mut self) -> u32 {
        if let (Some(TAG_ANONYMOUS), Some(0x0A)) = (self.read_byte(), self.read_byte()) {
            if let Some(bytes) = self.read_bytes(4) {
                return u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            }
        }
        0
    }

    pub fn read_i64(&mut self) -> i64 {
        if let (Some(TAG_ANONYMOUS), Some(0x07)) = (self.read_byte(), self.read_byte()) {
            if let Some(bytes) = self.read_bytes(8) {
                return i64::from_be_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                ]);
            }
        }
        0
    }

    pub fn read_u64(&mut self) -> u64 {
        if let (Some(TAG_ANONYMOUS), Some(0x0B)) = (self.read_byte(), self.read_byte()) {
            if let Some(bytes) = self.read_bytes(8) {
                return u64::from_be_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                ]);
            }
        }
        0
    }
}

pub fn encode_int(value: i32) -> Vec<u8> {
    let mut writer = TlvWriter::new();
    writer.write_i32(value);
    writer.into_bytes()
}

pub fn encode_bool(value: bool) -> Vec<u8> {
    let mut writer = TlvWriter::new();
    writer.write_bool(value);
    writer.into_bytes()
}

pub fn encode_uint(value: u32) -> Vec<u8> {
    let mut writer = TlvWriter::new();
    writer.write_u32(value);
    writer.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_int() {
        let encoded = encode_int(1600);
        assert!(!encoded.is_empty());
    }

    #[test]
    fn test_encode_bool() {
        let encoded = encode_bool(true);
        assert!(!encoded.is_empty());
    }

    #[test]
    fn test_round_trip_i32() {
        let encoded = encode_int(1600);
        let mut reader = TlvReader::new(encoded);
        assert_eq!(reader.read_i32(), 1600);
    }
}
