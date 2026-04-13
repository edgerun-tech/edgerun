//! Non-blocking TCP with async I/O.
//!
//! ## Fixes applied:
//! - `poll_write` no longer double-registers waker (was: stored locally AND called reactor)
//! - Legacy `TcpStream`/`TcpListener` removed — only `AsyncTcpStream`/`AsyncTcpListener`
//! - `split()` returns halves that share fd via Arc, with independent wakers

use std::future::Future;
use std::io::{self};
use std::net::{SocketAddr, ToSocketAddrs};
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use crate::sync::Mutex;
use std::task::{Context, Poll, Waker};

use crate::io_traits::{AsyncRead, AsyncWrite};
use crate::runtime::{current_rt, try_current_rt};

// ===========================================================================
// AsyncTcpStream
// ===========================================================================

/// Async TCP stream. Read and write use independent wakers.
///
/// The fd is managed via `Arc` reference counting — `Clone` shares the fd,
/// the last drop closes it.
pub struct AsyncTcpStream {
    fd: RawFd,
    read_waker: Mutex<Option<Waker>>,
    write_waker: Mutex<Option<Waker>>,
    /// Reference count — the last drop closes the fd.
    refs: Arc<AtomicUsize>,
}

impl AsyncTcpStream {
    /// Wraps an already-connected, non-blocking fd.
    /// Takes ownership — do not close it manually.
    pub fn from_fd(fd: RawFd) -> Self {
        Self {
            fd,
            read_waker: Mutex::new(None),
            write_waker: Mutex::new(None),
            refs: Arc::new(AtomicUsize::new(1)),
        }
    }

    /// Wraps an already-connected `std::net::TcpStream`.
    /// Sets non-blocking and takes ownership.
    pub fn from_std(stream: std::net::TcpStream) -> io::Result<Self> {
        stream.set_nonblocking(true)?;
        let raw = stream.as_raw_fd();
        std::mem::forget(stream);
        Ok(Self {
            fd: raw,
            read_waker: Mutex::new(None),
            write_waker: Mutex::new(None),
            refs: Arc::new(AtomicUsize::new(1)),
        })
    }

    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        unsafe {
            let mut storage: std::mem::MaybeUninit<libc::sockaddr_storage> =
                std::mem::MaybeUninit::zeroed();
            let mut len =
                std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
            let res = libc::getpeername(
                self.fd,
                storage.as_mut_ptr() as *mut libc::sockaddr,
                &mut len,
            );
            if res < 0 {
                return Err(io::Error::last_os_error());
            }
            let storage = storage.assume_init();
            sockaddr_to_addr(&storage, len)
        }
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        unsafe {
            let mut storage: std::mem::MaybeUninit<libc::sockaddr_storage> =
                std::mem::MaybeUninit::zeroed();
            let mut len =
                std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
            let res = libc::getsockname(
                self.fd,
                storage.as_mut_ptr() as *mut libc::sockaddr,
                &mut len,
            );
            if res < 0 {
                return Err(io::Error::last_os_error());
            }
            let storage = storage.assume_init();
            sockaddr_to_addr(&storage, len)
        }
    }

    pub fn shutdown_write(&self) -> io::Result<()> {
        let res = unsafe { libc::shutdown(self.fd, libc::SHUT_WR) };
        if res < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    /// Returns the value of the `TCP_NODELAY` option on this socket.
    pub fn nodelay(&self) -> io::Result<bool> {
        let mut opt: libc::c_int = 0;
        let mut optlen: libc::socklen_t = std::mem::size_of::<libc::c_int>() as _;
        let res = unsafe {
            libc::getsockopt(
                self.fd,
                libc::IPPROTO_TCP,
                libc::TCP_NODELAY,
                &mut opt as *mut _ as *mut _,
                &mut optlen,
            )
        };
        if res < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(opt != 0)
        }
    }

    /// Sets the value of the `TCP_NODELAY` option on this socket.
    pub fn set_nodelay(&self, nodelay: bool) -> io::Result<()> {
        let opt: libc::c_int = if nodelay { 1 } else { 0 };
        let res = unsafe {
            libc::setsockopt(
                self.fd,
                libc::IPPROTO_TCP,
                libc::TCP_NODELAY,
                &opt as *const _ as *const _,
                std::mem::size_of_val(&opt) as libc::socklen_t,
            )
        };
        if res < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    /// Returns the value of the `IP_TTL` option on this socket.
    pub fn ttl(&self) -> io::Result<u32> {
        let mut opt: libc::c_int = 0;
        let mut optlen: libc::socklen_t = std::mem::size_of::<libc::c_int>() as _;
        let res = unsafe {
            libc::getsockopt(
                self.fd,
                libc::IPPROTO_IP,
                libc::IP_TTL,
                &mut opt as *mut _ as *mut _,
                &mut optlen,
            )
        };
        if res < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(opt as u32)
        }
    }

    /// Sets the value of the `IP_TTL` option on this socket.
    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        let opt = ttl as libc::c_int;
        let res = unsafe {
            libc::setsockopt(
                self.fd,
                libc::IPPROTO_IP,
                libc::IP_TTL,
                &opt as *const _ as *const _,
                std::mem::size_of_val(&opt) as libc::socklen_t,
            )
        };
        if res < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    /// Raw fd — do not close it manually.
    pub fn as_raw_fd(&self) -> RawFd {
        self.fd
    }

    /// Split into separate read and write halves.
    /// Both halves share the fd via Arc refcount.
    pub fn split(self: &Arc<Self>) -> (AsyncReadHalf, AsyncWriteHalf) {
        self.refs.fetch_add(2, Ordering::Relaxed);
        (
            AsyncReadHalf { inner: Arc::clone(self) },
            AsyncWriteHalf { inner: Arc::clone(self) },
        )
    }

    // Internal: register with reactor for read.
    fn reactor_wait_read(&self, waker: Waker) {
        if let Some(rt) = try_current_rt() {
            rt.reactor.wait_read(self.fd, waker);
        }
    }

    // Internal: register with reactor for write.
    fn reactor_wait_write(&self, waker: Waker) {
        if let Some(rt) = try_current_rt() {
            rt.reactor.wait_write(self.fd, waker);
        }
    }
}

