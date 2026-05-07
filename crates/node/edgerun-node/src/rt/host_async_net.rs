//! Host-backed async network provider for Linux/user-space targets.
//!
//! The public boundary remains Edgerun runtime socket types. This module is an
//! internal provider that maps node-authorized capabilities to host sockets.

extern crate std;

use alloc::string::ToString;
use alloc::sync::Arc;
use core::future::Future;
use core::net::SocketAddr;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::io::{Read, Write};

use crate::rt::io::{AsyncRead, AsyncWrite, IoError, Result as IoResult};

fn io_error(error: std::io::Error) -> IoError {
    match error.kind() {
        std::io::ErrorKind::UnexpectedEof => IoError::UnexpectedEof,
        std::io::ErrorKind::WriteZero => IoError::WriteZero,
        _ => IoError::Other("host network operation failed"),
    }
}

#[derive(Clone)]
pub struct AsyncTcpStream {
    inner: Arc<std::net::TcpStream>,
}

impl AsyncTcpStream {
    fn from_std(stream: std::net::TcpStream) -> IoResult<Self> {
        stream.set_nonblocking(true).map_err(io_error)?;
        Ok(Self {
            inner: Arc::new(stream),
        })
    }

    pub fn local_addr(&self) -> IoResult<SocketAddr> {
        self.inner.local_addr().map_err(io_error)
    }

    pub fn peer_addr(&self) -> IoResult<SocketAddr> {
        self.inner.peer_addr().map_err(io_error)
    }

    pub fn split(self: Arc<Self>) -> (Arc<Self>, Arc<Self>) {
        (self.clone(), self)
    }
}

fn poll_host_read(
    stream: &std::net::TcpStream,
    cx: &mut Context<'_>,
    buf: &mut [u8],
) -> Poll<IoResult<usize>> {
    let mut stream = stream;
    match stream.read(buf) {
        Ok(value) => Poll::Ready(Ok(value)),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
        Err(error) => Poll::Ready(Err(io_error(error))),
    }
}

fn poll_host_write(
    stream: &std::net::TcpStream,
    cx: &mut Context<'_>,
    buf: &[u8],
) -> Poll<IoResult<usize>> {
    let mut stream = stream;
    match stream.write(buf) {
        Ok(value) => Poll::Ready(Ok(value)),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
        Err(error) => Poll::Ready(Err(io_error(error))),
    }
}

impl AsyncRead for AsyncTcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<IoResult<usize>> {
        poll_host_read(&self.inner, cx, buf)
    }
}

impl AsyncWrite for AsyncTcpStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<IoResult<usize>> {
        poll_host_write(&self.inner, cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        let mut stream = &*self.inner;
        Poll::Ready(stream.flush().map_err(io_error).map(|_| ()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        let result = self
            .inner
            .shutdown(std::net::Shutdown::Both)
            .or_else(|error| {
                if error.kind() == std::io::ErrorKind::NotConnected {
                    Ok(())
                } else {
                    Err(error)
                }
            })
            .map_err(io_error);
        Poll::Ready(result)
    }
}

impl AsyncRead for Arc<AsyncTcpStream> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<IoResult<usize>> {
        poll_host_read(&self.inner, cx, buf)
    }
}

impl AsyncWrite for Arc<AsyncTcpStream> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<IoResult<usize>> {
        poll_host_write(&self.inner, cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        let mut stream = &*self.inner;
        Poll::Ready(stream.flush().map_err(io_error).map(|_| ()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        let result = self
            .inner
            .shutdown(std::net::Shutdown::Both)
            .or_else(|error| {
                if error.kind() == std::io::ErrorKind::NotConnected {
                    Ok(())
                } else {
                    Err(error)
                }
            })
            .map_err(io_error);
        Poll::Ready(result)
    }
}

pub struct AsyncTcpListener {
    inner: std::net::TcpListener,
    bind: TcpBindSpec,
}

impl AsyncTcpListener {
    pub fn bind<A: std::net::ToSocketAddrs>(addr: A) -> IoResult<Self> {
        let listener = std::net::TcpListener::bind(addr).map_err(io_error)?;
        listener.set_nonblocking(true).map_err(io_error)?;
        let local_addr = listener.local_addr().map_err(io_error)?;
        Ok(Self {
            inner: listener,
            bind: TcpBindSpec::new(local_addr),
        })
    }

    pub fn bind_spec(&self) -> &TcpBindSpec {
        &self.bind
    }

    pub fn local_addr(&self) -> IoResult<SocketAddr> {
        self.inner.local_addr().map_err(io_error)
    }

    pub fn accept(&self) -> AcceptFuture<'_> {
        AcceptFuture { listener: self }
    }
}

pub struct AcceptFuture<'a> {
    listener: &'a AsyncTcpListener,
}

impl Future for AcceptFuture<'_> {
    type Output = IoResult<(Arc<AsyncTcpStream>, SocketAddr)>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.listener.inner.accept() {
            Ok((stream, peer)) => match AsyncTcpStream::from_std(stream) {
                Ok(stream) => Poll::Ready(Ok((Arc::new(stream), peer))),
                Err(error) => Poll::Ready(Err(error)),
            },
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(error) => Poll::Ready(Err(io_error(error))),
        }
    }
}

