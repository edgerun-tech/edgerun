//! Network Block Device protocol constants and frame helpers.
//!
//! This module owns deterministic NBD byte layouts. It does not bind sockets,
//! attach Linux devices, or call into a block backend.

use alloc::string::String;
use alloc::vec::Vec;

use crate::block::{BlockDeviceInfo, BlockError};
use edgerun_encoding::byteorder::{read_u16_be, read_u32_be, read_u64_be};

pub const NBD_MAGIC: u64 = 0x4e42444d41474943;
pub const NBD_OPTS_MAGIC: u64 = 0x49484156454f5054;
pub const NBD_REQUEST_MAGIC: u32 = 0x2560_9513;
pub const NBD_REPLY_MAGIC: u32 = 0x6744_6698;
pub const NBD_REP_MAGIC: u64 = 0x0003_e889_0455_65a9;

pub const NBD_FLAG_FIXED_NEWSTYLE: u16 = 1;
pub const NBD_FLAG_HAS_FLAGS: u16 = 1;
pub const NBD_FLAG_READ_ONLY: u16 = 1 << 1;
pub const NBD_FLAG_SEND_FLUSH: u16 = 1 << 2;
pub const NBD_FLAG_SEND_TRIM: u16 = 1 << 5;
pub const NBD_FLAG_SEND_WRITE_ZEROES: u16 = 1 << 6;

pub const NBD_OPT_EXPORT_NAME: u32 = 1;
pub const NBD_OPT_ABORT: u32 = 2;
pub const NBD_OPT_LIST: u32 = 3;

pub const NBD_REP_ACK: u32 = 1;
pub const NBD_REP_SERVER: u32 = 2;

pub const NBD_CMD_READ: u16 = 0;
pub const NBD_CMD_WRITE: u16 = 1;
pub const NBD_CMD_DISC: u16 = 2;
pub const NBD_CMD_FLUSH: u16 = 3;
pub const NBD_CMD_TRIM: u16 = 4;
pub const NBD_CMD_WRITE_ZEROES: u16 = 6;

pub const NBD_OPTION_HEADER_LEN: usize = 16;
pub const NBD_REQUEST_HEADER_LEN: usize = 28;
pub const NBD_OPTION_REPLY_HEADER_LEN: usize = 20;
pub const NBD_REPLY_HEADER_LEN: usize = 16;
pub const NBD_EXPORT_INFO_PADDING_LEN: usize = 124;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NbdExport {
    pub name: String,
    pub description: String,
}

