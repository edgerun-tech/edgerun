#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

pub const WIRE_MAGIC: &[u8; 4] = b"ERW0";
pub const WIRE_VERSION: u8 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WireError {
    Eof,
    InvalidMagic,
    InvalidVersion,
    InvalidTag,
    InvalidUtf8,
    NonCanonicalInteger,
    LengthOverflow,
    TrailingBytes,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WireValue {
    Unit,
    Bool(bool),
    U64(u64),
    I64(i64),
    Bytes(Vec<u8>),
    Text(String),
    List(Vec<WireValue>),
    Map(Vec<(WireValue, WireValue)>),
    Struct(Vec<WireField>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireField {
    pub tag: u32,
    pub value: WireValue,
}

pub trait WireEncode {
    fn encode_wire(&self, out: &mut Vec<u8>);

    fn to_wire_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        write_header(&mut out);
        self.encode_wire(&mut out);
        out
    }
}

pub trait WireDecode: Sized {
    fn decode_wire(input: &mut WireReader<'_>) -> Result<Self, WireError>;

    fn from_wire_bytes(bytes: &[u8]) -> Result<Self, WireError> {
        let mut input = WireReader::new(bytes);
        read_header(&mut input)?;
        let value = Self::decode_wire(&mut input)?;
        if !input.is_empty() {
            return Err(WireError::TrailingBytes);
        }
        Ok(value)
    }
}

pub fn canonical_bytes<T: WireEncode>(value: &T) -> Vec<u8> {
    value.to_wire_bytes()
}

pub fn write_header(out: &mut Vec<u8>) {
    out.extend_from_slice(WIRE_MAGIC);
    out.push(WIRE_VERSION);
}

pub fn read_header(input: &mut WireReader<'_>) -> Result<(), WireError> {
    let magic = input.take(4)?;
    if magic != WIRE_MAGIC {
        return Err(WireError::InvalidMagic);
    }
    let version = input.read_u8()?;
    if version != WIRE_VERSION {
        return Err(WireError::InvalidVersion);
    }
    Ok(())
}

pub struct WireReader<'a> {
    input: &'a [u8],
    offset: usize,
}

impl<'a> WireReader<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self { input, offset: 0 }
    }

    pub fn is_empty(&self) -> bool {
        self.offset == self.input.len()
    }

    pub fn read_u8(&mut self) -> Result<u8, WireError> {
        let b = *self.input.get(self.offset).ok_or(WireError::Eof)?;
        self.offset += 1;
        Ok(b)
    }

    pub fn take(&mut self, len: usize) -> Result<&'a [u8], WireError> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or(WireError::LengthOverflow)?;
        if end > self.input.len() {
            return Err(WireError::Eof);
        }
        let out = &self.input[self.offset..end];
        self.offset = end;
        Ok(out)
    }

    pub fn read_varu64(&mut self) -> Result<u64, WireError> {
        let mut value = 0u64;
        let mut shift = 0u32;
        for i in 0..10 {
            let b = self.read_u8()?;
            if shift == 63 && b > 1 {
                return Err(WireError::LengthOverflow);
            }
            value |= ((b & 0x7f) as u64) << shift;
            if b & 0x80 == 0 {
                if i > 0 && value < (1u64 << (7 * i)) {
                    return Err(WireError::NonCanonicalInteger);
                }
                return Ok(value);
            }
            shift += 7;
        }
        Err(WireError::LengthOverflow)
    }

    pub fn read_len(&mut self) -> Result<usize, WireError> {
        let len = self.read_varu64()?;
        usize::try_from(len).map_err(|_| WireError::LengthOverflow)
    }
}

pub fn write_varu64(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let mut b = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            b |= 0x80;
        }
        out.push(b);
        if value == 0 {
            break;
        }
    }
}

pub fn write_len(out: &mut Vec<u8>, len: usize) {
    write_varu64(out, len as u64);
}

pub fn zigzag_i64(value: i64) -> u64 {
    ((value << 1) ^ (value >> 63)) as u64
}

pub fn unzigzag_i64(value: u64) -> i64 {
    ((value >> 1) as i64) ^ (-((value & 1) as i64))
}

impl WireEncode for bool {
    fn encode_wire(&self, out: &mut Vec<u8>) {
        out.push(if *self { 1 } else { 0 });
    }
}

