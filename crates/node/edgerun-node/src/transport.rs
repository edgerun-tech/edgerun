//! Node-owned transport abstraction.
//!
//! Protocol services deal in streams and datagrams. The node decides whether a
//! transport is a host socket, bare network frame path, mesh route, email-carried
//! frame, browser IPC route, or another runtime carrier.

use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt;
use core::future::Future;
use core::net::SocketAddr;
use core::pin::Pin;
use core::task::{Context, Poll};

use crate::rt::{self, AsyncRead, AsyncWrite};

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

pub trait NodeStream: AsyncRead + AsyncWrite {
    fn local_transport_addr(&self) -> Result<TransportAddress, TransportError>;
    fn peer_transport_addr(&self) -> Result<TransportAddress, TransportError>;
}

pub trait NodeStreamListener {
    type Stream: NodeStream;

    fn local_transport_addr(&self) -> Result<TransportAddress, TransportError>;
    fn poll_accept_stream(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(Self::Stream, TransportAddress), TransportError>>;
}

pub trait NodeDatagram {
    fn local_transport_addr(&self) -> Result<TransportAddress, TransportError>;
    fn peer_transport_addr(&self) -> Result<Option<TransportAddress>, TransportError>;
    fn poll_recv_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        payload: &mut [u8],
    ) -> Poll<Result<(usize, TransportAddress), TransportError>>;
    fn poll_send_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        payload: &[u8],
        destination: &TransportAddress,
    ) -> Poll<Result<usize, TransportError>>;
}

pub trait RuntimeTransport {
    type Stream: NodeStream;
    type Listener: NodeStreamListener<Stream = Self::Stream>;
    type Datagram: NodeDatagram;
    type ConnectFuture<'a>: Future<Output = Result<Self::Stream, TransportError>> + 'a
    where
        Self: 'a;
    type BindStreamFuture<'a>: Future<Output = Result<Self::Listener, TransportError>> + 'a
    where
        Self: 'a;
    type BindDatagramFuture<'a>: Future<Output = Result<Self::Datagram, TransportError>> + 'a
    where
        Self: 'a;

    fn connect_stream<'a>(&'a self, addr: &'a TransportAddress) -> Self::ConnectFuture<'a>;
    fn bind_stream<'a>(&'a self, addr: &'a TransportAddress) -> Self::BindStreamFuture<'a>;
    fn bind_datagram<'a>(&'a self, addr: &'a TransportAddress) -> Self::BindDatagramFuture<'a>;
}

pub type BoxedNodeStream = Box<dyn NodeStream>;

pub struct Ready<T> {
    value: Option<T>,
}

impl<T> Ready<T> {
    pub const fn new(value: T) -> Self {
        Self { value: Some(value) }
    }
}

impl<T: Unpin> Future for Ready<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(
            self.value
                .take()
                .expect("ready future polled after completion"),
        )
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct HostSocketTransport;

#[derive(Clone, Copy, Debug, Default)]
pub struct BareFrameTransport;

pub struct HostConnectFuture {
    inner: rt::ConnectFuture,
}

