//! Async Unix domain sockets — local IPC via filesystem paths.
//!
//! ## Architecture
//! Mirrors TCP/UDP but uses `AF_UNIX` addresses (paths).
//! - `UnixStream`: connected byte stream between two local endpoints.
//! - `UnixListener`: accepts incoming UnixStream connections.
//! - `UnixDatagram`: connectionless datagram socket.

use std::future::Future;
use std::io::{self};
use std::os::unix::net::UnixListener as StdUnixListener;
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};

use crate::io_traits::{AsyncRead, AsyncWrite};
use crate::runtime::try_current_rt;

// ===========================================================================
// UnixStream
// ===========================================================================

/// Async Unix domain stream socket.
pub struct UnixStream {
    fd: RawFd,
    refs: Arc<AtomicUsize>,
}

impl UnixStream {
    /// Wraps an already-connected, non-blocking fd.
    pub fn from_fd(fd: RawFd) -> Self {
        Self {
            fd,
            refs: Arc::new(AtomicUsize::new(1)),
        }
    }

    /// Connect to a Unix socket path asynchronously.
    ///
    /// Uses non-blocking connect to avoid blocking worker threads.
    pub fn connect<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let fd = unsafe {
            libc::socket(libc::AF_UNIX, libc::SOCK_STREAM | libc::SOCK_CLOEXEC, 0)
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }

