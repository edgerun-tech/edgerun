extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

pub const UI_RECORD_MAX_FIELD_LEN: usize = 16 * 1024 * 1024;
pub const UI_RECORD_MAX_VEC_ENTRIES: usize = 65_536;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiRecordError {
    WrongMagic,
    LengthOverflow,
    LimitExceeded,
    Truncated,
    InvalidBool,
    InvalidUtf8,
    TrailingBytes,
}

pub struct UiRecordWriter {
    bytes: Vec<u8>,
}

impl UiRecordWriter {
    pub fn new(magic: &[u8]) -> Self {
        let mut bytes = Vec::with_capacity(magic.len());
        bytes.extend_from_slice(magic);
        Self { bytes }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn bytes(&mut self, value: &[u8]) -> Result<&mut Self, UiRecordError> {
        if value.len() > UI_RECORD_MAX_FIELD_LEN {
            return Err(UiRecordError::LimitExceeded);
        }
        let len = u32::try_from(value.len()).map_err(|_| UiRecordError::LengthOverflow)?;
        self.bytes.extend_from_slice(&len.to_le_bytes());
        self.bytes.extend_from_slice(value);
        Ok(self)
    }

    pub fn string(&mut self, value: &str) -> Result<&mut Self, UiRecordError> {
        self.bytes(value.as_bytes())
    }

    pub fn string_vec(&mut self, values: &[String]) -> Result<&mut Self, UiRecordError> {
        if values.len() > UI_RECORD_MAX_VEC_ENTRIES {
            return Err(UiRecordError::LimitExceeded);
        }
        let len = u32::try_from(values.len()).map_err(|_| UiRecordError::LengthOverflow)?;
        self.bytes.extend_from_slice(&len.to_le_bytes());
        for value in values {
            self.string(value)?;
        }
        Ok(self)
    }

    pub fn bytes_vec(&mut self, values: &[Vec<u8>]) -> Result<&mut Self, UiRecordError> {
        if values.len() > UI_RECORD_MAX_VEC_ENTRIES {
            return Err(UiRecordError::LimitExceeded);
        }
        let len = u32::try_from(values.len()).map_err(|_| UiRecordError::LengthOverflow)?;
        self.bytes.extend_from_slice(&len.to_le_bytes());
        for value in values {
            self.bytes(value)?;
        }
        Ok(self)
    }

    pub fn bool(&mut self, value: bool) -> &mut Self {
        self.bytes.push(u8::from(value));
        self
    }

    pub fn array_32(&mut self, value: &[u8; 32]) -> &mut Self {
        self.bytes.extend_from_slice(value);
        self
    }

    pub fn u64(&mut self, value: u64) -> &mut Self {
        self.bytes.extend_from_slice(&value.to_le_bytes());
        self
    }
}

#[derive(Debug)]
pub struct UiRecordReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> UiRecordReader<'a> {
    pub fn new(bytes: &'a [u8], magic: &[u8]) -> Result<Self, UiRecordError> {
        if !bytes.starts_with(magic) {
            return Err(UiRecordError::WrongMagic);
        }
        Ok(Self {
            bytes,
            offset: magic.len(),
        })
    }

