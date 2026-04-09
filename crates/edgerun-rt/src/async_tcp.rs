//! Non-blocking TCP connect + async I/O with separate read/write wakers.

use std::future::Future;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, ToSocketAddrs};
use std::os::unix::io::{AsRawFd, RawFd};
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::task::{Context, Poll, Waker};

// ===========================================================================
// AsyncTcpStream
// ===========================================================================

/// Async TCP stream with separate read and write wakers.
///
/// Unlike `Arc<Mutex<StdTcp>>`, read and write operations use independent
/// waker registrations. A read waiting for data does not block a write,
/// and vice versa.
pub struct AsyncTcpStream {
    fd: RawFd,
    read_waker: Mutex<Option<Waker>>,
    write_waker: Mutex<Option<Waker>>,
    /// Reference count — the last drop closes the fd.
    refs: Arc<AtomicUsize>,
}

impl AsyncTcpStream {
    /// Wraps an already-connected, non-blocking fd.
    /// Takes ownership of the fd — do not close it manually.
    pub fn from_fd(fd: RawFd) -> Self {
        Self {
            fd,
            read_waker: Mutex::new(None),
            write_waker: Mutex::new(None),
            refs: Arc::new(AtomicUsize::new(1)),
        }
    }

    /// Wraps an already-connected, non-blocking `std::net::TcpStream`.
    /// The stream is consumed and its fd is taken over.
    pub fn from_std(stream: std::net::TcpStream) -> io::Result<Self> {
        stream.set_nonblocking(true)?;
        let fd = stream.as_raw_fd();
        // Convert to RawFd and prevent the stream from closing it on drop.
        let raw = stream.as_raw_fd(); std::mem::forget(stream);
        Ok(Self {
            fd: raw,
            read_waker: Mutex::new(None),
            write_waker: Mutex::new(None),
            refs: Arc::new(AtomicUsize::new(1)),
        })
    }

    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        unsafe {
            let mut storage: std::mem::MaybeUninit<libc::sockaddr_storage> = std::mem::MaybeUninit::zeroed();
            let mut len = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
            let res = libc::getpeername(self.fd, storage.as_mut_ptr() as *mut libc::sockaddr, &mut len);
            if res < 0 {
                return Err(io::Error::last_os_error());
            }
            let storage = storage.assume_init();
            if storage.ss_family as i32 == libc::AF_INET {
                let sin: libc::sockaddr_in = std::ptr::read(&storage as *const _ as *const libc::sockaddr_in);
                Ok(SocketAddr::V4(std::net::SocketAddrV4::new(
                    std::net::Ipv4Addr::from(sin.sin_addr.s_addr.to_ne_bytes()),
                    u16::from_be(sin.sin_port),
                )))
            } else {
                Err(io::Error::new(io::ErrorKind::Other, "only IPv4 supported"))
            }
        }
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        unsafe {
            let mut storage: std::mem::MaybeUninit<libc::sockaddr_storage> = std::mem::MaybeUninit::zeroed();
            let mut len = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
            let res = libc::getsockname(self.fd, storage.as_mut_ptr() as *mut libc::sockaddr, &mut len);
            if res < 0 {
                return Err(io::Error::last_os_error());
            }
            let storage = storage.assume_init();
            if storage.ss_family as i32 == libc::AF_INET {
                let sin: libc::sockaddr_in = std::ptr::read(&storage as *const _ as *const libc::sockaddr_in);
                Ok(SocketAddr::V4(std::net::SocketAddrV4::new(
                    std::net::Ipv4Addr::from(sin.sin_addr.s_addr.to_ne_bytes()),
                    u16::from_be(sin.sin_port),
                )))
            } else {
                Err(io::Error::new(io::ErrorKind::Other, "only IPv4 supported"))
            }
        }
    }

    pub fn shutdown_write(&self) -> io::Result<()> {
        let res = unsafe { libc::shutdown(self.fd, libc::SHUT_WR) };
        if res < 0 { Err(io::Error::last_os_error()) } else { Ok(()) }
    }

    /// Raw fd — do not close it manually.
    pub fn as_raw_fd(&self) -> RawFd { self.fd }

    pub fn clear_read(&self) { self.read_waker.lock().unwrap().take(); }
    pub fn clear_write(&self) { self.write_waker.lock().unwrap().take(); }
    pub fn wait_read(&self, waker: Waker) { *self.read_waker.lock().unwrap() = Some(waker); }
    pub fn wait_write(&self, waker: Waker) { *self.write_waker.lock().unwrap() = Some(waker); }
}