impl WireDecode for bool {
    fn decode_wire(input: &mut WireReader<'_>) -> Result<Self, WireError> {
        match input.read_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(WireError::InvalidTag),
        }
    }
}

impl WireEncode for u64 {
    fn encode_wire(&self, out: &mut Vec<u8>) {
        write_varu64(out, *self);
    }
}

impl WireDecode for u64 {
    fn decode_wire(input: &mut WireReader<'_>) -> Result<Self, WireError> {
        input.read_varu64()
    }
}

impl WireEncode for u32 {
    fn encode_wire(&self, out: &mut Vec<u8>) {
        write_varu64(out, *self as u64);
    }
}

impl WireDecode for u32 {
    fn decode_wire(input: &mut WireReader<'_>) -> Result<Self, WireError> {
        let value = input.read_varu64()?;
        u32::try_from(value).map_err(|_| WireError::LengthOverflow)
    }
}

impl WireEncode for i64 {
    fn encode_wire(&self, out: &mut Vec<u8>) {
        write_varu64(out, zigzag_i64(*self));
    }
}

impl WireDecode for i64 {
    fn decode_wire(input: &mut WireReader<'_>) -> Result<Self, WireError> {
        Ok(unzigzag_i64(input.read_varu64()?))
    }
}

impl WireEncode for Vec<u8> {
    fn encode_wire(&self, out: &mut Vec<u8>) {
        write_len(out, self.len());
        out.extend_from_slice(self);
    }
}

impl WireDecode for Vec<u8> {
    fn decode_wire(input: &mut WireReader<'_>) -> Result<Self, WireError> {
        let len = input.read_len()?;
        Ok(input.take(len)?.to_vec())
    }
}

impl WireEncode for String {
    fn encode_wire(&self, out: &mut Vec<u8>) {
        let bytes = self.as_bytes();
        write_len(out, bytes.len());
        out.extend_from_slice(bytes);
    }
}

impl WireDecode for String {
    fn decode_wire(input: &mut WireReader<'_>) -> Result<Self, WireError> {
        let bytes = Vec::<u8>::decode_wire(input)?;
        String::from_utf8(bytes).map_err(|_| WireError::InvalidUtf8)
    }
}

impl WireEncode for &str {
    fn encode_wire(&self, out: &mut Vec<u8>) {
        let bytes = self.as_bytes();
        write_len(out, bytes.len());
        out.extend_from_slice(bytes);
    }
}

impl<T: WireEncode> WireEncode for Option<T> {
    fn encode_wire(&self, out: &mut Vec<u8>) {
        match self {
            None => out.push(0),
            Some(value) => {
                out.push(1);
                value.encode_wire(out);
            }
        }
    }
}

impl<T: WireDecode> WireDecode for Option<T> {
    fn decode_wire(input: &mut WireReader<'_>) -> Result<Self, WireError> {
        match input.read_u8()? {
            0 => Ok(None),
            1 => Ok(Some(T::decode_wire(input)?)),
            _ => Err(WireError::InvalidTag),
        }
    }
}

pub fn write_field<T: WireEncode>(out: &mut Vec<u8>, tag: u32, value: &T) {
    tag.encode_wire(out);
    value.encode_wire(out);
}

pub fn write_struct<T>(out: &mut Vec<u8>, fields: &[T])
where
    T: Fn(&mut Vec<u8>),
{
    write_len(out, fields.len());
    for field in fields {
        field(out);
    }
}

impl WireEncode for WireField {
    fn encode_wire(&self, out: &mut Vec<u8>) {
        self.tag.encode_wire(out);
        self.value.encode_wire(out);
    }
}

impl WireDecode for WireField {
    fn decode_wire(input: &mut WireReader<'_>) -> Result<Self, WireError> {
        Ok(Self {
            tag: u32::decode_wire(input)?,
            value: WireValue::decode_wire(input)?,
        })
    }
}