pub struct ConnectFuture {
    target: alloc::string::String,
    done: bool,
}

impl ConnectFuture {
    pub fn new<A: ToString>(addrs: A) -> Self {
        Self {
            target: addrs.to_string(),
            done: false,
        }
    }
}

impl Future for ConnectFuture {
    type Output = IoResult<Arc<AsyncTcpStream>>;

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.done {
            return Poll::Ready(Err(IoError::Other("host TCP connect already completed")));
        }
        self.done = true;
        match std::net::TcpStream::connect(&self.target) {
            Ok(stream) => Poll::Ready(AsyncTcpStream::from_std(stream).map(Arc::new)),
            Err(error) => Poll::Ready(Err(io_error(error))),
        }
    }
}

pub struct AsyncUdpSocket {
    inner: std::net::UdpSocket,
    bind: UdpBindSpec,
}

impl AsyncUdpSocket {
    pub fn bind<A: std::net::ToSocketAddrs>(addr: A) -> IoResult<Self> {
        let socket = std::net::UdpSocket::bind(addr).map_err(io_error)?;
        Self::from_std(socket)
    }

    pub fn from_std(socket: std::net::UdpSocket) -> IoResult<Self> {
        socket.set_nonblocking(true).map_err(io_error)?;
        let local_addr = socket.local_addr().map_err(io_error)?;
        Ok(Self {
            inner: socket,
            bind: UdpBindSpec::new(local_addr),
        })
    }

    pub fn bind_spec(&self) -> &UdpBindSpec {
        &self.bind
    }

    pub fn local_addr(&self) -> IoResult<SocketAddr> {
        self.inner.local_addr().map_err(io_error)
    }

    pub fn peer_addr(&self) -> IoResult<SocketAddr> {
        self.inner.peer_addr().map_err(io_error)
    }

    pub fn connect<A: std::net::ToSocketAddrs>(&self, addr: A) -> IoResult<()> {
        self.inner.connect(addr).map_err(io_error)
    }

    pub fn set_broadcast(&self, broadcast: bool) -> IoResult<()> {
        self.inner.set_broadcast(broadcast).map_err(io_error)
    }

    pub fn recv_from<'a>(&'a self, buf: &'a mut [u8]) -> UdpRecvFromFuture<'a> {
        UdpRecvFromFuture { socket: self, buf }
    }

    pub fn send_to<'a>(&'a self, buf: &'a [u8], target: SocketAddr) -> UdpSendToFuture<'a> {
        UdpSendToFuture {
            socket: self,
            buf,
            target,
        }
    }

    pub fn recv(&self, buf: &mut [u8]) -> IoResult<usize> {
        self.inner.recv(buf).map_err(io_error)
    }

    pub fn send(&self, buf: &[u8]) -> IoResult<usize> {
        self.inner.send(buf).map_err(io_error)
    }
}

pub struct UdpRecvFromFuture<'a> {
    socket: &'a AsyncUdpSocket,
    buf: &'a mut [u8],
}

impl Future for UdpRecvFromFuture<'_> {
    type Output = IoResult<(usize, SocketAddr)>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;
        match this.socket.inner.recv_from(this.buf) {
            Ok(value) => Poll::Ready(Ok(value)),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(error) => Poll::Ready(Err(io_error(error))),
        }
    }
}

pub struct UdpSendToFuture<'a> {
    socket: &'a AsyncUdpSocket,
    buf: &'a [u8],
    target: SocketAddr,
}

impl Future for UdpSendToFuture<'_> {
    type Output = IoResult<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.socket.inner.send_to(self.buf, self.target) {
            Ok(value) => Poll::Ready(Ok(value)),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(error) => Poll::Ready(Err(io_error(error))),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IpProtocol {
    Tcp,
    Udp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SocketCapability {
    pub protocol: IpProtocol,
    pub local_addr: SocketAddr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TcpBindSpec {
    pub capability: SocketCapability,
}

impl TcpBindSpec {
    pub const fn new(local_addr: SocketAddr) -> Self {
        Self {
            capability: SocketCapability {
                protocol: IpProtocol::Tcp,
                local_addr,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UdpBindSpec {
    pub capability: SocketCapability,
}

impl UdpBindSpec {
    pub const fn new(local_addr: SocketAddr) -> Self {
        Self {
            capability: SocketCapability {
                protocol: IpProtocol::Udp,
                local_addr,
            },
        }
    }
}
