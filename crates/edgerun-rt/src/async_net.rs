//! std-hosted async network compatibility for crates migrating off edgerun-rt.

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};
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

fn register_and_wake_waker(slot: &mut Option<Waker>, cx: &Context<'_>) {
    let needs_refresh =
        !matches!(slot.as_ref(), Some(registered) if registered.will_wake(cx.waker()));
    if needs_refresh {
        *slot = Some(cx.waker().clone());
    }
    if let Some(waker) = slot.as_ref() {
        waker.wake_by_ref();
    }
}

fn clear_waker(slot: &mut Option<Waker>) {
    slot.take();
}

pub struct AsyncTcpStream {
    inner: StdTcpStream,
    read_waker: Option<Waker>,
    write_waker: Option<Waker>,
    flush_waker: Option<Waker>,
}

impl AsyncTcpStream {
    #[cfg(unix)]
    pub fn from_fd(fd: RawFd) -> Self {
        Self {
            inner: unsafe { StdTcpStream::from_raw_fd(fd) },
            read_waker: None,
            write_waker: None,
            flush_waker: None,
        }
    }

    #[cfg(unix)]
    pub fn from_raw(fd: RawFd) -> Self {
        Self::from_fd(fd)
    }

    pub fn from_std(stream: StdTcpStream) -> std::io::Result<Self> {
        stream.set_nonblocking(true)?;
        Ok(Self {
            inner: stream,
            read_waker: None,
            write_waker: None,
            flush_waker: None,
        })
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

    pub fn split(self: Arc<Self>) -> (Arc<Self>, Arc<Self>) {
        (self.clone(), self)
    }
}

impl AsyncRead for AsyncTcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<IoResult<usize>> {
        let this = self.get_mut();
        match this.inner.read(buf) {
            Ok(n) => {
                clear_waker(&mut this.read_waker);
                Poll::Ready(Ok(n))
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                register_and_wake_waker(&mut this.read_waker, cx);
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(map_io_error(e))),
        }
    }
}

impl AsyncWrite for AsyncTcpStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<IoResult<usize>> {
        let this = self.get_mut();
        match this.inner.write(buf) {
            Ok(n) => {
                clear_waker(&mut this.write_waker);
                Poll::Ready(Ok(n))
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                register_and_wake_waker(&mut this.write_waker, cx);
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(map_io_error(e))),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        let this = self.get_mut();
        match this.inner.flush() {
            Ok(()) => {
                clear_waker(&mut this.flush_waker);
                Poll::Ready(Ok(()))
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                register_and_wake_waker(&mut this.flush_waker, cx);
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
    waker: Option<Waker>,
}

impl ConnectFuture {
    pub fn new<A: ToSocketAddrs>(addrs: A) -> Self {
        Self {
            addrs: addrs
                .to_socket_addrs()
                .map(|addrs| addrs.collect())
                .unwrap_or_default(),
            idx: 0,
            waker: None,
        }
    }
}

impl Future for ConnectFuture {
    type Output = std::io::Result<Arc<AsyncTcpStream>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        while let Some(addr) = this.addrs.get(this.idx).copied() {
            this.idx += 1;
            match StdTcpStream::connect(addr).and_then(AsyncTcpStream::from_std) {
                Ok(stream) => {
                    this.waker = None;
                    return Poll::Ready(Ok(Arc::new(stream)));
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    register_and_wake_waker(&mut this.waker, cx);
                    return Poll::Pending;
                }
                Err(_) => continue,
            }
        }
        this.waker = None;
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
        AcceptFuture {
            listener: self,
            waker: None,
        }
    }
}

pub struct AcceptFuture<'a> {
    listener: &'a AsyncTcpListener,
    waker: Option<Waker>,
}

impl Future for AcceptFuture<'_> {
    type Output = std::io::Result<(Arc<AsyncTcpStream>, SocketAddr)>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        match this.listener.inner.accept() {
            Ok((stream, addr)) => match AsyncTcpStream::from_std(stream) {
                Ok(stream) => {
                    this.waker = None;
                    Poll::Ready(Ok((Arc::new(stream), addr)))
                }
                Err(e) => {
                    this.waker = None;
                    Poll::Ready(Err(e))
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                register_and_wake_waker(&mut this.waker, cx);
                Poll::Pending
            }
            Err(e) => {
                this.waker = None;
                Poll::Ready(Err(e))
            }
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
        loop {
            match self.inner.send_to(buf, target) {
                Ok(n) => return Ok(n),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    crate::yieldnow().await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    pub async fn recv_from(&self, buf: &mut [u8]) -> std::io::Result<(usize, SocketAddr)> {
        loop {
            match self.inner.recv_from(buf) {
                Ok(result) => return Ok(result),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    crate::yieldnow().await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    pub fn send(&self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.send(buf)
    }

    pub fn recv(&self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.recv(buf)
    }
}
