//! std-hosted async network compatibility for crates migrating off edgerun-rt.

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener as StdTcpListener, TcpStream as StdTcpStream};
use std::net::{ToSocketAddrs, UdpSocket as StdUdpSocket};
#[cfg(unix)]
use std::os::fd::{FromRawFd, IntoRawFd, RawFd};

use crate::io::{AsyncRead, AsyncWrite, IoError, Result as IoResult};

fn map_io_error(error: std::io::Error) -> IoError {
    match error.kind() {
        std::io::ErrorKind::UnexpectedEof => IoError::UnexpectedEof,
        std::io::ErrorKind::WriteZero => IoError::WriteZero,
        _ => IoError::Other("io error"),
    }
}

pub struct AsyncTcpStream {
    inner: StdTcpStream,
}

impl AsyncTcpStream {
    #[cfg(unix)]
    pub fn from_fd(fd: RawFd) -> Self {
        Self {
            inner: unsafe { StdTcpStream::from_raw_fd(fd) },
        }
    }

    #[cfg(unix)]
    pub fn from_raw(fd: RawFd) -> Self {
        Self::from_fd(fd)
    }

    pub fn from_std(stream: StdTcpStream) -> std::io::Result<Self> {
        stream.set_nonblocking(true)?;
        Ok(Self { inner: stream })
    }

    #[cfg(unix)]
    pub fn into_fd(self) -> RawFd {
        self.inner.into_raw_fd()
    }

    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.inner.local_addr()
    }

    pub fn peer_addr(&self) -> std::io::Result<SocketAddr> {
        self.inner.peer_addr()
    }
}

impl AsyncRead for AsyncTcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<IoResult<usize>> {
        match self.get_mut().inner.read(buf) {
            Ok(n) => Poll::Ready(Ok(n)),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(map_io_error(e))),
        }
    }
}

impl AsyncWrite for AsyncTcpStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<IoResult<usize>> {
        match self.get_mut().inner.write(buf) {
            Ok(n) => Poll::Ready(Ok(n)),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(map_io_error(e))),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        match self.get_mut().inner.flush() {
            Ok(()) => Poll::Ready(Ok(())),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(map_io_error(e))),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<IoResult<()>> {
        match self.inner.shutdown(std::net::Shutdown::Both) {
            Ok(()) => Poll::Ready(Ok(())),
            Err(e) => Poll::Ready(Err(map_io_error(e))),
        }
    }
}

impl AsyncRead for Arc<AsyncTcpStream> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<IoResult<usize>> {
        match (&self.inner).read(buf) {
            Ok(n) => Poll::Ready(Ok(n)),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(map_io_error(e))),
        }
    }
}

impl AsyncWrite for Arc<AsyncTcpStream> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<IoResult<usize>> {
        match (&self.inner).write(buf) {
            Ok(n) => Poll::Ready(Ok(n)),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(map_io_error(e))),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        match (&self.inner).flush() {
            Ok(()) => Poll::Ready(Ok(())),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(map_io_error(e))),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<IoResult<()>> {
        match self.inner.shutdown(std::net::Shutdown::Both) {
            Ok(()) => Poll::Ready(Ok(())),
            Err(e) => Poll::Ready(Err(map_io_error(e))),
        }
    }
}

pub struct ConnectFuture {
    addrs: Vec<SocketAddr>,
    idx: usize,
}

impl ConnectFuture {
    pub fn new<A: ToSocketAddrs>(addrs: A) -> Self {
        Self {
            addrs: addrs
                .to_socket_addrs()
                .map(|addrs| addrs.collect())
                .unwrap_or_default(),
            idx: 0,
        }
    }
}

impl Future for ConnectFuture {
    type Output = std::io::Result<Arc<AsyncTcpStream>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        while let Some(addr) = self.addrs.get(self.idx).copied() {
            self.idx += 1;
            match StdTcpStream::connect(addr).and_then(AsyncTcpStream::from_std) {
                Ok(stream) => return Poll::Ready(Ok(Arc::new(stream))),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }
                Err(_) => continue,
            }
        }
        Poll::Ready(Err(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "all addresses refused",
        )))
    }
}

pub struct AsyncTcpListener {
    inner: StdTcpListener,
}

impl AsyncTcpListener {
    pub fn bind<A: ToSocketAddrs>(addr: A) -> std::io::Result<Self> {
        let listener = StdTcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        Ok(Self { inner: listener })
    }

    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.inner.local_addr()
    }

    pub fn accept(&self) -> AcceptFuture<'_> {
        AcceptFuture { listener: self }
    }
}

pub struct AcceptFuture<'a> {
    listener: &'a AsyncTcpListener,
}

impl Future for AcceptFuture<'_> {
    type Output = std::io::Result<(Arc<AsyncTcpStream>, SocketAddr)>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.listener.inner.accept() {
            Ok((stream, addr)) => match AsyncTcpStream::from_std(stream) {
                Ok(stream) => Poll::Ready(Ok((Arc::new(stream), addr))),
                Err(e) => Poll::Ready(Err(e)),
            },
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

pub struct AsyncUdpSocket {
    inner: StdUdpSocket,
}

impl AsyncUdpSocket {
    pub fn bind<A: ToSocketAddrs>(addr: A) -> std::io::Result<Self> {
        let socket = StdUdpSocket::bind(addr)?;
        Self::from_std(socket)
    }

    pub fn from_std(socket: StdUdpSocket) -> std::io::Result<Self> {
        socket.set_nonblocking(true)?;
        Ok(Self { inner: socket })
    }

    pub fn connect<A: ToSocketAddrs>(&self, addr: A) -> std::io::Result<()> {
        self.inner.connect(addr)
    }

    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.inner.local_addr()
    }

    pub fn peer_addr(&self) -> std::io::Result<SocketAddr> {
        self.inner.peer_addr()
    }

    pub fn set_broadcast(&self, broadcast: bool) -> std::io::Result<()> {
        self.inner.set_broadcast(broadcast)
    }

    pub async fn send_to(&self, buf: &[u8], target: SocketAddr) -> std::io::Result<usize> {
        self.inner.send_to(buf, target)
    }

    pub async fn recv_from(&self, buf: &mut [u8]) -> std::io::Result<(usize, SocketAddr)> {
        self.inner.recv_from(buf)
    }

    pub fn send(&self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.send(buf)
    }

    pub fn recv(&self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.recv(buf)
    }
}
