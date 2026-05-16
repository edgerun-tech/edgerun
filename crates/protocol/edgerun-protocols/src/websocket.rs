//! WebSocket frame helpers for adapters that tunnel protocol frames.
//!
//! This module owns deterministic frame classification and server-to-client
//! frame encoding. It does not read sockets or perform HTTP upgrade handling.

use alloc::string::FromUtf8Error;
use alloc::vec::Vec;
use alloc::{format, string::String};
use core::fmt;

use edgerun_crypto::sha1::{Digest, Sha1};
use edgerun_encoding::base64::standard_encode;
use edgerun_encoding::byteorder::{push_u16_be, push_u64_be, read_u16_be, read_u64_be};

pub const WS_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
pub const WS_BLOCK_PROTOCOL: &str = "edgerun-block-v1";
pub const WS_OPCODE_BINARY: u8 = 0x2;
pub const WS_OPCODE_CLOSE: u8 = 0x8;
pub const WS_OPCODE_PING: u8 = 0x9;
pub const WS_OPCODE_PONG: u8 = 0xA;
pub const WS_MAX_CONTROL_PAYLOAD_LEN: usize = 125;
pub const WS_MAX_HANDSHAKE_LEN: usize = 8192;
pub const WS_HTTP_HEADER_TERMINATOR: &[u8] = b"\r\n\r\n";
pub const WS_MASK_LEN: usize = 4;
pub const WS_BASE_HEADER_LEN: usize = 2;
pub const WS_EXTENDED_16_LEN: usize = 2;
pub const WS_EXTENDED_64_LEN: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WebSocketMessage {
    Binary(Vec<u8>),
    Ping(Vec<u8>),
    Pong,
    Close,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WebSocketError {
    TruncatedHeader,
    Fragmented,
    UnmaskedClientFrame,
    MissingKey,
    NotUpgrade,
    HandshakeTooLarge,
    InvalidUtf8,
    ControlPayloadTooLarge,
    UnsupportedOpcode,
    MessageTooLarge,
}

impl fmt::Display for WebSocketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TruncatedHeader => f.write_str("truncated websocket frame header"),
            Self::Fragmented => f.write_str("fragmented websocket frames are unsupported"),
            Self::UnmaskedClientFrame => f.write_str("client websocket frames must be masked"),
            Self::MissingKey => f.write_str("missing Sec-WebSocket-Key"),
            Self::NotUpgrade => f.write_str("request is not a websocket upgrade"),
            Self::HandshakeTooLarge => {
                f.write_str("websocket handshake exceeds maximum header size")
            }
            Self::InvalidUtf8 => f.write_str("websocket handshake is not valid UTF-8"),
            Self::ControlPayloadTooLarge => f.write_str("websocket control payload too large"),
            Self::UnsupportedOpcode => f.write_str("unsupported websocket opcode"),
            Self::MessageTooLarge => f.write_str("websocket message too large"),
        }
    }
}

impl core::error::Error for WebSocketError {}

impl From<FromUtf8Error> for WebSocketError {
    fn from(_value: FromUtf8Error) -> Self {
        Self::InvalidUtf8
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WebSocketFramePrefix {
    pub fin: bool,
    pub opcode: u8,
    pub masked: bool,
    pub payload_len_code: u8,
}

impl WebSocketFramePrefix {
    pub fn extended_len_bytes(self) -> usize {
        match self.payload_len_code {
            126 => WS_EXTENDED_16_LEN,
            127 => WS_EXTENDED_64_LEN,
            _ => 0,
        }
    }

    pub fn require_client_mask(self) -> Result<Self, WebSocketError> {
        if self.masked {
            Ok(self)
        } else {
            Err(WebSocketError::UnmaskedClientFrame)
        }
    }
}

pub fn decode_frame_prefix(header: &[u8]) -> Result<WebSocketFramePrefix, WebSocketError> {
    if header.len() < WS_BASE_HEADER_LEN {
        return Err(WebSocketError::TruncatedHeader);
    }
    Ok(WebSocketFramePrefix {
        fin: header[0] & 0x80 != 0,
        opcode: header[0] & 0x0f,
        masked: header[1] & 0x80 != 0,
        payload_len_code: header[1] & 0x7f,
    })
}

pub fn decode_payload_len(
    prefix: WebSocketFramePrefix,
    extended: &[u8],
    max_len: usize,
) -> Result<usize, WebSocketError> {
    let len = match prefix.payload_len_code {
        len @ 0..=125 => u64::from(len),
        126 => {
            if extended.len() < WS_EXTENDED_16_LEN {
                return Err(WebSocketError::TruncatedHeader);
            }
            u64::from(read_u16_be(extended, 0))
        }
        127 => {
            if extended.len() < WS_EXTENDED_64_LEN {
                return Err(WebSocketError::TruncatedHeader);
            }
            read_u64_be(extended, 0)
        }
        _ => unreachable!(),
    };
    let len = usize::try_from(len).map_err(|_| WebSocketError::MessageTooLarge)?;
    if len > max_len {
        return Err(WebSocketError::MessageTooLarge);
    }
    Ok(len)
}

pub fn header_value<'a>(request: &'a str, name: &str) -> Option<&'a str> {
    request.lines().skip(1).find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.trim()
            .eq_ignore_ascii_case(name)
            .then_some(value.trim())
    })
}

