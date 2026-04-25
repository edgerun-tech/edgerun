//! AsyncFd — wrap any raw file descriptor with async readiness.
//!
//! The core escape hatch: register an fd with the reactor and poll it
//! for read/write readiness. Used by higher-level types (TCP, UDP, Unix
//! sockets) and for custom I/O (pipes, TTYs, eventfd, inotify, etc.).

use std::future::Future;
use std::io;
use std::os::unix::io::{AsRawFd, RawFd};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::runtime::current_rt;

/// Wraps a raw file descriptor with async readiness polling.
///
/// The fd must already be in non-blocking mode. `AsyncFd` does **not**
/// perform any I/O — it only waits until the fd is ready for read or
/// write, then returns so the caller can call `read`/`write`/`recv`/`send`
/// on the underlying fd.
pub struct AsyncFd<F> {
    fd: F,
}

impl<F: AsRawFd> AsyncFd<F> {
    /// Create a new `AsyncFd` from an existing I/O handle.
    ///
    /// The underlying fd must be in non-blocking mode.
    pub fn new(fd: F) -> io::Result<Self> {
        let rt = current_rt();
        rt.reactor.get_or_register_fd(fd.as_raw_fd());
        Ok(Self { fd })
    }

    /// Get a reference to the inner I/O handle.
    pub fn get_ref(&self) -> &F {
        &self.fd
    }

    /// Get a mutable reference to the inner I/O handle.
    pub fn get_mut(&mut self) -> &mut F {
        &mut self.fd
    }

    /// Consume the wrapper and return the inner I/O handle.
    pub fn into_inner(self) -> F {
        self.fd
    }

    /// Wait until the fd is ready for reading.
    pub fn readable(&self) -> ReadyFuture {
        ReadyFuture {
            fd: self.fd.as_raw_fd(),
            dir: Direction::Read,
        }
    }

    /// Wait until the fd is ready for writing.
    pub fn writable(&self) -> ReadyFuture {
        ReadyFuture {
            fd: self.fd.as_raw_fd(),
            dir: Direction::Write,
        }
    }

    /// Poll for read readiness.
    pub fn poll_read_ready(&self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        poll_ready(self.fd.as_raw_fd(), true, cx)
    }

    /// Poll for write readiness.
    pub fn poll_write_ready(&self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        poll_ready(self.fd.as_raw_fd(), false, cx)
    }

    /// Execute a read closure on this fd.
    ///
    /// This is a direct passthrough to the closure — no automatic
    /// EAGAIN handling. If the closure returns `EAGAIN`/`EWOULDBLOCK`,
    /// the caller should await [`Self::readable()`] before retrying.
    pub fn try_io<R, C>(&self, f: C) -> io::Result<R>
    where
        C: FnOnce() -> io::Result<R>,
    {
        f()
    }

    /// Execute a write closure on this fd.
    ///
    /// This is a direct passthrough to the closure — no automatic
    /// EAGAIN handling. If the closure returns `EAGAIN`/`EWOULDBLOCK`,
    /// the caller should await [`Self::writable()`] before retrying.
    pub fn try_io_mut<R, C>(&self, f: C) -> io::Result<R>
    where
        C: FnOnce() -> io::Result<R>,
    {
        f()
    }
}

impl<F: AsRawFd> AsRawFd for AsyncFd<F> {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

impl<F: AsRawFd + std::fmt::Debug> std::fmt::Debug for AsyncFd<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsyncFd").field("inner", &self.fd).finish()
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Direction {
    Read,
    Write,
}

/// Future returned by [`AsyncFd::readable()`], [`AsyncFd::writable()`].
pub struct ReadyFuture {
    fd: RawFd,
    dir: Direction,
}

impl Future for ReadyFuture {
    type Output = io::Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        poll_ready(self.fd, self.dir == Direction::Read, cx)
    }
}

fn poll_ready(fd: RawFd, read: bool, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
    let rt = current_rt();

    // Immediate readiness check via poll(2) with timeout=0.
    let mut pfd = libc::pollfd {
        fd,
        events: if read {
            libc::POLLIN as _
        } else {
            libc::POLLOUT as _
        },
        revents: 0,
    };
    let res = unsafe { libc::poll(&mut pfd, 1, 0) };
    if res > 0 && (pfd.revents & pfd.events) != 0 {
        return Poll::Ready(Ok(()));
    }

    // Not ready yet — register with reactor.
    if read {
        rt.reactor.wait_read(fd, cx.waker().clone());
    } else {
        rt.reactor.wait_write(fd, cx.waker().clone());
    }

    // Double-check after registering waker (race condition avoidance).
    let res = unsafe { libc::poll(&mut pfd, 1, 0) };
    if res > 0 && (pfd.revents & pfd.events) != 0 {
        return Poll::Ready(Ok(()));
    }

    Poll::Pending
}

// ===========================================================================
// OwnedFd
// ===========================================================================

/// An `AsyncFd` that owns its raw file descriptor (closes on drop).
pub struct OwnedAsyncFd {
    fd: RawFd,
}

impl OwnedAsyncFd {
    /// Create from a raw fd. The fd is closed on drop.
    pub fn from_raw_fd(fd: RawFd) -> Self {
        Self { fd }
    }
}

impl AsRawFd for OwnedAsyncFd {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl Drop for OwnedAsyncFd {
    fn drop(&mut self) {
        unsafe { libc::close(self.fd) };
    }
}

/// Create an `AsyncFd` that owns a raw file descriptor.
pub fn async_fd_from_raw(fd: RawFd) -> io::Result<AsyncFd<OwnedAsyncFd>> {
    AsyncFd::new(OwnedAsyncFd::from_raw_fd(fd))
}

// ===========================================================================
// Pipe helpers
// ===========================================================================

/// Create a pair of connected async fds (like `pipe(2)`).
/// Returns `(read_fd, write_fd)`.
pub fn pipe() -> io::Result<(AsyncFd<OwnedAsyncFd>, AsyncFd<OwnedAsyncFd>)> {
    let mut fds: [RawFd; 2] = [0; 2];
    // Use pipe2 with O_CLOEXEC so fds are not inherited across exec.
    let res = unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) };
    if res < 0 {
        return Err(io::Error::last_os_error());
    }
    // Set non-blocking — with cleanup on failure.
    if let Err(e) = set_nonblocking(fds[0]) {
        unsafe {
            libc::close(fds[0]);
            libc::close(fds[1]);
        }
        return Err(e);
    }
    if let Err(e) = set_nonblocking(fds[1]) {
        unsafe {
            libc::close(fds[0]);
            libc::close(fds[1]);
        }
        return Err(e);
    }
    let read_fd = AsyncFd::new(OwnedAsyncFd::from_raw_fd(fds[0]))?;
    let write_fd = AsyncFd::new(OwnedAsyncFd::from_raw_fd(fds[1]))?;
    Ok((read_fd, write_fd))
}

fn set_nonblocking(fd: RawFd) -> io::Result<()> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(io::Error::last_os_error());
    }
    if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