impl Clone for AsyncTcpStream {
    fn clone(&self) -> Self {
        self.refs.fetch_add(1, Ordering::Relaxed);
        Self {
            fd: self.fd,
            read_waker: Mutex::new(None),
            write_waker: Mutex::new(None),
            refs: Arc::clone(&self.refs),
        }
    }
}

impl Drop for AsyncTcpStream {
    fn drop(&mut self) {
        if self.refs.fetch_sub(1, Ordering::AcqRel) == 1 {
            unsafe { libc::close(self.fd) };
        }
    }
}

impl Unpin for AsyncTcpStream {}

// ===========================================================================
// AsyncRead / AsyncWrite for Arc<AsyncTcpStream>
// ===========================================================================

impl AsyncRead for Arc<AsyncTcpStream> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        unsafe {
            let fd = self.fd;
            let slice =
                std::slice::from_raw_parts_mut(buf.as_mut_ptr(), buf.len());
            let n = libc::read(fd, slice.as_mut_ptr() as *mut libc::c_void, buf.len());
            if n < 0 {
                let e = io::Error::last_os_error();
                if e.kind() == io::ErrorKind::WouldBlock {
                    self.reactor_wait_read(cx.waker().clone());
                    Poll::Pending
                } else {
                    Poll::Ready(Err(e))
                }
            } else if n == 0 {
                Poll::Ready(Ok(0))
            } else {
                Poll::Ready(Ok(n as usize))
            }
        }
    }
}

impl AsyncWrite for Arc<AsyncTcpStream> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        unsafe {
            let fd = self.fd;
            let n =
                libc::write(fd, buf.as_ptr() as *const libc::c_void, buf.len());
            if n < 0 {
                let e = io::Error::last_os_error();
                if e.kind() == io::ErrorKind::WouldBlock {
                    // FIX: only register with reactor — no local waker storage.
                    // The reactor's FdInterest tracks the waker for this fd.
                    self.reactor_wait_write(cx.waker().clone());
                    Poll::Pending
                } else {
                    Poll::Ready(Err(e))
                }
            } else {
                Poll::Ready(Ok(n as usize))
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        unsafe {
            let res = libc::shutdown(self.fd, libc::SHUT_WR);
            if res < 0 {
                Poll::Ready(Err(io::Error::last_os_error()))
            } else {
                Poll::Ready(Ok(()))
            }
        }
    }
}

