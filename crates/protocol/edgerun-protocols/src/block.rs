//! Transport-independent remote block device protocol.
//!
//! This module owns block request/response types, rkyv payload encoding,
//! frame boundaries, range validation, and backend-driven request handling.
//! It does not bind sockets, open files, spawn tasks, or decide where a block
//! device lives.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::{fmt, write};

use crate::wire as edgerun_wire;
use crate::wire::{
    RemoteBlockDeviceInfo, RemoteBlockError, RemoteBlockRequest, RemoteBlockResponse, WireError,
};
use edgerun_encoding::byteorder::{push_u32_le, read_u32_le};

pub const BLOCK_PROTOCOL_VERSION: u16 = 1;
pub const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;
pub const BLOCK_FRAME_HEADER_LEN: usize = 4;
pub const MAX_ENCODED_FRAME_SIZE: usize = BLOCK_FRAME_HEADER_LEN + MAX_FRAME_SIZE;

pub type RequestId = u64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockDeviceInfo {
    pub block_size: u32,
    pub block_count: u64,
    pub readonly: bool,
    pub supports_flush: bool,
    pub supports_discard: bool,
    pub supports_write_zeroes: bool,
    pub model: String,
    pub serial: String,
}

impl BlockDeviceInfo {
    pub fn total_size_bytes(&self) -> u64 {
        self.block_count * u64::from(self.block_size)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockError {
    OutOfRange,
    ReadOnly,
    Misaligned,
    Unsupported,
    BackendFailure(String),
    ProtocolError(String),
    NotReady,
    Timeout,
}

impl fmt::Display for BlockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfRange => f.write_str("block request out of range"),
            Self::ReadOnly => f.write_str("block device is read-only"),
            Self::Misaligned => f.write_str("block request is misaligned"),
            Self::Unsupported => f.write_str("operation unsupported"),
            Self::BackendFailure(message) => write!(f, "backend failure: {message}"),
            Self::ProtocolError(message) => write!(f, "protocol error: {message}"),
            Self::NotReady => f.write_str("backend not ready"),
            Self::Timeout => f.write_str("operation timed out"),
        }
    }
}

impl core::error::Error for BlockError {}

impl<B: BlockBackend + ?Sized> BlockBackend for Arc<B> {
    fn info(&self) -> BlockDeviceInfo {
        self.as_ref().info()
    }

    fn read_blocks(&self, lba: u64, blocks: u32, out: &mut [u8]) -> Result<(), BlockError> {
        self.as_ref().read_blocks(lba, blocks, out)
    }

    fn write_blocks(&self, lba: u64, blocks: u32, data: &[u8]) -> Result<(), BlockError> {
        self.as_ref().write_blocks(lba, blocks, data)
    }

    fn flush(&self) -> Result<(), BlockError> {
        self.as_ref().flush()
    }

    fn discard_blocks(&self, lba: u64, blocks: u32) -> Result<(), BlockError> {
        self.as_ref().discard_blocks(lba, blocks)
    }

    fn write_zeroes(&self, lba: u64, blocks: u32) -> Result<(), BlockError> {
        self.as_ref().write_zeroes(lba, blocks)
    }
}

pub trait BlockBackend {
    fn info(&self) -> BlockDeviceInfo;
    fn read_blocks(&self, lba: u64, blocks: u32, out: &mut [u8]) -> Result<(), BlockError>;
    fn write_blocks(&self, lba: u64, blocks: u32, data: &[u8]) -> Result<(), BlockError>;
    fn flush(&self) -> Result<(), BlockError>;

    fn discard_blocks(&self, _lba: u64, _blocks: u32) -> Result<(), BlockError> {
        Err(BlockError::Unsupported)
    }

