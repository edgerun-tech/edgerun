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
        let payload = envelope.encode_to_vec();
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
        CapabilityRemoteEnvelope::decode(payload.as_slice())
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
