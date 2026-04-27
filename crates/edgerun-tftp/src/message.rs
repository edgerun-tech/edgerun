//! TFTP message parser/serializer — RFC 1350 + RFC 2347/2348.
//!
//! # Wire Format
//! ```text
//! RRQ/WRQ (op 1/2):
//!   2 bytes    string    1 byte    string   1 byte
//!   ------------------------------------------------
//!  | Opcode |  Filename  |   0   |   Mode   |   0   |  (options follow for RFC 2347)
//!   ------------------------------------------------
//!
//! DATA (op 3):
//!   2 bytes    2 bytes      n bytes
//!   ---------------------------------
//!  | Opcode |  Block #  |   Data    |
//!   ---------------------------------
//!
//! ACK (op 4):
//!   2 bytes    2 bytes
//!   ------------------
//!  | Opcode |  Block #  |
//!   ------------------
//!
//! ERROR (op 5):
//!   2 bytes    2 bytes      string    1 byte
//!   -----------------------------------------
//!  | Opcode |  ErrorCode |  ErrMsg   |   0   |
//!   -----------------------------------------
//!
//! OACK (op 6):
//!   2 bytes     string    1 byte    string   1 byte
//!   -------------------------------------------------
//!  | Opcode |   optname  |   0   |  optval  |   0   |  (repeated)
//!   -------------------------------------------------
//! ```

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::str::FromStr;

pub mod io {
    use alloc::string::{String, ToString};

    #[derive(Debug, Clone, Eq, PartialEq)]
    pub struct Error {
        kind: ErrorKind,
        message: String,
    }

    #[derive(Debug, Clone, Copy, Eq, PartialEq)]
    pub enum ErrorKind {
        InvalidData,
    }

    impl Error {
        pub fn new(kind: ErrorKind, message: impl ToString) -> Self {
            Self {
                kind,
                message: message.to_string(),
            }
        }

        pub fn kind(&self) -> ErrorKind {
            self.kind
        }
    }

    impl core::fmt::Display for Error {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str(&self.message)
        }
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default TFTP port.
pub const TFTP_PORT: u16 = 69;
/// Default block size.
pub const DEFAULT_BLKSIZE: u16 = 512;
/// Maximum block size per RFC 2348.
pub const MAX_BLKSIZE: u16 = 65464;
/// Default timeout (seconds).
pub const DEFAULT_TIMEOUT: u8 = 5;

// ---------------------------------------------------------------------------
// Opcodes
// ---------------------------------------------------------------------------

/// TFTP operation codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum TftpOpcode {
    /// Read Request
    RRQ = 1,
    /// Write Request
    WRQ = 2,
    /// Data
    DATA = 3,
    /// Acknowledgment
    ACK = 4,
    /// Error
    ERROR = 5,
    /// Option Acknowledgment (RFC 2347)
    OACK = 6,
}

impl TftpOpcode {
    pub fn from_u16(v: u16) -> Option<Self> {
        match v {
            1 => Some(Self::RRQ),
            2 => Some(Self::WRQ),
            3 => Some(Self::DATA),
            4 => Some(Self::ACK),
            5 => Some(Self::ERROR),
            6 => Some(Self::OACK),
            _ => None,
        }
    }

    pub fn as_u16(self) -> u16 {
        self as u16
    }
}

// ---------------------------------------------------------------------------
// TFTP Error Codes
// ---------------------------------------------------------------------------

/// TFTP error codes (RFC 1350).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum TftpError {
    /// Not defined, see error message (if any).
    NotDefined = 0,
    /// File not found.
    FileNotFound = 1,
    /// Access violation.
    AccessViolation = 2,
    /// Disk full or allocation exceeded.
    DiskFull = 3,
    /// Illegal TFTP operation.
    IllegalOperation = 4,
    /// Unknown transfer ID.
    UnknownTID = 5,
    /// File already exists.
    FileExists = 6,
    /// No such user.
    NoSuchUser = 7,
}

impl TftpError {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotDefined => "Not defined",
            Self::FileNotFound => "File not found",
            Self::AccessViolation => "Access violation",
            Self::DiskFull => "Disk full or allocation exceeded",
            Self::IllegalOperation => "Illegal TFTP operation",
            Self::UnknownTID => "Unknown transfer ID",
            Self::FileExists => "File already exists",
            Self::NoSuchUser => "No such user",
        }
    }
}