pub fn handshake_complete(bytes: &[u8]) -> bool {
    bytes.ends_with(WS_HTTP_HEADER_TERMINATOR)
}

pub fn decode_handshake_request(bytes: Vec<u8>) -> Result<String, WebSocketError> {
    if bytes.len() > WS_MAX_HANDSHAKE_LEN {
        return Err(WebSocketError::HandshakeTooLarge);
    }
    String::from_utf8(bytes).map_err(Into::into)
}

pub fn websocket_accept(key: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(key.as_bytes());
    hasher.update(WS_GUID.as_bytes());
    let digest = hasher.finalize();
    standard_encode(&digest)
}

pub fn encode_upgrade_response(request: &str, protocol: &str) -> Result<String, WebSocketError> {
    let key = header_value(request, "sec-websocket-key").ok_or(WebSocketError::MissingKey)?;
    let upgrade = header_value(request, "upgrade").unwrap_or_default();
    if !upgrade.eq_ignore_ascii_case("websocket") {
        return Err(WebSocketError::NotUpgrade);
    }

    let accept = websocket_accept(key.trim());
    Ok(format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: {accept}\r\n\
         Sec-WebSocket-Protocol: {protocol}\r\n\
         \r\n"
    ))
}

pub fn decode_client_message(
    fin: bool,
    opcode: u8,
    mask: [u8; 4],
    mut payload: Vec<u8>,
) -> Result<WebSocketMessage, WebSocketError> {
    if !fin {
        return Err(WebSocketError::Fragmented);
    }
    for (index, byte) in payload.iter_mut().enumerate() {
        *byte ^= mask[index % 4];
    }
    match opcode {
        WS_OPCODE_BINARY => Ok(WebSocketMessage::Binary(payload)),
        WS_OPCODE_CLOSE => Ok(WebSocketMessage::Close),
        WS_OPCODE_PING => Ok(WebSocketMessage::Ping(payload)),
        WS_OPCODE_PONG => Ok(WebSocketMessage::Pong),
        _ => Err(WebSocketError::UnsupportedOpcode),
    }
}

pub fn encode_server_frame(opcode: u8, payload: &[u8]) -> Result<Vec<u8>, WebSocketError> {
    if matches!(opcode, WS_OPCODE_CLOSE | WS_OPCODE_PING | WS_OPCODE_PONG)
        && payload.len() > WS_MAX_CONTROL_PAYLOAD_LEN
    {
        return Err(WebSocketError::ControlPayloadTooLarge);
    }

    let mut frame = Vec::with_capacity(10 + payload.len());
    frame.push(0x80 | (opcode & 0x0f));
    if payload.len() < 126 {
        frame.push(payload.len() as u8);
    } else if payload.len() <= u16::MAX as usize {
        frame.push(126);
        push_u16_be(&mut frame, payload.len() as u16);
    } else {
        frame.push(127);
        push_u64_be(&mut frame, payload.len() as u64);
    }
    frame.extend_from_slice(payload);
    Ok(frame)
}

pub fn encode_server_binary(payload: &[u8]) -> Result<Vec<u8>, WebSocketError> {
    encode_server_frame(WS_OPCODE_BINARY, payload)
}

pub fn encode_server_control(opcode: u8, payload: &[u8]) -> Result<Vec<u8>, WebSocketError> {
    encode_server_frame(opcode, payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn encodes_binary_frame() {
        let frame = encode_server_binary(b"abc").unwrap();
        assert_eq!(frame, vec![0x82, 3, b'a', b'b', b'c']);
    }

    #[test]
    fn decodes_masked_ping() {
        let message =
            decode_client_message(true, WS_OPCODE_PING, [1, 2, 3, 4], vec![9, 11]).unwrap();
        assert_eq!(message, WebSocketMessage::Ping(vec![8, 9]));
    }

    #[test]
    fn rejects_unmasked_client_prefix() {
        let prefix = decode_frame_prefix(&[0x82, 0x03]).unwrap();
        assert_eq!(
            prefix.require_client_mask(),
            Err(WebSocketError::UnmaskedClientFrame)
        );
    }

    #[test]
    fn rejects_large_control_payload() {
        assert_eq!(
            encode_server_control(WS_OPCODE_PONG, &[0; 126]),
            Err(WebSocketError::ControlPayloadTooLarge)
        );
    }

    #[test]
    fn decodes_extended_payload_len() {
        let prefix = decode_frame_prefix(&[0x82, 126]).unwrap();
        assert_eq!(prefix.extended_len_bytes(), WS_EXTENDED_16_LEN);
        assert_eq!(
            decode_payload_len(prefix, &[0x01, 0x00], 1024).unwrap(),
            256
        );
    }

    #[test]
    fn builds_upgrade_response() {
        let request = concat!(
            "GET / HTTP/1.1\r\n",
            "Upgrade: websocket\r\n",
            "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n",
            "\r\n"
        );
        let response = encode_upgrade_response(request, WS_BLOCK_PROTOCOL).unwrap();
        assert!(response.contains("HTTP/1.1 101 Switching Protocols"));
        assert!(response.contains("Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo="));
        assert!(response.contains("Sec-WebSocket-Protocol: edgerun-block-v1"));
    }

    #[test]
    fn detects_handshake_boundary() {
        assert!(!handshake_complete(b"GET / HTTP/1.1\r\n"));
        assert!(handshake_complete(b"GET / HTTP/1.1\r\n\r\n"));
    }
}