// Also impl directly on AsyncTcpStream for owned use.
impl AsyncRead for AsyncTcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        unsafe {
            let fd = self.fd;
            let slice =
                std::slice::from_raw_parts_mut(buf.as_mut_ptr(), buf.len());
            let n = libc::read(fd, slice.as_mut_ptr() as *mut libc::c_void, buf.len());
            if n < 0 {
                let e = io::Error::last_os_error();
                if e.kind() == io::ErrorKind::WouldBlock {
                    self.reactor_wait_read(cx.waker().clone());
                    Poll::Pending
                } else {
                    Poll::Ready(Err(e))
                }
            } else if n == 0 {
                Poll::Ready(Ok(0))
            } else {
                Poll::Ready(Ok(n as usize))
            }
        }
    }
}

impl AsyncWrite for AsyncTcpStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        unsafe {
            let fd = self.fd;
            let n =
                libc::write(fd, buf.as_ptr() as *const libc::c_void, buf.len());
            if n < 0 {
                let e = io::Error::last_os_error();
                if e.kind() == io::ErrorKind::WouldBlock {
                    self.reactor_wait_write(cx.waker().clone());
                    Poll::Pending
                } else {
                    Poll::Ready(Err(e))
                }
            } else {
                Poll::Ready(Ok(n as usize))
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        unsafe {
            let res = libc::shutdown(self.fd, libc::SHUT_WR);
            if res < 0 {
                Poll::Ready(Err(io::Error::last_os_error()))
            } else {
                Poll::Ready(Ok(()))
            }
        }
    }
}

// ===========================================================================
// Read/Write halves
// ===========================================================================

pub struct AsyncReadHalf {
    inner: Arc<AsyncTcpStream>,
}
pub struct AsyncWriteHalf {
    inner: Arc<AsyncTcpStream>,
}

impl AsyncRead for AsyncReadHalf {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        unsafe {
            let fd = self.inner.fd;
            let slice =
                std::slice::from_raw_parts_mut(buf.as_mut_ptr(), buf.len());
            let n = libc::read(fd, slice.as_mut_ptr() as *mut libc::c_void, buf.len());
            if n < 0 {
                let e = io::Error::last_os_error();
                if e.kind() == io::ErrorKind::WouldBlock {
                    self.inner.reactor_wait_read(cx.waker().clone());
                    Poll::Pending
                } else {
                    Poll::Ready(Err(e))
                }
            } else if n == 0 {
                Poll::Ready(Ok(0))
            } else {
                Poll::Ready(Ok(n as usize))
            }
        }
    }
}

impl AsyncWrite for AsyncWriteHalf {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        unsafe {
            let fd = self.inner.fd;
            let n =
                libc::write(fd, buf.as_ptr() as *const libc::c_void, buf.len());
            if n < 0 {
                let e = io::Error::last_os_error();
                if e.kind() == io::ErrorKind::WouldBlock {
                    self.inner.reactor_wait_write(cx.waker().clone());
                    Poll::Pending
                } else {
                    Poll::Ready(Err(e))
                }
            } else {
                Poll::Ready(Ok(n as usize))
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        unsafe {
            let res = libc::shutdown(self.inner.fd, libc::SHUT_WR);
            if res < 0 {
                Poll::Ready(Err(io::Error::last_os_error()))
            } else {
                Poll::Ready(Ok(()))
            }
        }
    }
}

impl Unpin for AsyncReadHalf {}
impl Unpin for AsyncWriteHalf {}

// ===========================================================================
// ConnectFuture — fully non-blocking connect
// ===========================================================================

/// Non-blocking TCP connect future.
pub struct ConnectFuture {
    state: ConnectState,
}

enum ConnectState {
    /// Still resolving addresses.
    Resolving {
        addrs: Vec<SocketAddr>,
        idx: usize,
    },
    /// Connect in progress on the given fd, with remaining addresses to try
    /// if this one fails.
    Connecting { fd: RawFd, remaining: Vec<SocketAddr> },
    /// Done.
    Done,
}

impl ConnectFuture {
    pub fn new<A: ToSocketAddrs>(addrs: A) -> Self {
        let addrs = addrs.to_socket_addrs().map(|a| a.collect()).unwrap_or_default();
        Self {
            state: ConnectState::Resolving { addrs, idx: 0 },
        }
    }
}

impl Future for ConnectFuture {
    type Output = io::Result<Arc<AsyncTcpStream>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };

        loop {
            match &mut this.state {
                ConnectState::Resolving { addrs, idx } => {
                    if *idx >= addrs.len() {
                        this.state = ConnectState::Done;
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::ConnectionRefused,
                            "all addresses refused",
                        )));
                    }

                    let addr = addrs[*idx];
                    *idx += 1;

