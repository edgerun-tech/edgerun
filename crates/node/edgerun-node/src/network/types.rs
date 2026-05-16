use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;
use core::net::SocketAddr;

use crate::rt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportProtocol {
    Stream,
    Datagram,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportCarrier {
    HostSocket,
    BareFrame,
    Mesh,
    Email,
    BrowserIpc,
    Memory,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransportAddress {
    pub carrier: TransportCarrier,
    pub protocol: TransportProtocol,
    pub endpoint: Vec<u8>,
}

impl TransportAddress {
    pub fn new(
        carrier: TransportCarrier,
        protocol: TransportProtocol,
        endpoint: impl Into<Vec<u8>>,
    ) -> Self {
        Self {
            carrier,
            protocol,
            endpoint: endpoint.into(),
        }
    }

    pub fn email_stream(endpoint: impl Into<Vec<u8>>) -> Self {
        Self::new(TransportCarrier::Email, TransportProtocol::Stream, endpoint)
    }

    pub fn email_datagram(endpoint: impl Into<Vec<u8>>) -> Self {
        Self::new(
            TransportCarrier::Email,
            TransportProtocol::Datagram,
            endpoint,
        )
    }

    pub fn host_stream(endpoint: impl Into<Vec<u8>>) -> Self {
        Self::new(
            TransportCarrier::HostSocket,
            TransportProtocol::Stream,
            endpoint,
        )
    }

    pub fn host_datagram(endpoint: impl Into<Vec<u8>>) -> Self {
        Self::new(
            TransportCarrier::HostSocket,
            TransportProtocol::Datagram,
            endpoint,
        )
    }

    pub fn bare_datagram(endpoint: impl Into<Vec<u8>>) -> Self {
        Self::new(
            TransportCarrier::BareFrame,
            TransportProtocol::Datagram,
            endpoint,
        )
    }
}

impl From<SocketAddr> for TransportAddress {
    fn from(addr: SocketAddr) -> Self {
        Self::new(
            TransportCarrier::HostSocket,
            TransportProtocol::Stream,
            addr.to_string().into_bytes(),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransportFrame {
    pub source: TransportAddress,
    pub destination: TransportAddress,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransportError {
    UnsupportedCarrier,
    AddressUnavailable,
    Io(String),
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedCarrier => f.write_str("unsupported transport carrier"),
            Self::AddressUnavailable => f.write_str("transport address unavailable"),
            Self::Io(error) => write!(f, "transport I/O error: {error}"),
        }
    }
}

impl core::error::Error for TransportError {}

impl From<rt::IoError> for TransportError {
    fn from(error: rt::IoError) -> Self {
        Self::Io(error.to_string())
    }
}