impl Future for HostConnectFuture {
    type Output = Result<Arc<rt::AsyncTcpStream>, TransportError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match unsafe { Pin::new_unchecked(&mut self.inner) }.poll(cx) {
            Poll::Ready(Ok(stream)) => Poll::Ready(Ok(stream)),
            Poll::Ready(Err(error)) => Poll::Ready(Err(error.into())),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl RuntimeTransport for HostSocketTransport {
    type Stream = Arc<rt::AsyncTcpStream>;
    type Listener = rt::AsyncTcpListener;
    type Datagram = rt::AsyncUdpSocket;
    type ConnectFuture<'a> = HostConnectFuture;
    type BindStreamFuture<'a> = Ready<Result<Self::Listener, TransportError>>;
    type BindDatagramFuture<'a> = Ready<Result<Self::Datagram, TransportError>>;

    fn connect_stream<'a>(&'a self, addr: &'a TransportAddress) -> Self::ConnectFuture<'a> {
        let target = core::str::from_utf8(&addr.endpoint).unwrap_or_default();
        HostConnectFuture {
            inner: rt::ConnectFuture::new(target),
        }
    }

    fn bind_stream<'a>(&'a self, addr: &'a TransportAddress) -> Self::BindStreamFuture<'a> {
        Ready::new(
            parse_host_socket_addr(addr)
                .and_then(|addr| rt::AsyncTcpListener::bind(addr).map_err(TransportError::from)),
        )
    }

    fn bind_datagram<'a>(&'a self, addr: &'a TransportAddress) -> Self::BindDatagramFuture<'a> {
        Ready::new(
            parse_host_socket_addr(addr)
                .and_then(|addr| rt::AsyncUdpSocket::bind(addr).map_err(TransportError::from)),
        )
    }
}

impl HostSocketTransport {
    pub fn bind_stream_now(
        &self,
        addr: &TransportAddress,
    ) -> Result<rt::AsyncTcpListener, TransportError> {
        parse_host_socket_addr(addr)
            .and_then(|addr| rt::AsyncTcpListener::bind(addr).map_err(TransportError::from))
    }

    pub fn bind_datagram_now(
        &self,
        addr: &TransportAddress,
    ) -> Result<rt::AsyncUdpSocket, TransportError> {
        parse_host_socket_addr(addr)
            .and_then(|addr| rt::AsyncUdpSocket::bind(addr).map_err(TransportError::from))
    }
}

impl BareFrameTransport {
    pub fn bind_datagram_now(
        &self,
        addr: &TransportAddress,
    ) -> Result<rt::UdpSocket, TransportError> {
        let local = parse_rt_datagram_addr(addr)?;
        let mut socket = rt::UdpSocket::new();
        socket
            .bind(local)
            .map_err(|_| TransportError::Io("bare datagram bind failed".into()))?;
        Ok(socket)
    }
}

impl NodeStream for rt::AsyncTcpStream {
    fn local_transport_addr(&self) -> Result<TransportAddress, TransportError> {
        Ok(self.local_addr()?.into())
    }

    fn peer_transport_addr(&self) -> Result<TransportAddress, TransportError> {
        Ok(self.peer_addr()?.into())
    }
}

impl NodeStream for Arc<rt::AsyncTcpStream> {
    fn local_transport_addr(&self) -> Result<TransportAddress, TransportError> {
        Ok(self.local_addr()?.into())
    }

    fn peer_transport_addr(&self) -> Result<TransportAddress, TransportError> {
        Ok(self.peer_addr()?.into())
    }
}

impl NodeStreamListener for rt::AsyncTcpListener {
    type Stream = Arc<rt::AsyncTcpStream>;

    fn local_transport_addr(&self) -> Result<TransportAddress, TransportError> {
        Ok(self.local_addr()?.into())
    }

    fn poll_accept_stream(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(Self::Stream, TransportAddress), TransportError>> {
        let listener = self.get_mut();
        let mut accept = listener.accept();
        match unsafe { Pin::new_unchecked(&mut accept) }.poll(cx) {
            Poll::Ready(Ok((stream, peer))) => Poll::Ready(Ok((stream, peer.into()))),
            Poll::Ready(Err(error)) => Poll::Ready(Err(error.into())),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl NodeDatagram for rt::AsyncUdpSocket {
    fn local_transport_addr(&self) -> Result<TransportAddress, TransportError> {
        let mut addr: TransportAddress = self.local_addr()?.into();
        addr.protocol = TransportProtocol::Datagram;
        Ok(addr)
    }

    fn peer_transport_addr(&self) -> Result<Option<TransportAddress>, TransportError> {
        match self.peer_addr() {
            Ok(peer) => {
                let mut addr: TransportAddress = peer.into();
                addr.protocol = TransportProtocol::Datagram;
                Ok(Some(addr))
            }
            Err(_) => Ok(None),
        }
    }

    fn poll_recv_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        payload: &mut [u8],
    ) -> Poll<Result<(usize, TransportAddress), TransportError>> {
        let socket = self.get_mut();
        let mut recv = socket.recv_from(payload);
        match unsafe { Pin::new_unchecked(&mut recv) }.poll(cx) {
            Poll::Ready(Ok((len, source))) => {
                let mut addr: TransportAddress = source.into();
                addr.protocol = TransportProtocol::Datagram;
                Poll::Ready(Ok((len, addr)))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error.into())),
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_send_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        payload: &[u8],
        destination: &TransportAddress,
    ) -> Poll<Result<usize, TransportError>> {
        let destination = parse_host_socket_addr(destination)?;
        let socket = self.get_mut();
        let mut send = socket.send_to(payload, destination);
        match unsafe { Pin::new_unchecked(&mut send) }.poll(cx) {
            Poll::Ready(Ok(len)) => Poll::Ready(Ok(len)),
            Poll::Ready(Err(error)) => Poll::Ready(Err(error.into())),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl NodeDatagram for rt::UdpSocket {
    fn local_transport_addr(&self) -> Result<TransportAddress, TransportError> {
        self.local_addr()
            .map(rt_datagram_addr)
            .ok_or(TransportError::AddressUnavailable)
    }

    fn peer_transport_addr(&self) -> Result<Option<TransportAddress>, TransportError> {
        Ok(self.remote_addr().map(rt_datagram_addr))
    }

    fn poll_recv_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        payload: &mut [u8],
    ) -> Poll<Result<(usize, TransportAddress), TransportError>> {
        match self.recv_from(payload) {
            Ok((len, source)) => Poll::Ready(Ok((len, rt_datagram_addr(source)))),
            Err(_) => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }

    fn poll_send_frame(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        payload: &[u8],
        destination: &TransportAddress,
    ) -> Poll<Result<usize, TransportError>> {
        let destination = match parse_rt_datagram_addr(destination) {
            Ok(addr) => addr,
            Err(error) => return Poll::Ready(Err(error)),
        };
        Poll::Ready(
            self.send_to(payload, destination)
                .map_err(|_| TransportError::Io("bare datagram send failed".into())),
        )
    }
}

fn parse_host_socket_addr(addr: &TransportAddress) -> Result<SocketAddr, TransportError> {
    if addr.carrier != TransportCarrier::HostSocket {
        return Err(TransportError::UnsupportedCarrier);
    }
    let text =
        core::str::from_utf8(&addr.endpoint).map_err(|_| TransportError::AddressUnavailable)?;
    text.parse().map_err(|_| TransportError::AddressUnavailable)
}

fn rt_datagram_addr(addr: rt::SocketAddr) -> TransportAddress {
    let ip = addr.ip_bytes();
    TransportAddress::bare_datagram(
        alloc::format!("{}.{}.{}.{}:{}", ip[0], ip[1], ip[2], ip[3], addr.port()).into_bytes(),
    )
}

fn parse_rt_datagram_addr(addr: &TransportAddress) -> Result<rt::SocketAddr, TransportError> {
    if addr.carrier != TransportCarrier::BareFrame {
        return Err(TransportError::UnsupportedCarrier);
    }
    let text =
        core::str::from_utf8(&addr.endpoint).map_err(|_| TransportError::AddressUnavailable)?;
    let Some((ip_text, port_text)) = text.rsplit_once(':') else {
        return Err(TransportError::AddressUnavailable);
    };
    let port = port_text
        .parse::<u16>()
        .map_err(|_| TransportError::AddressUnavailable)?;
    let mut octets = [0u8; 4];
    let mut count = 0usize;
    for (idx, part) in ip_text.split('.').enumerate() {
        if idx >= 4 {
            return Err(TransportError::AddressUnavailable);
        }
        octets[idx] = part
            .parse::<u8>()
            .map_err(|_| TransportError::AddressUnavailable)?;
        count += 1;
    }
    if count != 4 {
        return Err(TransportError::AddressUnavailable);
    }
    Ok(rt::SocketAddr::from_bytes4(octets, port))
}