    fn write_zeroes(&self, _lba: u64, _blocks: u32) -> Result<(), BlockError> {
        Err(BlockError::Unsupported)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockRequest {
    Handshake {
        protocol_version: u16,
    },
    GetInfo,
    Read {
        request_id: RequestId,
        lba: u64,
        blocks: u32,
    },
    Write {
        request_id: RequestId,
        lba: u64,
        blocks: u32,
        data: Vec<u8>,
    },
    Flush {
        request_id: RequestId,
    },
    Discard {
        request_id: RequestId,
        lba: u64,
        blocks: u32,
    },
    WriteZeroes {
        request_id: RequestId,
        lba: u64,
        blocks: u32,
    },
    Ping,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockResponse {
    HandshakeAck {
        protocol_version: u16,
    },
    Info(BlockDeviceInfo),
    ReadResult {
        request_id: RequestId,
        data: Vec<u8>,
    },
    WriteAck {
        request_id: RequestId,
    },
    FlushAck {
        request_id: RequestId,
    },
    DiscardAck {
        request_id: RequestId,
    },
    WriteZeroesAck {
        request_id: RequestId,
    },
    Pong,
    Error {
        request_id: Option<RequestId>,
        error: BlockError,
    },
}

pub fn handle_request<B: BlockBackend + ?Sized>(
    backend: &B,
    request: BlockRequest,
) -> BlockResponse {
    match request {
        BlockRequest::Handshake { protocol_version } => {
            if protocol_version == BLOCK_PROTOCOL_VERSION {
                BlockResponse::HandshakeAck {
                    protocol_version: BLOCK_PROTOCOL_VERSION,
                }
            } else {
                BlockResponse::Error {
                    request_id: None,
                    error: BlockError::ProtocolError(format!(
                        "unsupported protocol version {protocol_version}"
                    )),
                }
            }
        }
        BlockRequest::GetInfo => BlockResponse::Info(backend.info()),
        BlockRequest::Read {
            request_id,
            lba,
            blocks,
        } => {
            let info = backend.info();
            match checked_len_bytes(&info, blocks) {
                Ok(len) => {
                    let mut data = vec![0_u8; len];
                    match backend.read_blocks(lba, blocks, &mut data) {
                        Ok(()) => BlockResponse::ReadResult { request_id, data },
                        Err(error) => BlockResponse::Error {
                            request_id: Some(request_id),
                            error,
                        },
                    }
                }
                Err(error) => BlockResponse::Error {
                    request_id: Some(request_id),
                    error,
                },
            }
        }
        BlockRequest::Write {
            request_id,
            lba,
            blocks,
            data,
        } => match backend.write_blocks(lba, blocks, &data) {
            Ok(()) => BlockResponse::WriteAck { request_id },
            Err(error) => BlockResponse::Error {
                request_id: Some(request_id),
                error,
            },
        },
        BlockRequest::Flush { request_id } => match backend.flush() {
            Ok(()) => BlockResponse::FlushAck { request_id },
            Err(error) => BlockResponse::Error {
                request_id: Some(request_id),
                error,
            },
        },
        BlockRequest::Discard {
            request_id,
            lba,
            blocks,
        } => match backend.discard_blocks(lba, blocks) {
            Ok(()) => BlockResponse::DiscardAck { request_id },
            Err(error) => BlockResponse::Error {
                request_id: Some(request_id),
                error,
            },
        },
        BlockRequest::WriteZeroes {
            request_id,
            lba,
            blocks,
        } => match backend.write_zeroes(lba, blocks) {
            Ok(()) => BlockResponse::WriteZeroesAck { request_id },
            Err(error) => BlockResponse::Error {
                request_id: Some(request_id),
                error,
            },
        },
        BlockRequest::Ping => BlockResponse::Pong,
    }
}

pub fn expect_handshake_response(response: BlockResponse) -> Result<(), BlockError> {
    match response {
        BlockResponse::HandshakeAck { protocol_version }
            if protocol_version == BLOCK_PROTOCOL_VERSION =>
        {
            Ok(())
        }
        response => Err(unexpected_response("handshake", response)),
    }
}

pub fn expect_info_response(response: BlockResponse) -> Result<BlockDeviceInfo, BlockError> {
    match response {
        BlockResponse::Info(info) => Ok(info),
        response => Err(unexpected_response("info", response)),
    }
}

pub fn expect_pong_response(response: BlockResponse) -> Result<(), BlockError> {
    match response {
        BlockResponse::Pong => Ok(()),
        response => Err(unexpected_response("ping", response)),
    }
}

pub fn expect_read_response(
    response: BlockResponse,
    request_id: RequestId,
) -> Result<Vec<u8>, BlockError> {
    match response {
        BlockResponse::ReadResult {
            request_id: response_id,
            data,
        } if response_id == request_id => Ok(data),
        BlockResponse::Error {
            request_id: Some(response_id),
            error,
        } if response_id == request_id => Err(error),
        response => Err(unexpected_response("read", response)),
    }
}

pub fn expect_write_response(
    response: BlockResponse,
    request_id: RequestId,
) -> Result<(), BlockError> {
    match response {
        BlockResponse::WriteAck {
            request_id: response_id,
        } if response_id == request_id => Ok(()),
        BlockResponse::Error {
            request_id: Some(response_id),
            error,
        } if response_id == request_id => Err(error),
        response => Err(unexpected_response("write", response)),
    }
}

pub fn expect_flush_response(
    response: BlockResponse,
    request_id: RequestId,
) -> Result<(), BlockError> {
    match response {
        BlockResponse::FlushAck {
            request_id: response_id,
        } if response_id == request_id => Ok(()),
        BlockResponse::Error {
            request_id: Some(response_id),
            error,
        } if response_id == request_id => Err(error),
        response => Err(unexpected_response("flush", response)),
    }
}

pub fn expect_discard_response(
    response: BlockResponse,
    request_id: RequestId,
) -> Result<(), BlockError> {
    match response {
        BlockResponse::DiscardAck {
            request_id: response_id,
        } if response_id == request_id => Ok(()),
        BlockResponse::Error {
            request_id: Some(response_id),
            error,
        } if response_id == request_id => Err(error),
        response => Err(unexpected_response("discard", response)),
    }
}

pub fn expect_write_zeroes_response(
    response: BlockResponse,
    request_id: RequestId,
) -> Result<(), BlockError> {
    match response {
        BlockResponse::WriteZeroesAck {
            request_id: response_id,
        } if response_id == request_id => Ok(()),
        BlockResponse::Error {
            request_id: Some(response_id),
            error,
        } if response_id == request_id => Err(error),
        response => Err(unexpected_response("write_zeroes", response)),
    }
}

pub fn encode_request_payload(request: &BlockRequest) -> Result<Vec<u8>, BlockError> {
    let wire = block_request_to_wire(request);
    Ok(edgerun_wire::to_bytes::<WireError>(&wire)
        .map_err(map_wire_error)?
        .into_vec())
}

pub fn decode_request_payload(payload: &[u8]) -> Result<BlockRequest, BlockError> {
    let owned = payload.to_vec();
    edgerun_wire::from_bytes::<RemoteBlockRequest, WireError>(&owned)
        .map(block_request_from_wire)
        .map_err(map_wire_error)
}

pub fn encode_response_payload(response: &BlockResponse) -> Result<Vec<u8>, BlockError> {
    let wire = block_response_to_wire(response);
    Ok(edgerun_wire::to_bytes::<WireError>(&wire)
        .map_err(map_wire_error)?
        .into_vec())
}

pub fn decode_response_payload(payload: &[u8]) -> Result<BlockResponse, BlockError> {
    let owned = payload.to_vec();
    edgerun_wire::from_bytes::<RemoteBlockResponse, WireError>(&owned)
        .map(block_response_from_wire)
        .map_err(map_wire_error)
}

pub fn encode_request_frame(request: &BlockRequest) -> Result<Vec<u8>, BlockError> {
    frame_payload(&encode_request_payload(request)?)
}

pub fn decode_request_frame(frame: &[u8]) -> Result<BlockRequest, BlockError> {
    decode_request_payload(&decode_frame(frame)?)
}

pub fn encode_response_frame(response: &BlockResponse) -> Result<Vec<u8>, BlockError> {
    frame_payload(&encode_response_payload(response)?)
}

pub fn decode_response_frame(frame: &[u8]) -> Result<BlockResponse, BlockError> {
    decode_response_payload(&decode_frame(frame)?)
}

pub fn frame_payload(payload: &[u8]) -> Result<Vec<u8>, BlockError> {
    let len = u32::try_from(payload.len())
        .map_err(|_| BlockError::ProtocolError("frame too large to encode".into()))?;
    let mut frame = Vec::with_capacity(BLOCK_FRAME_HEADER_LEN + payload.len());
    push_u32_le(&mut frame, len);
    frame.extend_from_slice(payload);
    Ok(frame)
}

pub fn decode_frame(frame: &[u8]) -> Result<Vec<u8>, BlockError> {
    let len = decode_frame_len(frame)?;
    let end = BLOCK_FRAME_HEADER_LEN
        .checked_add(len)
        .ok_or_else(|| BlockError::ProtocolError("frame length overflow".into()))?;
    if frame.len() != end {
        return Err(BlockError::ProtocolError("frame length mismatch".into()));
    }
    Ok(frame[BLOCK_FRAME_HEADER_LEN..].to_vec())
}

pub fn decode_frame_len(header: &[u8]) -> Result<usize, BlockError> {
    if header.len() < BLOCK_FRAME_HEADER_LEN {
        return Err(BlockError::ProtocolError("truncated frame header".into()));
    }
    let len = read_u32_le(header, 0) as usize;
    if len > MAX_FRAME_SIZE {
        return Err(BlockError::ProtocolError(
            "frame exceeds maximum size".into(),
        ));
    }
    Ok(len)
}

pub fn checked_len_bytes(info: &BlockDeviceInfo, blocks: u32) -> Result<usize, BlockError> {
    (blocks as usize)
        .checked_mul(info.block_size as usize)
        .ok_or_else(|| BlockError::ProtocolError("byte length overflow".into()))
}

pub fn validate_range(info: &BlockDeviceInfo, lba: u64, blocks: u32) -> Result<(), BlockError> {
    if blocks == 0 {
        return Err(BlockError::ProtocolError("block count must be > 0".into()));
    }
    let end = lba
        .checked_add(u64::from(blocks))
        .ok_or(BlockError::OutOfRange)?;
    if end > info.block_count {
        return Err(BlockError::OutOfRange);
    }
    Ok(())
}

pub fn validate_transfer(
    info: &BlockDeviceInfo,
    lba: u64,
    blocks: u32,
    actual_len: usize,
) -> Result<(), BlockError> {
    validate_range(info, lba, blocks)?;
    let expected_len = checked_len_bytes(info, blocks)?;
    if actual_len != expected_len {
        return Err(BlockError::Misaligned);
    }
    Ok(())
}

pub fn validate_device_info(info: &BlockDeviceInfo) -> Result<(), BlockError> {
    if info.block_size == 0 {
        return Err(BlockError::ProtocolError("block size must be > 0".into()));
    }
    if info.block_count == 0 {
        return Err(BlockError::ProtocolError("block count must be > 0".into()));
    }
    let _ = total_size_len(info)?;
    Ok(())
}

pub fn total_size_len(info: &BlockDeviceInfo) -> Result<usize, BlockError> {
    let bytes = info
        .block_count
        .checked_mul(u64::from(info.block_size))
        .ok_or_else(|| BlockError::ProtocolError("device size overflow".into()))?;
    usize::try_from(bytes).map_err(|_| BlockError::ProtocolError("device size too large".into()))
}

pub fn byte_offset(info: &BlockDeviceInfo, lba: u64) -> Result<u64, BlockError> {
    lba.checked_mul(u64::from(info.block_size))
        .ok_or_else(|| BlockError::ProtocolError("byte offset overflow".into()))
}

pub fn byte_range(
    info: &BlockDeviceInfo,
    lba: u64,
    blocks: u32,
) -> Result<core::ops::Range<usize>, BlockError> {
    validate_range(info, lba, blocks)?;
    let start = byte_offset(info, lba)?;
    let len = checked_len_bytes(info, blocks)?;
    let start = usize::try_from(start)
        .map_err(|_| BlockError::ProtocolError("byte offset too large".into()))?;
    let end = start
        .checked_add(len)
        .ok_or_else(|| BlockError::ProtocolError("byte range overflow".into()))?;
    Ok(start..end)
}

fn map_wire_error(error: WireError) -> BlockError {
    BlockError::ProtocolError(format!("rkyv wire error: {error}"))
}

fn unexpected_response(operation: &str, response: BlockResponse) -> BlockError {
    BlockError::ProtocolError(format!("unexpected {operation} response: {response:?}"))
}

fn block_device_info_to_wire(info: &BlockDeviceInfo) -> RemoteBlockDeviceInfo {
    RemoteBlockDeviceInfo {
        block_size: info.block_size,
        block_count: info.block_count,
        readonly: info.readonly,
        supports_flush: info.supports_flush,
        supports_discard: info.supports_discard,
        supports_write_zeroes: info.supports_write_zeroes,
        model: info.model.clone(),
        serial: info.serial.clone(),
    }
}

fn block_device_info_from_wire(info: RemoteBlockDeviceInfo) -> BlockDeviceInfo {
    BlockDeviceInfo {
        block_size: info.block_size,
        block_count: info.block_count,
        readonly: info.readonly,
        supports_flush: info.supports_flush,
        supports_discard: info.supports_discard,
        supports_write_zeroes: info.supports_write_zeroes,
        model: info.model,
        serial: info.serial,
    }
}

fn block_error_to_wire(error: &BlockError) -> RemoteBlockError {
    match error {
        BlockError::OutOfRange => RemoteBlockError::OutOfRange,
        BlockError::ReadOnly => RemoteBlockError::ReadOnly,
        BlockError::Misaligned => RemoteBlockError::Misaligned,
        BlockError::Unsupported => RemoteBlockError::Unsupported,
        BlockError::BackendFailure(message) => RemoteBlockError::BackendFailure(message.clone()),
        BlockError::ProtocolError(message) => RemoteBlockError::ProtocolError(message.clone()),
        BlockError::NotReady => RemoteBlockError::NotReady,
        BlockError::Timeout => RemoteBlockError::Timeout,
    }
}

fn block_error_from_wire(error: RemoteBlockError) -> BlockError {
    match error {
        RemoteBlockError::OutOfRange => BlockError::OutOfRange,
        RemoteBlockError::ReadOnly => BlockError::ReadOnly,
        RemoteBlockError::Misaligned => BlockError::Misaligned,
        RemoteBlockError::Unsupported => BlockError::Unsupported,
        RemoteBlockError::BackendFailure(message) => BlockError::BackendFailure(message),
        RemoteBlockError::ProtocolError(message) => BlockError::ProtocolError(message),
        RemoteBlockError::NotReady => BlockError::NotReady,
        RemoteBlockError::Timeout => BlockError::Timeout,
    }
}

fn block_request_to_wire(request: &BlockRequest) -> RemoteBlockRequest {
    match request {
        BlockRequest::Handshake { protocol_version } => RemoteBlockRequest::Handshake {
            protocol_version: *protocol_version,
        },
        BlockRequest::GetInfo => RemoteBlockRequest::GetInfo,
        BlockRequest::Read {
            request_id,
            lba,
            blocks,
        } => RemoteBlockRequest::Read {
            request_id: *request_id,
            lba: *lba,
            blocks: *blocks,
        },
        BlockRequest::Write {
            request_id,
            lba,
            blocks,
            data,
        } => RemoteBlockRequest::Write {
            request_id: *request_id,
            lba: *lba,
            blocks: *blocks,
            data: data.clone(),
        },
        BlockRequest::Flush { request_id } => RemoteBlockRequest::Flush {
            request_id: *request_id,
        },
        BlockRequest::Discard {
            request_id,
            lba,
            blocks,
        } => RemoteBlockRequest::Discard {
            request_id: *request_id,
            lba: *lba,
            blocks: *blocks,
        },
        BlockRequest::WriteZeroes {
            request_id,
            lba,
            blocks,
        } => RemoteBlockRequest::WriteZeroes {
            request_id: *request_id,
            lba: *lba,
            blocks: *blocks,
        },
        BlockRequest::Ping => RemoteBlockRequest::Ping,
    }
}

fn block_request_from_wire(request: RemoteBlockRequest) -> BlockRequest {
    match request {
        RemoteBlockRequest::Handshake { protocol_version } => {
            BlockRequest::Handshake { protocol_version }
        }
        RemoteBlockRequest::GetInfo => BlockRequest::GetInfo,
        RemoteBlockRequest::Read {
            request_id,
            lba,
            blocks,
        } => BlockRequest::Read {
            request_id,
            lba,
            blocks,
        },
        RemoteBlockRequest::Write {
            request_id,
            lba,
            blocks,
            data,
        } => BlockRequest::Write {
            request_id,
            lba,
            blocks,
            data,
        },
        RemoteBlockRequest::Flush { request_id } => BlockRequest::Flush { request_id },
        RemoteBlockRequest::Discard {
            request_id,
            lba,
            blocks,
        } => BlockRequest::Discard {
            request_id,
            lba,
            blocks,
        },
        RemoteBlockRequest::WriteZeroes {
            request_id,
            lba,
            blocks,
        } => BlockRequest::WriteZeroes {
            request_id,
            lba,
            blocks,
        },
        RemoteBlockRequest::Ping => BlockRequest::Ping,
    }
}

fn block_response_to_wire(response: &BlockResponse) -> RemoteBlockResponse {
    match response {
        BlockResponse::HandshakeAck { protocol_version } => RemoteBlockResponse::HandshakeAck {
            protocol_version: *protocol_version,
        },
        BlockResponse::Info(info) => RemoteBlockResponse::Info(block_device_info_to_wire(info)),
        BlockResponse::ReadResult { request_id, data } => RemoteBlockResponse::ReadResult {
            request_id: *request_id,
            data: data.clone(),
        },
        BlockResponse::WriteAck { request_id } => RemoteBlockResponse::WriteAck {
            request_id: *request_id,
        },
        BlockResponse::FlushAck { request_id } => RemoteBlockResponse::FlushAck {
            request_id: *request_id,
        },
        BlockResponse::DiscardAck { request_id } => RemoteBlockResponse::DiscardAck {
            request_id: *request_id,
        },
        BlockResponse::WriteZeroesAck { request_id } => RemoteBlockResponse::WriteZeroesAck {
            request_id: *request_id,
        },
        BlockResponse::Pong => RemoteBlockResponse::Pong,
        BlockResponse::Error { request_id, error } => RemoteBlockResponse::Error {
            request_id: *request_id,
            error: block_error_to_wire(error),
        },
    }
}

fn block_response_from_wire(response: RemoteBlockResponse) -> BlockResponse {
    match response {
        RemoteBlockResponse::HandshakeAck { protocol_version } => {
            BlockResponse::HandshakeAck { protocol_version }
        }
        RemoteBlockResponse::Info(info) => BlockResponse::Info(block_device_info_from_wire(info)),
        RemoteBlockResponse::ReadResult { request_id, data } => {
            BlockResponse::ReadResult { request_id, data }
        }
        RemoteBlockResponse::WriteAck { request_id } => BlockResponse::WriteAck { request_id },
        RemoteBlockResponse::FlushAck { request_id } => BlockResponse::FlushAck { request_id },
        RemoteBlockResponse::DiscardAck { request_id } => BlockResponse::DiscardAck { request_id },
        RemoteBlockResponse::WriteZeroesAck { request_id } => {
            BlockResponse::WriteZeroesAck { request_id }
        }
        RemoteBlockResponse::Pong => BlockResponse::Pong,
        RemoteBlockResponse::Error { request_id, error } => BlockResponse::Error {
            request_id,
            error: block_error_from_wire(error),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestBackend;

    impl BlockBackend for TestBackend {
        fn info(&self) -> BlockDeviceInfo {
            BlockDeviceInfo {
                block_size: 512,
                block_count: 8,
                readonly: false,
                supports_flush: true,
                supports_discard: true,
                supports_write_zeroes: true,
                model: "test".into(),
                serial: "serial".into(),
            }
        }

        fn read_blocks(&self, _lba: u64, _blocks: u32, out: &mut [u8]) -> Result<(), BlockError> {
            out.fill(0xaa);
            Ok(())
        }

        fn write_blocks(&self, _lba: u64, _blocks: u32, _data: &[u8]) -> Result<(), BlockError> {
            Ok(())
        }

        fn flush(&self) -> Result<(), BlockError> {
            Ok(())
        }
    }

    #[test]
    fn request_frame_roundtrips() {
        let request = BlockRequest::Read {
            request_id: 7,
            lba: 1,
            blocks: 2,
        };
        let frame = encode_request_frame(&request).unwrap();
        assert_eq!(decode_request_frame(&frame).unwrap(), request);
    }

    #[test]
    fn response_frame_roundtrips() {
        let response = BlockResponse::Info(TestBackend.info());
        let frame = encode_response_frame(&response).unwrap();
        assert_eq!(decode_response_frame(&frame).unwrap(), response);
    }

    #[test]
    fn frame_len_decodes_header() {
        let frame = frame_payload(&[1, 2, 3]).unwrap();
        assert_eq!(
            decode_frame_len(&frame[..BLOCK_FRAME_HEADER_LEN]).unwrap(),
            3
        );
    }

    #[test]
    fn encoded_frame_max_includes_header() {
        assert_eq!(
            MAX_ENCODED_FRAME_SIZE,
            MAX_FRAME_SIZE + BLOCK_FRAME_HEADER_LEN
        );
    }

    #[test]
    fn handle_request_reads_from_backend() {
        let response = handle_request(
            &TestBackend,
            BlockRequest::Read {
                request_id: 3,
                lba: 0,
                blocks: 1,
            },
        );
        match response {
            BlockResponse::ReadResult { request_id, data } => {
                assert_eq!(request_id, 3);
                assert_eq!(data.len(), 512);
                assert!(data.iter().all(|byte| *byte == 0xaa));
            }
            other => panic!("unexpected response: {other:?}"),
        }
    }

    #[test]
    fn expect_read_response_accepts_matching_id() {
        let data = expect_read_response(
            BlockResponse::ReadResult {
                request_id: 9,
                data: vec![1, 2, 3],
            },
            9,
        )
        .unwrap();
        assert_eq!(data, vec![1, 2, 3]);
    }

    #[test]
    fn expect_read_response_rejects_wrong_id() {
        assert!(matches!(
            expect_read_response(
                BlockResponse::ReadResult {
                    request_id: 10,
                    data: vec![]
                },
                9
            ),
            Err(BlockError::ProtocolError(_))
        ));
    }

    #[test]
    fn expect_write_response_propagates_matching_error() {
        assert_eq!(
            expect_write_response(
                BlockResponse::Error {
                    request_id: Some(4),
                    error: BlockError::ReadOnly,
                },
                4,
            ),
            Err(BlockError::ReadOnly)
        );
    }
}
