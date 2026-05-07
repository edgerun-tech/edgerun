//! Transport-free DNS-over-TCP frame codec.
//!
//! DNS over TCP carries each DNS wire message behind a two-byte big-endian
//! length prefix. This module owns that framing without owning sockets.

use alloc::vec;
use alloc::vec::Vec;

use super::io;
use super::limits::{validate_dns_wire_bounds, MAX_DNS_MESSAGE_LEN};

const DNS_TCP_LENGTH_LEN: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsTcpFrameError {
    ZeroLength,
    MessageTooLarge,
    InvalidDnsMessage,
}

impl From<DnsTcpFrameError> for io::Error {
    fn from(value: DnsTcpFrameError) -> Self {
        let message = match value {
            DnsTcpFrameError::ZeroLength => "zero DNS TCP frame length",
            DnsTcpFrameError::MessageTooLarge => "DNS TCP frame exceeds maximum wire length",
            DnsTcpFrameError::InvalidDnsMessage => "invalid DNS message in TCP frame",
        };
        io::Error::new(io::ErrorKind::InvalidData, message)
    }
}

pub fn dns_tcp_frame_len(prefix: [u8; DNS_TCP_LENGTH_LEN]) -> Result<usize, DnsTcpFrameError> {
    let len = u16::from_be_bytes(prefix) as usize;
    if len == 0 {
        return Err(DnsTcpFrameError::ZeroLength);
    }
    if len > MAX_DNS_MESSAGE_LEN {
        return Err(DnsTcpFrameError::MessageTooLarge);
    }
    Ok(len)
}

pub fn encode_dns_tcp_frame(message: &[u8]) -> Result<Vec<u8>, DnsTcpFrameError> {
    if message.is_empty() {
        return Err(DnsTcpFrameError::ZeroLength);
    }
    if message.len() > MAX_DNS_MESSAGE_LEN {
        return Err(DnsTcpFrameError::MessageTooLarge);
    }

    let mut frame = Vec::with_capacity(DNS_TCP_LENGTH_LEN + message.len());
    frame.extend_from_slice(&(message.len() as u16).to_be_bytes());
    frame.extend_from_slice(message);
    Ok(frame)
}

#[derive(Debug, Default, Clone)]
pub struct DnsTcpFrameDecoder {
    buffered: Vec<u8>,
}

impl DnsTcpFrameDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn buffered_len(&self) -> usize {
        self.buffered.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffered.is_empty()
    }

    pub fn push(&mut self, bytes: &[u8]) {
        self.buffered.extend_from_slice(bytes);
    }

    pub fn next_frame(&mut self) -> Result<Option<Vec<u8>>, DnsTcpFrameError> {
        if self.buffered.len() < DNS_TCP_LENGTH_LEN {
            return Ok(None);
        }

        let len = dns_tcp_frame_len([self.buffered[0], self.buffered[1]])?;
        let frame_end = DNS_TCP_LENGTH_LEN + len;
        if self.buffered.len() < frame_end {
            return Ok(None);
        }

        let message = self.buffered[DNS_TCP_LENGTH_LEN..frame_end].to_vec();
        validate_dns_wire_bounds(&message).map_err(|_| DnsTcpFrameError::InvalidDnsMessage)?;
        self.buffered.drain(..frame_end);
        Ok(Some(message))
    }
}

pub fn decode_single_dns_tcp_frame(frame: &[u8]) -> Result<Option<Vec<u8>>, DnsTcpFrameError> {
    let mut decoder = DnsTcpFrameDecoder {
        buffered: frame.to_vec(),
    };
    decoder.next_frame()
}

pub fn formerr_dns_query_frame(id: u16) -> Vec<u8> {
    let mut message = vec![
        (id >> 8) as u8,
        id as u8,
        0x81,
        0x81,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ];
    encode_dns_tcp_frame(&message).unwrap_or_else(|_| {
        message.clear();
        message
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_dns_message() -> [u8; 12] {
        [0u8; 12]
    }

    #[test]
    fn encodes_length_prefixed_frame() {
        let message = empty_dns_message();
        let frame = encode_dns_tcp_frame(&message).unwrap();
        assert_eq!(&frame[..2], &[0, 12]);
        assert_eq!(&frame[2..], &message);
    }

    #[test]
    fn decoder_waits_for_complete_frame() {
        let message = empty_dns_message();
        let frame = encode_dns_tcp_frame(&message).unwrap();
        let mut decoder = DnsTcpFrameDecoder::new();

        decoder.push(&frame[..4]);
        assert_eq!(decoder.next_frame().unwrap(), None);
        decoder.push(&frame[4..]);
        assert_eq!(decoder.next_frame().unwrap(), Some(message.to_vec()));
        assert!(decoder.is_empty());
    }

    #[test]
    fn rejects_zero_length_frame() {
        assert_eq!(
            decode_single_dns_tcp_frame(&[0, 0]),
            Err(DnsTcpFrameError::ZeroLength)
        );
    }
}