// ---------------------------------------------------------------------------
// Negotiated Options (RFC 2347/2348)
// ---------------------------------------------------------------------------

/// Negotiated TFTP transfer options.
#[derive(Debug, Clone)]
pub struct TftpOptions {
    /// Block size (default 512, max 65464 per RFC 2348).
    pub blksize: u16,
    /// Transfer size in bytes (client learns file size before transfer).
    pub tsize: Option<u64>,
    /// Timeout interval in seconds (default 5, RFC 2349).
    pub timeout: u8,
}

impl Default for TftpOptions {
    fn default() -> Self {
        Self {
            blksize: DEFAULT_BLKSIZE,
            tsize: None,
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

impl TftpOptions {
    /// Parse options from a client RRQ option section.
    pub fn parse_from_request(data: &[u8], offset: usize) -> (Self, BTreeMap<String, String>) {
        let mut opts = Self::default();
        let mut raw = BTreeMap::new();

        if offset >= data.len() {
            return (opts, raw);
        }

        let strings = parse_string_pairs(data, offset);
        for (key, value) in strings {
            let key_lower = key.to_lowercase();
            raw.insert(key.clone(), value.clone());
            match key_lower.as_str() {
                "blksize" => {
                    if let Ok(bs) = u16::from_str(&value) {
                        opts.blksize = bs.clamp(8, MAX_BLKSIZE);
                    }
                }
                "tsize" => {
                    // Client sends "0" to request tsize; server responds with actual size
                    if let Ok(size) = u64::from_str(&value) {
                        if size > 0 {
                            opts.tsize = Some(size);
                        }
                        // If size == 0, client is just requesting tsize info
                    }
                }
                "timeout" => {
                    if let Ok(t) = u8::from_str(&value) {
                        if t >= 1 {
                            opts.timeout = t;
                        }
                    }
                }
                _ => {}
            }
        }

        (opts, raw)
    }

    /// Serialize options to wire format (for OACK).
    pub fn to_wire(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        buf.extend_from_slice(b"blksize");
        buf.push(0);
        buf.extend_from_slice(self.blksize.to_string().as_bytes());
        buf.push(0);

        if let Some(size) = self.tsize {
            buf.extend_from_slice(b"tsize");
            buf.push(0);
            buf.extend_from_slice(size.to_string().as_bytes());
            buf.push(0);
        }

        buf.extend_from_slice(b"timeout");
        buf.push(0);
        buf.extend_from_slice(self.timeout.to_string().as_bytes());
        buf.push(0);

        buf
    }
}

// ---------------------------------------------------------------------------
// TFTP Messages
// ---------------------------------------------------------------------------

/// A TFTP message.
#[derive(Debug, Clone)]
pub enum TftpMessage {
    /// Read request from client.
    RRQ {
        filename: String,
        mode: String,
        options: TftpOptions,
        raw_options: BTreeMap<String, String>,
    },
    /// Write request from client (not supported for PXE, returns error).
    WRQ { filename: String, mode: String },
    /// Data block from server.
    DATA { block: u16, data: Vec<u8> },
    /// Acknowledgment from client.
    ACK { block: u16 },
    /// Error message.
    ERROR { code: TftpError, message: String },
    /// Option acknowledgment from server (RFC 2347).
    OACK { options: TftpOptions },
}

impl TftpMessage {
    /// Convenience: create an octet-mode read request.
    pub fn rrq(filename: &str) -> Self {
        Self::RRQ {
            filename: filename.to_string(),
            mode: "octet".to_string(),
            options: TftpOptions::default(),
            raw_options: BTreeMap::new(),
        }
    }

    /// Serialize this message to wire format.
    pub fn to_wire(&self) -> Vec<u8> {
        match self {
            Self::RRQ {
                filename,
                mode,
                options,
                raw_options,
            } => {
                let mut buf = Vec::new();
                buf.extend_from_slice(&1u16.to_be_bytes()); // RRQ opcode
                buf.extend_from_slice(filename.as_bytes());
                buf.push(0);
                buf.extend_from_slice(mode.as_bytes());
                buf.push(0);

                // Serialize options
                if let Some(tsize) = options.tsize {
                    buf.extend_from_slice(b"tsize");
                    buf.push(0);
                    buf.extend_from_slice(tsize.to_string().as_bytes());
                    buf.push(0);
                }
                buf.extend_from_slice(b"blksize");
                buf.push(0);
                buf.extend_from_slice(options.blksize.to_string().as_bytes());
                buf.push(0);
                buf.extend_from_slice(b"timeout");
                buf.push(0);
                buf.extend_from_slice(options.timeout.to_string().as_bytes());
                buf.push(0);

                for (key, value) in raw_options {
                    if !["blksize", "tsize", "timeout"].contains(&key.to_lowercase().as_str()) {
                        buf.extend_from_slice(key.as_bytes());
                        buf.push(0);
                        buf.extend_from_slice(value.as_bytes());
                        buf.push(0);
                    }
                }

                buf
            }

            Self::WRQ { filename, mode } => {
                let mut buf = Vec::new();
                buf.extend_from_slice(&2u16.to_be_bytes()); // WRQ opcode
                buf.extend_from_slice(filename.as_bytes());
                buf.push(0);
                buf.extend_from_slice(mode.as_bytes());
                buf.push(0);
                buf
            }

            Self::DATA { block, data } => {
                let mut buf = Vec::with_capacity(4 + data.len());
                buf.extend_from_slice(&3u16.to_be_bytes()); // DATA opcode
                buf.extend_from_slice(&block.to_be_bytes());
                buf.extend_from_slice(data);
                buf
            }

            Self::ACK { block } => {
                let mut buf = vec![0u8; 4];
                buf[0..2].copy_from_slice(&4u16.to_be_bytes()); // ACK opcode
                buf[2..4].copy_from_slice(&block.to_be_bytes());
                buf
            }

            Self::ERROR { code, message } => {
                let mut buf = Vec::with_capacity(4 + message.len() + 1);
                buf.extend_from_slice(&5u16.to_be_bytes()); // ERROR opcode
                buf.extend_from_slice(&(*code as u16).to_be_bytes());
                buf.extend_from_slice(message.as_bytes());
                buf.push(0);
                buf
            }

            Self::OACK { options } => {
                let mut buf = Vec::new();
                buf.extend_from_slice(&6u16.to_be_bytes()); // OACK opcode
                buf.extend_from_slice(&options.to_wire());
                buf
            }
        }
    }

    /// Parse a TFTP message from wire format.
    pub fn from_wire(data: &[u8]) -> Result<Self, io::Error> {
        if data.len() < 2 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "TFTP message too short",
            ));
        }

        let opcode = u16::from_be_bytes([data[0], data[1]]);

        match TftpOpcode::from_u16(opcode) {
            Some(TftpOpcode::RRQ) => {
                let (filename, after_filename) = read_string(data, 2)?;
                let mode_data = &data[after_filename..];
                let (mode, after_mode) = read_string(mode_data, 0)?;

                let (options, raw_options) = if after_mode < mode_data.len() {
                    TftpOptions::parse_from_request(mode_data, after_mode)
                } else {
                    (TftpOptions::default(), BTreeMap::new())
                };

                Ok(Self::RRQ {
                    filename,
                    mode,
                    options,
                    raw_options,
                })
            }

            Some(TftpOpcode::WRQ) => {
                let (filename, after_filename) = read_string(data, 2)?;
                let mode_data = &data[after_filename..];
                let (mode, _) = read_string(mode_data, 0)?;
                Ok(Self::WRQ { filename, mode })
            }

            Some(TftpOpcode::DATA) => {
                if data.len() < 4 {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "DATA too short"));
                }
                let block = u16::from_be_bytes([data[2], data[3]]);
                let d = data[4..].to_vec();
                Ok(Self::DATA { block, data: d })
            }