impl Default for NbdExport {
    fn default() -> Self {
        Self {
            name: "edgerun".into(),
            description: "edgerun virtual disk export".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NbdNegotiatedExport {
    pub size_bytes: u64,
    pub transmission_flags: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NbdOptionRequest {
    ExportName(Vec<u8>),
    Abort,
    List,
    Unsupported { option: u32, payload: Vec<u8> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NbdRequestHeader {
    pub flags: u16,
    pub command: u16,
    pub handle: u64,
    pub offset: u64,
    pub length: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NbdOptionHeader {
    pub option: u32,
    pub length: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NbdOptionReplyHeader {
    pub option: u32,
    pub reply_type: u32,
    pub length: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NbdReplyHeader {
    pub error: u32,
    pub handle: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NbdServerHandshake {
    pub handshake_flags: u16,
}

pub fn transmission_flags(info: &BlockDeviceInfo) -> u16 {
    let mut flags = NBD_FLAG_HAS_FLAGS;
    if info.supports_flush {
        flags |= NBD_FLAG_SEND_FLUSH;
    }
    if info.supports_discard {
        flags |= NBD_FLAG_SEND_TRIM;
    }
    if info.supports_write_zeroes {
        flags |= NBD_FLAG_SEND_WRITE_ZEROES;
    }
    flags
}

pub fn map_nbd_error(error: &BlockError) -> u32 {
    match error {
        BlockError::ReadOnly => 30,
        BlockError::OutOfRange => 22,
        BlockError::Misaligned => 22,
        BlockError::Unsupported => 95,
        BlockError::Timeout => 110,
        BlockError::NotReady => 11,
        BlockError::BackendFailure(_) | BlockError::ProtocolError(_) => 5,
    }
}

pub fn encode_server_handshake() -> Vec<u8> {
    let mut out = Vec::with_capacity(18);
    out.extend_from_slice(&NBD_MAGIC.to_be_bytes());
    out.extend_from_slice(&NBD_OPTS_MAGIC.to_be_bytes());
    out.extend_from_slice(&NBD_FLAG_FIXED_NEWSTYLE.to_be_bytes());
    out
}

pub fn encode_client_flags(flags: u32) -> Vec<u8> {
    flags.to_be_bytes().to_vec()
}

pub fn encode_option_request(option: u32, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(16 + payload.len());
    out.extend_from_slice(&NBD_OPTS_MAGIC.to_be_bytes());
    out.extend_from_slice(&option.to_be_bytes());
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    out.extend_from_slice(payload);
    out
}

pub fn encode_export_info(info: &BlockDeviceInfo) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + 2 + NBD_EXPORT_INFO_PADDING_LEN);
    out.extend_from_slice(&info.total_size_bytes().to_be_bytes());
    out.extend_from_slice(&transmission_flags(info).to_be_bytes());
    out.extend_from_slice(&[0_u8; NBD_EXPORT_INFO_PADDING_LEN]);
    out
}

pub fn decode_server_handshake(bytes: &[u8]) -> Result<NbdServerHandshake, BlockError> {
    if bytes.len() < 18 {
        return Err(BlockError::ProtocolError(
            "short NBD server handshake".into(),
        ));
    }
    if read_u64_be(bytes, 0) != NBD_MAGIC {
        return Err(BlockError::ProtocolError("invalid NBD magic".into()));
    }
    if read_u64_be(bytes, 8) != NBD_OPTS_MAGIC {
        return Err(BlockError::ProtocolError(
            "invalid NBD options magic".into(),
        ));
    }
    Ok(NbdServerHandshake {
        handshake_flags: read_u16_be(bytes, 16),
    })
}

pub fn decode_export_info(bytes: &[u8]) -> Result<NbdNegotiatedExport, BlockError> {
    let expected_len = 8 + 2 + NBD_EXPORT_INFO_PADDING_LEN;
    if bytes.len() < expected_len {
        return Err(BlockError::ProtocolError("short NBD export info".into()));
    }
    Ok(NbdNegotiatedExport {
        size_bytes: read_u64_be(bytes, 0),
        transmission_flags: read_u16_be(bytes, 8),
    })
}

pub fn encode_option_reply(option: u32, reply_type: u32, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(20 + payload.len());
    out.extend_from_slice(&NBD_REP_MAGIC.to_be_bytes());
    out.extend_from_slice(&option.to_be_bytes());
    out.extend_from_slice(&reply_type.to_be_bytes());
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    out.extend_from_slice(payload);
    out
}

pub fn encode_request_header(command: u16, handle: u64, offset: u64, length: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(28);
    out.extend_from_slice(&NBD_REQUEST_MAGIC.to_be_bytes());
    out.extend_from_slice(&0_u16.to_be_bytes());
    out.extend_from_slice(&command.to_be_bytes());
    out.extend_from_slice(&handle.to_be_bytes());
    out.extend_from_slice(&offset.to_be_bytes());
    out.extend_from_slice(&length.to_be_bytes());
    out
}

pub fn encode_simple_reply(handle: u64, error: u32, payload: Option<&[u8]>) -> Vec<u8> {
    let payload_len = payload.map_or(0, <[u8]>::len);
    let mut out = Vec::with_capacity(16 + payload_len);
    out.extend_from_slice(&NBD_REPLY_MAGIC.to_be_bytes());
    out.extend_from_slice(&error.to_be_bytes());
    out.extend_from_slice(&handle.to_be_bytes());
    if let Some(payload) = payload {
        out.extend_from_slice(payload);
    }
    out
}

pub fn decode_option_request(option: u32, payload: Vec<u8>) -> NbdOptionRequest {
    match option {
        NBD_OPT_EXPORT_NAME => NbdOptionRequest::ExportName(payload),
        NBD_OPT_ABORT => NbdOptionRequest::Abort,
        NBD_OPT_LIST => NbdOptionRequest::List,
        _ => NbdOptionRequest::Unsupported { option, payload },
    }
}

pub fn decode_option_header(bytes: &[u8]) -> Result<NbdOptionHeader, BlockError> {
    if bytes.len() < NBD_OPTION_HEADER_LEN {
        return Err(BlockError::ProtocolError("short NBD option header".into()));
    }
    if read_u64_be(bytes, 0) != NBD_OPTS_MAGIC {
        return Err(BlockError::ProtocolError("invalid NBD option magic".into()));
    }
    Ok(NbdOptionHeader {
        option: read_u32_be(bytes, 8),
        length: read_u32_be(bytes, 12),
    })
}

pub fn decode_option_reply_header(bytes: &[u8]) -> Result<NbdOptionReplyHeader, BlockError> {
    if bytes.len() < NBD_OPTION_REPLY_HEADER_LEN {
        return Err(BlockError::ProtocolError(
            "short NBD option reply header".into(),
        ));
    }
    if read_u64_be(bytes, 0) != NBD_REP_MAGIC {
        return Err(BlockError::ProtocolError(
            "invalid NBD option reply magic".into(),
        ));
    }
    Ok(NbdOptionReplyHeader {
        option: read_u32_be(bytes, 8),
        reply_type: read_u32_be(bytes, 12),
        length: read_u32_be(bytes, 16),
    })
}

pub fn decode_request_header(bytes: &[u8]) -> Result<NbdRequestHeader, BlockError> {
    if bytes.len() < NBD_REQUEST_HEADER_LEN {
        return Err(BlockError::ProtocolError("short NBD request header".into()));
    }
    if read_u32_be(bytes, 0) != NBD_REQUEST_MAGIC {
        return Err(BlockError::ProtocolError(
            "invalid NBD request magic".into(),
        ));
    }
    Ok(NbdRequestHeader {
        flags: read_u16_be(bytes, 4),
        command: read_u16_be(bytes, 6),
        handle: read_u64_be(bytes, 8),
        offset: read_u64_be(bytes, 16),
        length: read_u32_be(bytes, 24),
    })
}

pub fn decode_reply_header(bytes: &[u8]) -> Result<NbdReplyHeader, BlockError> {
    if bytes.len() < NBD_REPLY_HEADER_LEN {
        return Err(BlockError::ProtocolError("short NBD reply header".into()));
    }
    if read_u32_be(bytes, 0) != NBD_REPLY_MAGIC {
        return Err(BlockError::ProtocolError("invalid NBD reply magic".into()));
    }
    Ok(NbdReplyHeader {
        error: read_u32_be(bytes, 4),
        handle: read_u64_be(bytes, 8),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_info() -> BlockDeviceInfo {
        BlockDeviceInfo {
            block_size: 512,
            block_count: 8,
            readonly: false,
            supports_flush: true,
            supports_discard: true,
            supports_write_zeroes: false,
            model: "test".into(),
            serial: "serial".into(),
        }
    }

    #[test]
    fn encodes_handshake() {
        let bytes = encode_server_handshake();
        assert_eq!(read_u64_be(&bytes, 0), NBD_MAGIC);
        assert_eq!(read_u64_be(&bytes, 8), NBD_OPTS_MAGIC);
        assert_eq!(read_u16_be(&bytes, 16), NBD_FLAG_FIXED_NEWSTYLE);
    }

    #[test]
    fn encodes_export_info() {
        let info = test_info();
        let bytes = encode_export_info(&info);
        let export = decode_export_info(&bytes).unwrap();
        assert_eq!(export.size_bytes, 4096);
        assert_eq!(
            export.transmission_flags,
            NBD_FLAG_HAS_FLAGS | NBD_FLAG_SEND_FLUSH | NBD_FLAG_SEND_TRIM
        );
        assert_eq!(bytes.len(), 8 + 2 + NBD_EXPORT_INFO_PADDING_LEN);
    }

    #[test]
    fn decodes_server_handshake() {
        let bytes = encode_server_handshake();
        let handshake = decode_server_handshake(&bytes).unwrap();
        assert_eq!(handshake.handshake_flags, NBD_FLAG_FIXED_NEWSTYLE);
    }

    #[test]
    fn encodes_option_request() {
        let bytes = encode_option_request(NBD_OPT_EXPORT_NAME, b"disk");
        let header = decode_option_header(&bytes).unwrap();
        assert_eq!(header.option, NBD_OPT_EXPORT_NAME);
        assert_eq!(header.length, 4);
        assert_eq!(&bytes[16..], b"disk");
    }

    #[test]
    fn option_reply_header_roundtrips() {
        let bytes = encode_option_reply(NBD_OPT_LIST, NBD_REP_SERVER, b"alpha");
        let header = decode_option_reply_header(&bytes).unwrap();
        assert_eq!(header.option, NBD_OPT_LIST);
        assert_eq!(header.reply_type, NBD_REP_SERVER);
        assert_eq!(header.length, 5);
        assert_eq!(&bytes[NBD_OPTION_REPLY_HEADER_LEN..], b"alpha");
    }

    #[test]
    fn request_header_roundtrips() {
        let bytes = encode_request_header(NBD_CMD_READ, 7, 512, 1024);
        let header = decode_request_header(&bytes).unwrap();
        assert_eq!(header.command, NBD_CMD_READ);
        assert_eq!(header.handle, 7);
        assert_eq!(header.offset, 512);
        assert_eq!(header.length, 1024);
    }

    #[test]
    fn reply_header_roundtrips() {
        let bytes = encode_simple_reply(99, 5, Some(b"payload"));
        let header = decode_reply_header(&bytes).unwrap();
        assert_eq!(header.error, 5);
        assert_eq!(header.handle, 99);
        assert_eq!(&bytes[NBD_REPLY_HEADER_LEN..], b"payload");
    }
}
