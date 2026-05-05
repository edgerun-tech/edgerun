//! Decode Wayland messages from bytes.

use super::{align4, Message};

/// Decode errors.
#[derive(Debug)]
pub enum DecodeError {
    /// Message too short to contain header.
    ShortHeader,
    /// Message size exceeds maximum.
    TooLarge(usize),
    /// String argument out of bounds.
    StringOutOfBounds,
    /// Array argument out of bounds.
    ArrayOutOfBounds,
    /// Invalid UTF-8 in string argument.
    InvalidUtf8,
    /// Unexpected end of argument buffer.
    Truncated,
}

/// Decoded string argument.
#[derive(Debug)]
pub struct WireString(pub String);

/// Decoded array argument.
#[derive(Debug)]
pub struct WireArray(pub Vec<u8>);

/// Cursor over raw message args for typed decoding.
#[derive(Debug)]
pub struct ArgCursor<'a> {
    data: &'a [u8],
    fds: &'a [i32],
    pos: usize,
    sender_id: u32,
    opcode: u16,
    size: u16,
}

impl<'a> ArgCursor<'a> {
    /// Create a cursor over raw message bytes and fds.
    pub fn new(data: &'a [u8], fds: &'a [i32]) -> Self {
        let sender_id = if data.len() >= 8 {
            u32::from_le_bytes([data[0], data[1], data[2], data[3]])
        } else {
            0
        };
        let opcode = if data.len() >= 8 {
            u16::from_le_bytes([data[4], data[5]])
        } else {
            0
        };
        let size = if data.len() >= 8 {
            u16::from_le_bytes([data[6], data[7]])
        } else {
            0
        };
        Self {
            data,
            fds,
            pos: 8, // skip header
            sender_id,
            opcode,
            size,
        }
    }

    /// Create a cursor over message args.
    pub fn from_message(msg: &'a Message) -> Self {
        Self {
            data: &msg.args,
            fds: &msg.fds,
            pos: 0,
            sender_id: msg.sender_id,
            opcode: msg.opcode,
            size: msg.size,
        }
    }

    /// Create a cursor over just the args portion (header already parsed).
    pub fn from_args(args: &'a [u8], fds: &'a [i32]) -> Self {
        Self {
            data: args,
            fds,
            pos: 0,
            sender_id: 0,
            opcode: 0,
            size: 0,
        }
    }

    /// Get the sender_id from the message header.
    pub fn sender_id(&self) -> Option<u32> {
        if self.size > 0 {
            Some(self.sender_id)
        } else {
            None
        }
    }

    /// Get the opcode from the message header.
    pub fn opcode(&self) -> Option<u16> {
        if self.size > 0 {
            Some(self.opcode)
        } else {
            None
        }
    }

    /// Get the message size from the header.
    pub fn size(&self) -> Option<u16> {
        if self.size > 0 {
            Some(self.size)
        } else {
            None
        }
    }

    /// Read an int (i32).
    pub fn int(&mut self) -> Result<i32, DecodeError> {
        let bytes = self.read_bytes(4)?;
        Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// Read a uint (u32).
    pub fn uint(&mut self) -> Result<u32, DecodeError> {
        let bytes = self.read_bytes(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// Read a fixed-point value (i32, s15.16).
    pub fn fixed(&mut self) -> Result<i32, DecodeError> {
        self.int()
    }

    /// Read an object id (u32).
    pub fn object(&mut self) -> Result<u32, DecodeError> {
        self.uint()
    }

    /// Read a new_id (u32). On the server side, `new_id` is just a u32.
    pub fn new_id(&mut self) -> Result<u32, DecodeError> {
        self.uint()
    }

    /// Read a string argument. Returns `None` if the client sent -1 (null).
    pub fn string(&mut self) -> Result<Option<WireString>, DecodeError> {
        if self.pos + 4 > self.data.len() {
            return Err(DecodeError::Truncated);
        }
        let len = u32::from_le_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ]) as usize;
        self.pos += 4;

        // -1 means null
        if len == 0 {
            return Ok(None);
        }

        if self.pos + len > self.data.len() {
            return Err(DecodeError::StringOutOfBounds);
        }

        let str_bytes = &self.data[self.pos..self.pos + len - 1]; // exclude null
        self.pos = align4(self.pos + len);

        let s = std::str::from_utf8(str_bytes)
            .map_err(|_| DecodeError::InvalidUtf8)?
            .to_string();

        Ok(Some(WireString(s)))
    }

    /// Read an array argument.
    pub fn array(&mut self) -> Result<WireArrays, DecodeError> {
        if self.pos + 4 > self.data.len() {
            return Err(DecodeError::Truncated);
        }
        let len = u32::from_le_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ]) as usize;
        self.pos += 4;

        if self.pos + len > self.data.len() {
            return Err(DecodeError::ArrayOutOfBounds);
        }

        let data = self.data[self.pos..self.pos + len].to_vec();
        self.pos = align4(self.pos + len);

