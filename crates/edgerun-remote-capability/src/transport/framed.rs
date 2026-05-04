//! Length-prefixed framed transport over any `Read + Write` stream.

use crate::prelude::v1::*;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;

use edgerun_capabilities::CapabilityError;
use edgerun_core::protocol::capability_runtime::CapabilityRemoteEnvelope;

use crate::protocol::RemoteCapabilityTransport;

#[derive(Debug)]
pub struct FramedRemoteTransport<S> {
    stream: S,
}

impl<S> FramedRemoteTransport<S> {
    pub fn new(stream: S) -> Self {
        Self { stream }
    }

    pub fn into_inner(self) -> S {
        self.stream
    }
}

impl FramedRemoteTransport<UnixStream> {
    pub fn connect_unix(path: impl AsRef<Path>) -> Result<Self, CapabilityError> {
        UnixStream::connect(path)
            .map(Self::new)
            .map_err(|err| CapabilityError::Provider(err.to_string()))
    }
}

impl FramedRemoteTransport<TcpStream> {
    pub fn connect_tcp(addr: impl ToSocketAddrs) -> Result<Self, CapabilityError> {
        TcpStream::connect(addr)
            .map(Self::new)
            .map_err(|err| CapabilityError::Provider(err.to_string()))
    }
}

impl<S> RemoteCapabilityTransport for FramedRemoteTransport<S>
where
    S: Read + Write,
{
    fn send(&mut self, envelope: CapabilityRemoteEnvelope) -> Result<(), CapabilityError> {
        let payload = encode_capability_remote_envelope(envelope);
        let len = u32::try_from(payload.len()).map_err(|_| {
            CapabilityError::InvalidRequest("remote capability envelope exceeds max frame size")
        })?;
        self.stream
            .write_all(&len.to_be_bytes())
            .and_then(|_| self.stream.write_all(&payload))
            .map_err(|err| CapabilityError::Provider(err.to_string()))
    }

    fn recv(&mut self) -> Result<Option<CapabilityRemoteEnvelope>, CapabilityError> {
        let mut len_buf = [0u8; 4];
        let bytes_read = self
            .stream
            .read(&mut len_buf)
            .map_err(|err| CapabilityError::Provider(err.to_string()))?;
        if bytes_read == 0 {
            return Ok(None);
        }
        if bytes_read != 4 {
            self.stream
                .read_exact(&mut len_buf[bytes_read..])
                .map_err(|err| CapabilityError::Provider(err.to_string()))?;
        }
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut payload = vec![0u8; len];
        self.stream
            .read_exact(&mut payload)
            .map_err(|err| CapabilityError::Provider(err.to_string()))?;
        decode_capability_remote_envelope(payload.as_slice())
            .map(Some)
            .map_err(|err| CapabilityError::Provider(err.to_string()))
    }
}

pub fn accept_unix(
    listener: &UnixListener,
) -> Result<FramedRemoteTransport<UnixStream>, CapabilityError> {
    listener
        .accept()
        .map(|(stream, _)| FramedRemoteTransport::new(stream))
        .map_err(|err| CapabilityError::Provider(err.to_string()))
}

pub fn accept_tcp(
    listener: &TcpListener,
) -> Result<FramedRemoteTransport<TcpStream>, CapabilityError> {
    listener
        .accept()
        .map(|(stream, _)| FramedRemoteTransport::new(stream))
        .map_err(|err| CapabilityError::Provider(err.to_string()))
}

fn encode_capability_remote_envelope(envelope: &CapabilityRemoteEnvelope) -> Vec<u8> {
    // Transitional native frame payload. This preserves the transport framing
    // without bringing prost back. Replace with edgerun-wire typed encoding next.
    let mut out = Vec::new();
    out.extend_from_slice(b"ERCR");
    out.extend_from_slice(&envelope.envelope_version.to_le_bytes());
    out.extend_from_slice(&envelope.session_id);
    out.extend_from_slice(&envelope.seq.to_le_bytes());
    out.extend_from_slice(&(envelope.message_type as u32).to_le_bytes());
    out.extend_from_slice(&(envelope.payload.len() as u32).to_le_bytes());
    out.extend_from_slice(&envelope.payload);
    out
}

fn decode_capability_remote_envelope(bytes: &[u8]) -> Result<CapabilityRemoteEnvelope, String> {
    if bytes.len() < 4 + 4 + 8 + 8 + 4 + 4 {
        return Err("remote capability frame too short".to_string());
    }
    if &bytes[0..4] != b"ERCR" {
        return Err("invalid remote capability frame magic".to_string());
    }

    let mut offset = 4;

    let read_u32 = |bytes: &[u8], offset: &mut usize| -> Result<u32, String> {
        let end = *offset + 4;
        let slice = bytes
            .get(*offset..end)
            .ok_or_else(|| "short u32".to_string())?;
        *offset = end;
        Ok(u32::from_le_bytes(
            slice.try_into().map_err(|_| "bad u32".to_string())?,
        ))
    };

    let read_u64 = |bytes: &[u8], offset: &mut usize| -> Result<u64, String> {
        let end = *offset + 8;
        let slice = bytes
            .get(*offset..end)
            .ok_or_else(|| "short u64".to_string())?;
        *offset = end;
        Ok(u64::from_le_bytes(
            slice.try_into().map_err(|_| "bad u64".to_string())?,
        ))
    };

    let envelope_version = read_u32(bytes, &mut offset)?;
    let session_slice = bytes
        .get(offset..offset + 8)
        .ok_or_else(|| "short session_id".to_string())?;
    let mut session_id = Vec::new();
    session_id.extend_from_slice(session_slice);
    offset += 8;

    let seq = read_u64(bytes, &mut offset)?;
    let message_type = read_u32(bytes, &mut offset)? as i32;
    let payload_len = read_u32(bytes, &mut offset)? as usize;
    let payload = bytes
        .get(offset..offset + payload_len)
        .ok_or_else(|| "short payload".to_string())?
        .to_vec();

    Ok(CapabilityRemoteEnvelope {
        envelope_version,
        session_id,
        seq,
        message_type,
        payload,
        signature: None,
    })
}