    pub fn finish(&self) -> Result<(), UiRecordError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(UiRecordError::TrailingBytes)
        }
    }

    pub fn read_bytes(&mut self) -> Result<Vec<u8>, UiRecordError> {
        let len = self.read_u32()? as usize;
        if len > UI_RECORD_MAX_FIELD_LEN {
            return Err(UiRecordError::LimitExceeded);
        }
        Ok(self.read_exact(len)?.to_vec())
    }

    pub fn read_string(&mut self) -> Result<String, UiRecordError> {
        String::from_utf8(self.read_bytes()?).map_err(|_| UiRecordError::InvalidUtf8)
    }

    pub fn read_string_vec(&mut self) -> Result<Vec<String>, UiRecordError> {
        let len = self.read_u32()? as usize;
        if len > UI_RECORD_MAX_VEC_ENTRIES {
            return Err(UiRecordError::LimitExceeded);
        }
        let mut out = Vec::with_capacity(len);
        for _ in 0..len {
            out.push(self.read_string()?);
        }
        Ok(out)
    }

    pub fn read_bytes_vec(&mut self) -> Result<Vec<Vec<u8>>, UiRecordError> {
        let len = self.read_u32()? as usize;
        if len > UI_RECORD_MAX_VEC_ENTRIES {
            return Err(UiRecordError::LimitExceeded);
        }
        let mut out = Vec::with_capacity(len);
        for _ in 0..len {
            out.push(self.read_bytes()?);
        }
        Ok(out)
    }

    pub fn read_bool(&mut self) -> Result<bool, UiRecordError> {
        match self.read_exact(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(UiRecordError::InvalidBool),
        }
    }

    pub fn read_array_32(&mut self) -> Result<[u8; 32], UiRecordError> {
        let bytes = self.read_exact(32)?;
        let mut out = [0u8; 32];
        out.copy_from_slice(bytes);
        Ok(out)
    }

    pub fn read_u64(&mut self) -> Result<u64, UiRecordError> {
        let bytes = self.read_exact(8)?;
        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_u32(&mut self) -> Result<u32, UiRecordError> {
        let bytes = self.read_exact(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8], UiRecordError> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or(UiRecordError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(UiRecordError::Truncated);
        }
        let out = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::String;
    use alloc::vec;

    #[test]
    fn record_codec_round_trips_values() {
        let mut writer = UiRecordWriter::new(b"TEST1");
        writer
            .string("slot")
            .unwrap()
            .bytes(&[1, 2, 3])
            .unwrap()
            .bytes_vec(&[vec![4, 5], vec![6]])
            .unwrap()
            .string_vec(&["a".into(), "b".into()])
            .unwrap()
            .bool(true)
            .array_32(&[7; 32])
            .u64(42);
        let bytes = writer.into_bytes();

        let mut reader = UiRecordReader::new(&bytes, b"TEST1").unwrap();
        assert_eq!(reader.read_string().unwrap(), "slot");
        assert_eq!(reader.read_bytes().unwrap(), vec![1, 2, 3]);
        assert_eq!(reader.read_bytes_vec().unwrap(), vec![vec![4, 5], vec![6]]);
        assert_eq!(
            reader.read_string_vec().unwrap(),
            vec![String::from("a"), String::from("b")]
        );
        assert!(reader.read_bool().unwrap());
        assert_eq!(reader.read_array_32().unwrap(), [7; 32]);
        assert_eq!(reader.read_u64().unwrap(), 42);
        assert_eq!(reader.finish(), Ok(()));
    }

    #[test]
    fn record_codec_rejects_bad_records() {
        assert_eq!(
            UiRecordReader::new(b"BAD", b"TEST1").unwrap_err(),
            UiRecordError::WrongMagic
        );

        let mut truncated = UiRecordWriter::new(b"TEST1");
        truncated.bytes(&[1, 2, 3]).unwrap();
        let mut bytes = truncated.into_bytes();
        bytes.pop();
        let mut reader = UiRecordReader::new(&bytes, b"TEST1").unwrap();
        assert_eq!(reader.read_bytes(), Err(UiRecordError::Truncated));

        let mut bool_record = UiRecordWriter::new(b"TEST1");
        bool_record.bytes.push(2);
        let bool_bytes = bool_record.into_bytes();
        let mut reader = UiRecordReader::new(&bool_bytes, b"TEST1").unwrap();
        assert_eq!(reader.read_bool(), Err(UiRecordError::InvalidBool));
    }

    #[test]
    fn record_codec_rejects_abusive_lengths_before_allocation() {
        let mut field = b"TEST1".to_vec();
        field.extend_from_slice(&((UI_RECORD_MAX_FIELD_LEN as u32) + 1).to_le_bytes());
        let mut reader = UiRecordReader::new(&field, b"TEST1").unwrap();
        assert_eq!(reader.read_bytes(), Err(UiRecordError::LimitExceeded));

        let mut vec_record = b"TEST1".to_vec();
        vec_record.extend_from_slice(&((UI_RECORD_MAX_VEC_ENTRIES as u32) + 1).to_le_bytes());
        let mut reader = UiRecordReader::new(&vec_record, b"TEST1").unwrap();
        assert_eq!(reader.read_string_vec(), Err(UiRecordError::LimitExceeded));
    }
}