            Some(TftpOpcode::ACK) => {
                if data.len() < 4 {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "ACK too short"));
                }
                let block = u16::from_be_bytes([data[2], data[3]]);
                Ok(Self::ACK { block })
            }

            Some(TftpOpcode::ERROR) => {
                if data.len() < 4 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "ERROR too short",
                    ));
                }
                let code_val = u16::from_be_bytes([data[2], data[3]]);
                let code = TftpError::from_u16(code_val).unwrap_or(TftpError::NotDefined);
                let msg_end = data[4..]
                    .iter()
                    .position(|&b| b == 0)
                    .unwrap_or(data.len() - 4);
                let message = String::from_utf8_lossy(&data[4..4 + msg_end]).to_string();
                Ok(Self::ERROR { code, message })
            }

            Some(TftpOpcode::OACK) => {
                let (options, _) = TftpOptions::parse_from_request(data, 2);
                Ok(Self::OACK { options })
            }

            None => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unknown TFTP opcode: {}", opcode),
            )),
        }
    }

    /// Convenience: create an ERROR message.
    pub fn error(code: TftpError, message: &str) -> Self {
        Self::ERROR {
            code,
            message: message.to_string(),
        }
    }

    /// Convenience: create a DATA block.
    pub fn data(block: u16, data: Vec<u8>) -> Self {
        Self::DATA { block, data }
    }

    /// Convenience: create an ACK.
    pub fn ack(block: u16) -> Self {
        Self::ACK { block }
    }
}