        // Set non-blocking.
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 {
            unsafe { libc::close(fd) };
            return Err(io::Error::last_os_error());
        }
        if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
            unsafe { libc::close(fd) };
            return Err(io::Error::last_os_error());
        }

        // Build sockaddr_un.
        let path_bytes = path.as_ref().as_os_str().as_encoded_bytes();
        if path_bytes.len() > 107 {
            unsafe { libc::close(fd) };
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "path too long"));
        }
        let mut addr: libc::sockaddr_un = unsafe { std::mem::zeroed() };
        addr.sun_family = libc::AF_UNIX as _;
        unsafe {
            std::ptr::copy_nonoverlapping(
                path_bytes.as_ptr(),
                addr.sun_path.as_mut_ptr() as *mut u8,
                path_bytes.len(),
            );
        }
        let addrlen = std::mem::size_of::<libc::sockaddr_un>() as libc::socklen_t;

        let res = unsafe { libc::connect(fd, &addr as *const _ as *const _, addrlen) };
        if res < 0 {
            let e = io::Error::last_os_error();
            if e.kind() != io::ErrorKind::WouldBlock
                && e.raw_os_error() != Some(libc::EINPROGRESS)
            {
                unsafe { libc::close(fd) };
                return Err(e);
            }
            // Connect in progress — the caller should poll for write readiness.
        }

        Ok(Self {
            fd,
            refs: Arc::new(AtomicUsize::new(1)),
        })
    }

    /// Split into read and write halves.
    pub fn split(self: &Arc<Self>) -> (UnixReadHalf, UnixWriteHalf) {
        self.refs.fetch_add(2, Ordering::Relaxed);
        (
            UnixReadHalf { inner: Arc::clone(self) },
            UnixWriteHalf { inner: Arc::clone(self) },
        )
    }

    pub fn shutdown_write(&self) -> io::Result<()> {
        let res = unsafe { libc::shutdown(self.fd, libc::SHUT_WR) };
        if res < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub fn as_raw_fd(&self) -> RawFd {
        self.fd
    }

    /// Returns the local socket address.
    pub fn local_addr(&self) -> io::Result<std::os::unix::net::SocketAddr> {
        // Use std's UnixStream to get the address.
        // We can't reconstruct a std::UnixStream from a raw fd safely
        // (double-close risk). Use libc directly.
        unsafe {
            let mut addr: libc::sockaddr_storage = std::mem::zeroed();
            let mut addrlen: libc::socklen_t = std::mem::size_of::<libc::sockaddr_storage>() as _;
            let res = libc::getsockname(
                self.fd,
                &mut addr as *mut _ as *mut libc::sockaddr,
                &mut addrlen,
            );
            if res < 0 {
                return Err(io::Error::last_os_error());
            }
            if addr.ss_family as libc::c_int != libc::AF_UNIX {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a unix socket"));
            }
            let unix_addr = &addr as *const _ as *const libc::sockaddr_un;
            let path = std::ffi::CStr::from_ptr((*unix_addr).sun_path.as_ptr())
                .to_string_lossy()
                .into_owned();
            std::os::unix::net::SocketAddr::from_pathname(&path)
        }
    }

    /// Returns the peer socket address.
    pub fn peer_addr(&self) -> io::Result<std::os::unix::net::SocketAddr> {
        unsafe {
            let mut addr: libc::sockaddr_storage = std::mem::zeroed();
            let mut addrlen: libc::socklen_t = std::mem::size_of::<libc::sockaddr_storage>() as _;
            let res = libc::getpeername(
                self.fd,
                &mut addr as *mut _ as *mut libc::sockaddr,
                &mut addrlen,
            );
            if res < 0 {
                return Err(io::Error::last_os_error());
            }
            if addr.ss_family as libc::c_int != libc::AF_UNIX {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a unix socket"));
            }
            let unix_addr = &addr as *const _ as *const libc::sockaddr_un;
            let path = std::ffi::CStr::from_ptr((*unix_addr).sun_path.as_ptr())
                .to_string_lossy()
                .into_owned();
            std::os::unix::net::SocketAddr::from_pathname(&path)
        }
    }

    /// Creates a new independently owned handle to the underlying socket.
    pub fn try_clone(&self) -> io::Result<Self> {
        let new_fd = unsafe { libc::dup(self.fd) };
        if new_fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // Set non-blocking on the new fd.
        let flags = unsafe { libc::fcntl(new_fd, libc::F_GETFL) };
        if flags < 0 {
            return Err(io::Error::last_os_error());
        }
        if unsafe { libc::fcntl(new_fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
            return Err(io::Error::last_os_error());
        }
        // Don't register with reactor again — the new fd is a different
        // kernel fd number, so it needs its own reactor entry.
        if let Some(rt) = try_current_rt() {
            rt.reactor.get_or_register_fd(new_fd);
        }
        Ok(Self {
            fd: new_fd,
            refs: Arc::new(AtomicUsize::new(1)),
        })
    }

    fn reactor_wait_read(&self, waker: Waker) {
        if let Some(rt) = try_current_rt() {
            rt.reactor.wait_read(self.fd, waker);
        }
    }

    fn reactor_wait_write(&self, waker: Waker) {
        if let Some(rt) = try_current_rt() {
            rt.reactor.wait_write(self.fd, waker);
        }
    }
}

impl Clone for UnixStream {
    fn clone(&self) -> Self {
        self.refs.fetch_add(1, Ordering::Relaxed);
        Self {
            fd: self.fd,
            refs: Arc::clone(&self.refs),
        }
    }
}

impl Drop for UnixStream {
    fn drop(&mut self) {
        if self.refs.fetch_sub(1, Ordering::AcqRel) == 1 {
            unsafe { libc::close(self.fd) };
        }
    }
}

impl Unpin for UnixStream {}

// AsyncRead / AsyncWrite for Arc<UnixStream>
impl AsyncRead for Arc<UnixStream> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        unsafe {
            let fd = self.fd;
            let slice = std::slice::from_raw_parts_mut(buf.as_mut_ptr(), buf.len());
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

impl AsyncWrite for Arc<UnixStream> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        unsafe {
            let fd = self.fd;
            let n = libc::write(fd, buf.as_ptr() as *const libc::c_void, buf.len());
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

// AsyncRead / AsyncWrite for owned UnixStream
impl AsyncRead for UnixStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        unsafe {
            let fd = self.fd;
            let slice = std::slice::from_raw_parts_mut(buf.as_mut_ptr(), buf.len());
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

impl AsyncWrite for UnixStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        unsafe {
            let fd = self.fd;
            let n = libc::write(fd, buf.as_ptr() as *const libc::c_void, buf.len());
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

// Read/Write halves
pub struct UnixReadHalf {
    inner: Arc<UnixStream>,
}

pub struct UnixWriteHalf {
    inner: Arc<UnixStream>,
}

impl AsyncRead for UnixReadHalf {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        unsafe {
            let fd = self.inner.fd;
            let slice = std::slice::from_raw_parts_mut(buf.as_mut_ptr(), buf.len());
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

impl AsyncWrite for UnixWriteHalf {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        unsafe {
            let fd = self.inner.fd;
            let n = libc::write(fd, buf.as_ptr() as *const libc::c_void, buf.len());
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

impl Unpin for UnixReadHalf {}
impl Unpin for UnixWriteHalf {}

// ===========================================================================
// UnixListener
// ===========================================================================

/// Async Unix domain socket listener.
pub struct UnixListener {
    inner: StdUnixListener,
    fd: RawFd,
}

impl UnixListener {
    pub fn bind<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        // Clean up stale socket file.
        let _ = std::fs::remove_file(path.as_ref());
        let listener = StdUnixListener::bind(path)?;
        listener.set_nonblocking(true)?;
        let fd = listener.as_raw_fd();
        Ok(Self { inner: listener, fd })
    }

    pub fn accept(&self) -> UnixAcceptFuture<'_> {
        UnixAcceptFuture { listener: self }
    }
}

impl Drop for UnixListener {
    fn drop(&mut self) {
        // The socket file is intentionally left on disk.
        // Callers should clean up if needed.
    }
}

pub struct UnixAcceptFuture<'a> {
    listener: &'a UnixListener,
}

impl Future for UnixAcceptFuture<'_> {
    type Output = io::Result<Arc<UnixStream>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.listener.inner.accept() {
            Ok((stream, _addr)) => {
                stream.set_nonblocking(true)?;
                let raw = stream.as_raw_fd();
                std::mem::forget(stream);
                let async_stream = Arc::new(UnixStream::from_fd(raw));
                if let Some(rt) = try_current_rt() {
                    rt.reactor.get_or_register_fd(raw);
                }
                Poll::Ready(Ok(async_stream))
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
