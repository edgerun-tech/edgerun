//! Decode Wayland messages from bytes.

use super::{Message, align4, MAX_MESSAGE_SIZE};

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
    pos: usize,
}

impl<'a> ArgCursor<'a> {
    /// Create a cursor over message args.
    pub fn new(msg: &'a Message) -> Self {
        Self {
            data: &msg.args,
            pos: 0,
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