impl TftpError {
    fn from_u16(v: u16) -> Option<Self> {
        match v {
            0 => Some(Self::NotDefined),
            1 => Some(Self::FileNotFound),
            2 => Some(Self::AccessViolation),
            3 => Some(Self::DiskFull),
            4 => Some(Self::IllegalOperation),
            5 => Some(Self::UnknownTID),
            6 => Some(Self::FileExists),
            7 => Some(Self::NoSuchUser),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn read_string(data: &[u8], offset: usize) -> Result<(String, usize), io::Error> {
    let end = data[offset..]
        .iter()
        .position(|&b| b == 0)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Unterminated string"))?;
    let s = String::from_utf8_lossy(&data[offset..offset + end]).to_string();
    Ok((s, offset + end + 1))
}

fn parse_string_pairs(data: &[u8], offset: usize) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let mut pos = offset;
    while pos < data.len() {
        if data[pos] == 0 {
            break; // trailing null
        }
        let name_end = match data[pos..].iter().position(|&b| b == 0) {
            Some(e) => pos + e,
            None => break,
        };
        let name = String::from_utf8_lossy(&data[pos..name_end]).to_string();
        pos = name_end + 1;

        if pos >= data.len() {
            break;
        }
        let val_end = match data[pos..].iter().position(|&b| b == 0) {
            Some(e) => pos + e,
            None => break,
        };
        let value = String::from_utf8_lossy(&data[pos..val_end]).to_string();
        pos = val_end + 1;

        pairs.push((name, value));
    }
    pairs
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rrq_parse() {
        // RRQ for "pxelinux.0" mode "octet"
        let mut data = vec![0, 1]; // RRQ opcode
        data.extend_from_slice(b"pxelinux.0");
        data.push(0);
        data.extend_from_slice(b"octet");
        data.push(0);

        let msg = TftpMessage::from_wire(&data).unwrap();
        match msg {
            TftpMessage::RRQ { filename, mode, .. } => {
                assert_eq!(filename, "pxelinux.0");
                assert_eq!(mode, "octet");
            }
            _ => panic!("Expected RRQ"),
        }
    }

    #[test]
    fn test_rrq_with_options() {
        let mut data = vec![0, 1];
        data.extend_from_slice(b"bootx64.efi");
        data.push(0);
        data.extend_from_slice(b"octet");
        data.push(0);
        // blksize 1456
        data.extend_from_slice(b"blksize");
        data.push(0);
        data.extend_from_slice(b"1456");
        data.push(0);
        // tsize 0 (client requests tsize)
        data.extend_from_slice(b"tsize");
        data.push(0);
        data.extend_from_slice(b"0");
        data.push(0);

        let msg = TftpMessage::from_wire(&data).unwrap();
        match msg {
            TftpMessage::RRQ { options, .. } => {
                assert_eq!(options.blksize, 1456);
            }
            _ => panic!("Expected RRQ"),
        }
    }

    #[test]
    fn test_data_roundtrip() {
        let msg = TftpMessage::data(1, vec![1, 2, 3, 4, 5]);
        let wire = msg.to_wire();
        let parsed = TftpMessage::from_wire(&wire).unwrap();
        match parsed {
            TftpMessage::DATA { block, data } => {
                assert_eq!(block, 1);
                assert_eq!(data, vec![1, 2, 3, 4, 5]);
            }
            _ => panic!("Expected DATA"),
        }
    }

    #[test]
    fn test_ack_roundtrip() {
        let msg = TftpMessage::ack(42);
        let wire = msg.to_wire();
        let parsed = TftpMessage::from_wire(&wire).unwrap();
        match parsed {
            TftpMessage::ACK { block } => {
                assert_eq!(block, 42);
            }
            _ => panic!("Expected ACK"),
        }
    }

    #[test]
    fn test_error_roundtrip() {
        let msg = TftpMessage::error(TftpError::FileNotFound, "No such file");
        let wire = msg.to_wire();
        let parsed = TftpMessage::from_wire(&wire).unwrap();
        match parsed {
            TftpMessage::ERROR { code, message } => {
                assert_eq!(code, TftpError::FileNotFound);
                assert_eq!(message, "No such file");
            }
            _ => panic!("Expected ERROR"),
        }
    }

    #[test]
    fn test_oack_roundtrip() {
        let opts = TftpOptions {
            blksize: 1456,
            tsize: Some(1048576),
            timeout: 3,
        };
        let msg = TftpMessage::OACK { options: opts };
        let wire = msg.to_wire();
        let parsed = TftpMessage::from_wire(&wire).unwrap();
        match parsed {
            TftpMessage::OACK { options } => {
                assert_eq!(options.blksize, 1456);
                assert_eq!(options.tsize, Some(1048576));
                assert_eq!(options.timeout, 3);
            }
            _ => panic!("Expected OACK"),
        }
    }

    #[test]
    fn test_data_wire_format() {
        let msg = TftpMessage::data(1, vec![0xAA, 0xBB]);
        let wire = msg.to_wire();
        // opcode(2) + block(2) + data(2) = 6 bytes
        assert_eq!(wire.len(), 6);
        assert_eq!(wire[0..2], [0, 3]); // DATA opcode
        assert_eq!(wire[2..4], [0, 1]); // block 1
        assert_eq!(wire[4..6], [0xAA, 0xBB]);
    }

    #[test]
    fn test_wrq_parse() {
        let mut data = vec![0, 2]; // WRQ
        data.extend_from_slice(b"upload.bin");
        data.push(0);
        data.extend_from_slice(b"octet");
        data.push(0);

        let msg = TftpMessage::from_wire(&data).unwrap();
        match msg {
            TftpMessage::WRQ { filename, mode } => {
                assert_eq!(filename, "upload.bin");
                assert_eq!(mode, "octet");
            }
            _ => panic!("Expected WRQ"),
        }
    }

    #[test]
    fn test_parse_string_pairs() {
        let mut data = Vec::new();
        data.extend_from_slice(b"blksize");
        data.push(0);
        data.extend_from_slice(b"1456");
        data.push(0);
        data.extend_from_slice(b"tsize");
        data.push(0);
        data.extend_from_slice(b"0");
        data.push(0);

        let pairs = parse_string_pairs(&data, 0);
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], ("blksize".to_string(), "1456".to_string()));
        assert_eq!(pairs[1], ("tsize".to_string(), "0".to_string()));
    }

    #[test]
    fn test_opcode_from_u16() {
        assert_eq!(TftpOpcode::from_u16(1), Some(TftpOpcode::RRQ));
        assert_eq!(TftpOpcode::from_u16(6), Some(TftpOpcode::OACK));
        assert_eq!(TftpOpcode::from_u16(99), None);
    }

    #[test]
    fn test_error_from_u16() {
        assert_eq!(TftpError::from_u16(1), Some(TftpError::FileNotFound));
        assert_eq!(TftpError::from_u16(5), Some(TftpError::UnknownTID));
        assert_eq!(TftpError::from_u16(99), None);
    }
}