impl Drop for AsyncTcpStream {
    fn drop(&mut self) {
        if self.refs.fetch_sub(1, Ordering::AcqRel) == 1 {
            unsafe { libc::close(self.fd); }
        }
    }
}

// ===========================================================================
// ConnectFuture — fully non-blocking connect
// ===========================================================================

/// Non-blocking TCP connect future.
///
/// Uses raw sockets with `SOCK_NONBLOCK` so the connect never blocks
/// a worker thread. The reactor dispatches the completion via epoll.
pub struct ConnectFuture {
    state: ConnectState,
}

enum ConnectState {
    /// Still resolving addresses.
    Resolving {
        addrs: Vec<SocketAddr>,
        idx: usize,
    },
    /// Connect in progress on the given fd.
    Connecting { fd: RawFd },
    /// Done (success or failure).
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

                    // Create non-blocking socket.
                    let fd = unsafe {
                        libc::socket(libc::AF_INET, libc::SOCK_STREAM | libc::SOCK_NONBLOCK, 0)
                    };
                    if fd < 0 {
                        return Poll::Ready(Err(io::Error::last_os_error()));
                    }

                    // Start connect.
                    let sock_addr = socket_addr_to_sockaddr_in(&addr);
                    let res = unsafe {
                        libc::connect(
                            fd,
                            &sock_addr as *const _ as *const libc::sockaddr,
                            std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
                        )
                    };

                    if res == 0 {
                        // Connected immediately (loopback).
                        return Poll::Ready(Ok(Arc::new(AsyncTcpStream::from_fd(fd))));
                    }

                    let err = io::Error::last_os_error();
                    if err.kind() == io::ErrorKind::WouldBlock
                        || err.raw_os_error() == Some(libc::EINPROGRESS)
                        || err.raw_os_error() == Some(libc::EALREADY)
                    {
                        // Connect in progress.
                        this.state = ConnectState::Connecting { fd };

                        // Register with the reactor for write readiness.
                        crate::register_connecting_fd(fd, cx.waker().clone());

                        return Poll::Pending;
                    }

                    // Connection refused — try next address.
                    unsafe { libc::close(fd); }
                    // Continue loop to try next address.
                }

                ConnectState::Connecting { fd } => {
                    let current_fd = *fd;

                    // Check if connect succeeded by trying a zero-length connect
                    // or checking SO_ERROR.
                    let mut error: libc::c_int = 0;
                    let mut len = std::mem::size_of::<libc::c_int>() as libc::socklen_t;
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
                        // Connection succeeded.
                        this.state = ConnectState::Done;
                        return Poll::Ready(Ok(Arc::new(AsyncTcpStream::from_fd(current_fd))));
                    }

                    // Connection failed — close fd and try next address.
                    unsafe { libc::close(current_fd); }
                    this.state = ConnectState::Resolving {
                        addrs: Vec::new(), // No more addresses — we consumed them.
                        idx: 0,
                    };
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::ConnectionRefused,
                        "connect failed",
                    )));
                }

                ConnectState::Done => {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::Other,
                        "connect future polled after completion",
                    )));
                }
            }
        }
    }
}

fn socket_addr_to_sockaddr_in(addr: &SocketAddr) -> libc::sockaddr_in {
    match addr {
        SocketAddr::V4(v4) => libc::sockaddr_in {
            sin_family: libc::AF_INET as u16,
            sin_port: v4.port().to_be(),
            sin_addr: libc::in_addr {
                s_addr: u32::from_ne_bytes(v4.ip().octets()),
            },
            sin_zero: [0; 8],
        },
        SocketAddr::V6(_) => {
            // Fallback — real impl would use sockaddr_in6.
            libc::sockaddr_in {
                sin_family: libc::AF_INET as u16,
                sin_port: 0,
                sin_addr: libc::in_addr { s_addr: 0 },
                sin_zero: [0; 8],
            }
        }
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
}

pub struct AcceptFuture<'a> { listener: &'a AsyncTcpListener }

impl Future for AcceptFuture<'_> {
    type Output = io::Result<(Arc<AsyncTcpStream>, SocketAddr)>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.listener.inner.accept() {
            Ok((stream, addr)) => {
                stream.set_nonblocking(true)?;
                let raw = stream.as_raw_fd(); std::mem::forget(stream);
                let async_stream = Arc::new(AsyncTcpStream::from_fd(raw));
                Poll::Ready(Ok((async_stream, addr)))
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                crate::register_fd_read(self.listener.fd, cx.waker().clone());
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}
