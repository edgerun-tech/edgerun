//! Length-prefixed framed transport over any `Read + Write` stream.

use crate::prelude::v1::*;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;

use edgerun_capabilities::CapabilityError;
use edgerun_protocols::core_protocol::protocol::capability_runtime::CapabilityRemoteEnvelope;

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
        let payload = encode_capability_remote_envelope(&envelope);
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
    edgerun_protocols::wire::to_bytes::<edgerun_protocols::wire::WireError>(envelope)
        .expect("remote capability envelope must serialize through rkyv")
        .into_vec()
}

fn decode_capability_remote_envelope(bytes: &[u8]) -> Result<CapabilityRemoteEnvelope, String> {
    let owned = bytes.to_vec();
    edgerun_protocols::wire::from_bytes::<
        CapabilityRemoteEnvelope,
        edgerun_protocols::wire::WireError,
    >(&owned)
    .map_err(|_| "invalid rkyv remote capability envelope".to_owned())
}