                    // Create socket with appropriate address family for IPv6 support
                    let family = match addr {
                        SocketAddr::V4(_) => libc::AF_INET,
                        SocketAddr::V6(_) => libc::AF_INET6,
                    };
                    let fd = unsafe {
                        libc::socket(
                            family,
                            libc::SOCK_STREAM | libc::SOCK_NONBLOCK,
                            0,
                        )
                    };
                    if fd < 0 {
                        return Poll::Ready(Err(io::Error::last_os_error()));
                    }

                    let sock_addr = socket_addr_to_sockaddr(&addr);
                    let res = unsafe {
                        libc::connect(
                            fd,
                            sock_addr.as_ptr(),
                            sock_addr.len(),
                        )
                    };

                    if res == 0 {
                        return Poll::Ready(Ok(Arc::new(AsyncTcpStream::from_fd(fd))));
                    }

                    let err = io::Error::last_os_error();
                    if err.kind() == io::ErrorKind::WouldBlock
                        || err.raw_os_error() == Some(libc::EINPROGRESS)
                        || err.raw_os_error() == Some(libc::EALREADY)
                    {
                        // Save remaining addresses to try if connect fails.
                        let remaining = addrs[*idx..].to_vec();
                        this.state = ConnectState::Connecting { fd, remaining };
                        if let Some(rt) = try_current_rt() {
                            rt.reactor.wait_connect(fd, cx.waker().clone());
                        }
                        return Poll::Pending;
                    }

                    // Connection refused — try next address.
                    unsafe { libc::close(fd) };
                }

                ConnectState::Connecting { fd, remaining } => {
                    let current_fd = *fd;
                    let remaining = std::mem::take(remaining);

                    let mut error: libc::c_int = 0;
                    let mut len =
                        std::mem::size_of::<libc::c_int>() as libc::socklen_t;
                    let res = unsafe {
                        libc::getsockopt(
                            current_fd,
                            libc::SOL_SOCKET,
                            libc::SO_ERROR,
                            &mut error as *mut _ as *mut libc::c_void,
                            &mut len,
                        )
                    };

                    if res == 0 && error == 0 {
                        this.state = ConnectState::Done;
                        return Poll::Ready(Ok(Arc::new(AsyncTcpStream::from_fd(
                            current_fd,
                        ))));
                    }

                    unsafe { libc::close(current_fd) };

                    // Try remaining addresses.
                    if remaining.is_empty() {
                        this.state = ConnectState::Done;
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::ConnectionRefused,
                            "connect failed",
                        )));
                    }
                    this.state = ConnectState::Resolving {
                        addrs: remaining,
                        idx: 0,
                    };
                    // Continue the loop to try next address.
                }

                ConnectState::Done => {
                    return Poll::Ready(Err(io::Error::other(
                        "connect future polled after completion",
                    )));
                }
            }
        }
    }
}

fn socket_addr_to_sockaddr(addr: &SocketAddr) -> SocketAddrStorage {
    match addr {
        SocketAddr::V4(v4) => {
            let sin = libc::sockaddr_in {
                sin_family: libc::AF_INET as u16,
                sin_port: v4.port().to_be(),
                sin_addr: libc::in_addr {
                    s_addr: u32::from_ne_bytes(v4.ip().octets()),
                },
                sin_zero: [0; 8],
            };
            let mut storage = SocketAddrStorage::new();
            unsafe {
                std::ptr::write(storage.0.as_mut_ptr() as *mut libc::sockaddr_in, sin);
                storage.1 = std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;
            }
            storage
        }
        SocketAddr::V6(v6) => {
            let sin6 = libc::sockaddr_in6 {
                sin6_family: libc::AF_INET6 as u16,
                sin6_port: v6.port().to_be(),
                sin6_addr: libc::in6_addr {
                    s6_addr: v6.ip().octets(),
                },
                sin6_flowinfo: v6.flowinfo(),
                sin6_scope_id: v6.scope_id(),
            };
            let mut storage = SocketAddrStorage::new();
            unsafe {
                std::ptr::write(storage.0.as_mut_ptr() as *mut libc::sockaddr_in6, sin6);
                storage.1 = std::mem::size_of::<libc::sockaddr_in6>() as libc::socklen_t;
            }
            storage
        }
    }
}

/// Helper to hold sockaddr storage with length
struct SocketAddrStorage(
    std::mem::MaybeUninit<libc::sockaddr_storage>,
    libc::socklen_t,
);