impl WireEncode for WireValue {
    fn encode_wire(&self, out: &mut Vec<u8>) {
        match self {
            WireValue::Unit => out.push(0),
            WireValue::Bool(v) => {
                out.push(1);
                v.encode_wire(out);
            }
            WireValue::U64(v) => {
                out.push(2);
                v.encode_wire(out);
            }
            WireValue::I64(v) => {
                out.push(3);
                v.encode_wire(out);
            }
            WireValue::Bytes(v) => {
                out.push(4);
                v.encode_wire(out);
            }
            WireValue::Text(v) => {
                out.push(5);
                v.encode_wire(out);
            }
            WireValue::List(v) => {
                out.push(6);
                write_len(out, v.len());
                for item in v {
                    item.encode_wire(out);
                }
            }
            WireValue::Map(v) => {
                out.push(7);
                write_len(out, v.len());
                for (k, val) in v {
                    k.encode_wire(out);
                    val.encode_wire(out);
                }
            }
            WireValue::Struct(v) => {
                out.push(8);
                write_len(out, v.len());
                let mut last = 0u32;
                for field in v {
                    debug_assert!(field.tag > last);
                    last = field.tag;
                    field.encode_wire(out);
                }
            }
        }
    }
}

impl WireDecode for WireValue {
    fn decode_wire(input: &mut WireReader<'_>) -> Result<Self, WireError> {
        match input.read_u8()? {
            0 => Ok(WireValue::Unit),
            1 => Ok(WireValue::Bool(bool::decode_wire(input)?)),
            2 => Ok(WireValue::U64(u64::decode_wire(input)?)),
            3 => Ok(WireValue::I64(i64::decode_wire(input)?)),
            4 => Ok(WireValue::Bytes(Vec::<u8>::decode_wire(input)?)),
            5 => Ok(WireValue::Text(String::decode_wire(input)?)),
            6 => {
                let len = input.read_len()?;
                let mut values = Vec::with_capacity(len);
                for _ in 0..len {
                    values.push(WireValue::decode_wire(input)?);
                }
                Ok(WireValue::List(values))
            }
            7 => {
                let len = input.read_len()?;
                let mut values = Vec::with_capacity(len);
                for _ in 0..len {
                    values.push((
                        WireValue::decode_wire(input)?,
                        WireValue::decode_wire(input)?,
                    ));
                }
                Ok(WireValue::Map(values))
            }
            8 => {
                let len = input.read_len()?;
                let mut fields = Vec::with_capacity(len);
                let mut last = 0u32;
                for _ in 0..len {
                    let field = WireField::decode_wire(input)?;
                    if field.tag <= last {
                        return Err(WireError::InvalidTag);
                    }
                    last = field.tag;
                    fields.push(field);
                }
                Ok(WireValue::Struct(fields))
            }
            _ => Err(WireError::InvalidTag),
        }
    }
}

pub fn struct_value(mut fields: Vec<WireField>) -> WireValue {
    fields.sort_by_key(|f| f.tag);
    fields.dedup_by_key(|f| f.tag);
    WireValue::Struct(fields)
}

pub fn field(tag: u32, value: WireValue) -> WireField {
    WireField { tag, value }
}

pub fn bytes(value: impl AsRef<[u8]>) -> WireValue {
    WireValue::Bytes(value.as_ref().to_vec())
}

pub fn text(value: impl ToString) -> WireValue {
    WireValue::Text(value.to_string())
}

pub fn u64v(value: u64) -> WireValue {
    WireValue::U64(value)
}

pub fn i64v(value: i64) -> WireValue {
    WireValue::I64(value)
}

pub fn boolv(value: bool) -> WireValue {
    WireValue::Bool(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varint_is_minimal_and_roundtrips() {
        let values = [0, 1, 127, 128, 255, 16_384, u32::MAX as u64, u64::MAX];
        for value in values {
            let mut out = Vec::new();
            write_varu64(&mut out, value);
            let mut input = WireReader::new(&out);
            assert_eq!(input.read_varu64().unwrap(), value);
            assert!(input.is_empty());
        }
    }

    #[test]
    fn rejects_non_canonical_varint() {
        let mut input = WireReader::new(&[0x80, 0x00]);
        assert_eq!(input.read_varu64(), Err(WireError::NonCanonicalInteger));
    }

    #[test]
    fn value_roundtrip_is_deterministic() {
        let value = struct_value(vec![
            field(3, text("hello")),
            field(1, u64v(42)),
            field(2, WireValue::List(vec![bytes([1, 2, 3]), boolv(true)])),
        ]);
        let a = canonical_bytes(&value);
        let b = canonical_bytes(&value);
        assert_eq!(a, b);
        let decoded = WireValue::from_wire_bytes(&a).unwrap();
        assert_eq!(decoded, value);
    }
}