        Ok(WireArrays(data))
    }

    /// Read an fd argument (index into `msg.fds`).
    pub fn fd(&mut self) -> Result<i32, DecodeError> {
        // Fd arguments are encoded as 4-byte placeholder (always 0) in the data stream.
        // The actual fd is in msg.fds. We just skip 4 bytes here.
        self.read_bytes(4)?;
        // The caller should get the actual fd from msg.fds[self.fd_index]
        Ok(0)
    }

    fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        if self.pos + n > self.data.len() {
            return Err(DecodeError::Truncated);
        }
        let slice = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }
}

/// Decoded array data.
#[derive(Debug)]
pub struct WireArrays(pub Vec<u8>);

impl WireArrays {
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

/// Parse a single complete message from a byte buffer.
///
/// Returns the message and the number of bytes consumed.
/// FDs must be provided separately (they come from SCM_RIGHTS).
pub fn parse_message(data: &[u8], fds: Vec<i32>) -> Result<Option<(Message, usize)>, DecodeError> {
    if data.len() < 8 {
        return Ok(None); // need at least the header
    }

    let sender_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let opcode = u16::from_le_bytes([data[4], data[5]]);
    let size = u16::from_le_bytes([data[6], data[7]]) as usize;

    if size < 8 {
        return Err(DecodeError::ShortHeader);
    }
    if size > super::MAX_MESSAGE_SIZE {
        return Err(DecodeError::TooLarge(size));
    }
    if data.len() < size {
        return Ok(None); // message not complete yet
    }

    let args = data[8..size].to_vec();

    Ok(Some((
        Message {
            sender_id,
            opcode,
            size: size as u16,
            args,
            fds,
        },
        size,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::encode::*;

    #[test]
    fn test_decode_uint() {
        let msg = message_uint(42, 1, 0xDEADBEEF);
        let mut cursor = ArgCursor::from_message(&msg);
        assert_eq!(cursor.uint().unwrap(), 0xDEADBEEF);
    }

    #[test]
    fn test_decode_string() {
        let msg = message_uint_string(42, 1, 1, "hello");
        let mut cursor = ArgCursor::from_message(&msg);
        assert_eq!(cursor.uint().unwrap(), 1);
        let s = cursor.string().unwrap().unwrap();
        assert_eq!(s.0, "hello");
    }

    #[test]
    fn test_decode_null_string() {
        // Empty string (null)
        let mut buf = Vec::new();
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes()); // length 0 = null
        let msg = Message {
            sender_id: 42,
            opcode: 1,
            size: (8 + buf.len()) as u16,
            args: buf,
            fds: Vec::new(),
        };
        let mut cursor = ArgCursor::from_message(&msg);
        assert_eq!(cursor.uint().unwrap(), 1);
        assert!(cursor.string().unwrap().is_none());
    }

    #[test]
    fn test_decode_array() {
        let mut buf = Vec::new();
        let data = [10u8, 20, 30];
        encode_array(&mut buf, &data);
        let msg = Message {
            sender_id: 42,
            opcode: 1,
            size: (8 + buf.len()) as u16,
            args: buf,
            fds: Vec::new(),
        };
        let mut cursor = ArgCursor::from_message(&msg);
        let arr = cursor.array().unwrap();
        assert_eq!(arr.as_slice(), &[10, 20, 30]);
    }

    #[test]
    fn test_parse_message_roundtrip() {
        let original = message_uint2(42, 1, 0xAABBCCDD, 0x11223344);
        let encoded = encode(&original);

        let result = parse_message(&encoded, vec![]).unwrap();
        assert!(result.is_some());
        let (msg, consumed) = result.unwrap();
        assert_eq!(consumed, encoded.len());
        assert_eq!(msg.sender_id, 42);
        assert_eq!(msg.opcode, 1);
        assert_eq!(msg.size, 16);

        let mut cursor = ArgCursor::from_message(&msg);
        assert_eq!(cursor.uint().unwrap(), 0xAABBCCDD);
        assert_eq!(cursor.uint().unwrap(), 0x11223344);
    }

    #[test]
    fn test_parse_incomplete_message() {
        let msg = message_uint(42, 1, 0xDEADBEEF);
        let encoded = encode(&msg);
        // Only provide first 4 bytes (incomplete header)
        let result = parse_message(&encoded[..4], vec![]).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_decode_fixed() {
        let msg = message_uint_fixed(42, 1, 1, 0x10000); // 1.0 in s15.16
        let mut cursor = ArgCursor::from_message(&msg);
        assert_eq!(cursor.uint().unwrap(), 1);
        let fixed = cursor.fixed().unwrap();
        assert_eq!(fixed, 0x10000);
    }

    #[test]
    fn test_cursor_header_fields() {
        let msg = message_uint(42, 3, 0xDEADBEEF);
        let cursor = ArgCursor::from_message(&msg);
        assert_eq!(cursor.sender_id(), Some(42));
        assert_eq!(cursor.opcode(), Some(3));
        assert_eq!(cursor.size(), Some(12));
    }
}