impl SocketAddrStorage {
    fn new() -> Self {
        Self(std::mem::MaybeUninit::zeroed(), 0)
    }

    fn as_ptr(&self) -> *const libc::sockaddr {
        self.0.as_ptr() as *const libc::sockaddr
    }

    fn len(&self) -> libc::socklen_t {
        self.1
    }
}

// ===========================================================================
// AsyncTcpListener
// ===========================================================================

/// Async TCP listener with non-blocking accept.
pub struct AsyncTcpListener {
    inner: std::net::TcpListener,
    fd: RawFd,
}

impl AsyncTcpListener {
    pub fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        let listener = std::net::TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        let fd = listener.as_raw_fd();
        Ok(Self { inner: listener, fd })
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.inner.local_addr()
    }

    pub fn accept(&self) -> AcceptFuture<'_> {
        AcceptFuture { listener: self }
    }

    /// Create from an already-registered, non-blocking raw fd.
    ///
    /// # Safety
    /// The fd must be in non-blocking mode and already registered
    /// with the reactor.
    pub(crate) fn from_fd(fd: RawFd) -> Self {
        Self {
            inner: unsafe { std::net::TcpListener::from_raw_fd(fd) },
            fd,
        }
    }
}

pub struct AcceptFuture<'a> {
    listener: &'a AsyncTcpListener,
}

impl Future for AcceptFuture<'_> {
    type Output = io::Result<(Arc<AsyncTcpStream>, SocketAddr)>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.listener.inner.accept() {
            Ok((stream, addr)) => {
                stream.set_nonblocking(true)?;
                let raw = stream.as_raw_fd();
                std::mem::forget(stream);
                let async_stream = Arc::new(AsyncTcpStream::from_fd(raw));
                // Register the accepted fd with the reactor.
                if let Some(rt) = try_current_rt() {
                    rt.reactor.get_or_register_fd(raw);
                }
                Poll::Ready(Ok((async_stream, addr)))
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                if let Some(rt) = try_current_rt() {
                    rt.reactor.wait_read(self.listener.fd, cx.waker().clone());
                }
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

// ===========================================================================
// Address conversion helper
// ===========================================================================

fn fd_local_addr(fd: RawFd) -> io::Result<SocketAddr> {
    unsafe {
        let mut storage: std::mem::MaybeUninit<libc::sockaddr_storage> =
            std::mem::MaybeUninit::zeroed();
        let mut len =
            std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
        let res = libc::getsockname(
            fd,
            storage.as_mut_ptr() as *mut libc::sockaddr,
            &mut len,
        );
        if res < 0 {
            return Err(io::Error::last_os_error());
        }
        let storage = storage.assume_init();
        sockaddr_to_addr(&storage, len)
    }
}

fn fd_peer_addr(fd: RawFd) -> io::Result<SocketAddr> {
    unsafe {
        let mut storage: std::mem::MaybeUninit<libc::sockaddr_storage> =
            std::mem::MaybeUninit::zeroed();
        let mut len =
            std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
        let res = libc::getpeername(
            fd,
            storage.as_mut_ptr() as *mut libc::sockaddr,
            &mut len,
        );
        if res < 0 {
            return Err(io::Error::last_os_error());
        }
        let storage = storage.assume_init();
        sockaddr_to_addr(&storage, len)
    }
}

fn sockaddr_to_addr(
    storage: &libc::sockaddr_storage,
    len: libc::socklen_t,
) -> io::Result<SocketAddr> {
    unsafe {
        let family = storage.ss_family;
        if family as i32 == libc::AF_INET {
            let sin: libc::sockaddr_in =
                std::ptr::read(storage as *const _ as *const libc::sockaddr_in);
            Ok(SocketAddr::V4(std::net::SocketAddrV4::new(
                std::net::Ipv4Addr::from(sin.sin_addr.s_addr.to_ne_bytes()),
                u16::from_be(sin.sin_port),
            )))
        } else if family as i32 == libc::AF_INET6 {
            let sin6: libc::sockaddr_in6 =
                std::ptr::read(storage as *const _ as *const libc::sockaddr_in6);
            Ok(SocketAddr::V6(std::net::SocketAddrV6::new(
                std::net::Ipv6Addr::from(sin6.sin6_addr.s6_addr),
                u16::from_be(sin6.sin6_port),
                sin6.sin6_flowinfo,
                sin6.sin6_scope_id,
            )))
        } else {
            Err(io::Error::other(
                "unsupported address family",
            ))
        }
    }
}
