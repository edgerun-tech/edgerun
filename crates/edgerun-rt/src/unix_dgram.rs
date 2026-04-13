//! Async Unix domain datagram sockets — connectionless local IPC.
//!
//! Mirrors `AsyncUdpSocket` but uses `AF_UNIX` addresses (paths).

use std::io;
use std::os::unix::net::SocketAddr;
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::io_traits::{AsyncRead, AsyncWrite};
use crate::runtime::try_current_rt;

/// Async Unix domain datagram socket.
pub struct UnixDatagram {
    fd: RawFd,
}

impl UnixDatagram {
    /// Creates a new Unix datagram socket bound to the given path.
    pub fn bind<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let _ = std::fs::remove_file(path.as_ref());
        let path_bytes = path.as_ref().to_str().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "path contains invalid UTF-8")
        })?;

        let fd = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_DGRAM | libc::SOCK_NONBLOCK, 0) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }

        let addr = make_addr(path_bytes)?;
        let addrlen = std::mem::size_of::<libc::sockaddr_un>() as libc::socklen_t;
        let res = unsafe { libc::bind(fd, &addr as *const _ as *const _, addrlen) };
        if res < 0 {
            let e = io::Error::last_os_error();
            unsafe { libc::close(fd) };
            return Err(e);
        }

        Ok(Self { fd })
    }

    /// Creates an unbound Unix datagram socket.
    pub fn unbound() -> io::Result<Self> {
        let fd = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_DGRAM | libc::SOCK_NONBLOCK, 0) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self { fd })
    }

    /// Connects the socket to the given path.
    pub fn connect<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let path_bytes = path.as_ref().to_str().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "path contains invalid UTF-8")
        })?;
        let addr = make_addr(path_bytes)?;
        let addrlen = std::mem::size_of::<libc::sockaddr_un>() as libc::socklen_t;
        let res = unsafe { libc::connect(self.fd, &addr as *const _ as *const _, addrlen) };
        if res < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    /// Receives a single datagram.
    pub fn poll_recv(&self, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        unsafe {
            let n = libc::recv(self.fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len(), 0);
            if n < 0 {
                let e = io::Error::last_os_error();
                if e.kind() == io::ErrorKind::WouldBlock {
                    if let Some(rt) = try_current_rt() {
                        rt.reactor.wait_read(self.fd, cx.waker().clone());
                    }
                    Poll::Pending
                } else {
                    Poll::Ready(Err(e))
                }
            } else {
                Poll::Ready(Ok(n as usize))
            }
        }
    }

    /// Receives a datagram, returning the sender's address.
    pub fn poll_recv_from(&self, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<(usize, SocketAddr)>> {
        unsafe {
            let mut addr: libc::sockaddr_un = std::mem::zeroed();
            let mut addrlen: libc::socklen_t = std::mem::size_of::<libc::sockaddr_un>() as _;
            let n = libc::recvfrom(
                self.fd,
                buf.as_mut_ptr() as *mut libc::c_void,
                buf.len(),
                0,
                &mut addr as *mut _ as *mut libc::sockaddr,
                &mut addrlen,
            );
            if n < 0 {
                let e = io::Error::last_os_error();
                if e.kind() == io::ErrorKind::WouldBlock {
                    if let Some(rt) = try_current_rt() {
                        rt.reactor.wait_read(self.fd, cx.waker().clone());
                    }
                    Poll::Pending
                } else {
                    Poll::Ready(Err(e))
                }
            } else {
                let path = std::ffi::CStr::from_ptr(addr.sun_path.as_ptr())
                    .to_string_lossy()
                    .into_owned();
                let socket_addr = SocketAddr::from_pathname(&path)
                    .unwrap_or_else(|_| SocketAddr::from_pathname("").unwrap());
                Poll::Ready(Ok((n as usize, socket_addr)))
            }
        }
    }

    /// Sends data to the connected peer.
    pub fn poll_send(&self, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        unsafe {
            let n = libc::send(self.fd, buf.as_ptr() as *const libc::c_void, buf.len(), 0);
            if n < 0 {
                let e = io::Error::last_os_error();
                if e.kind() == io::ErrorKind::WouldBlock {
                    if let Some(rt) = try_current_rt() {
                        rt.reactor.wait_write(self.fd, cx.waker().clone());
                    }
                    Poll::Pending
                } else {
                    Poll::Ready(Err(e))
                }
            } else {
                Poll::Ready(Ok(n as usize))
            }
        }
    }

    /// Sends data to the given path.
    pub fn poll_send_to<P: AsRef<Path>>(&self, cx: &mut Context<'_>, buf: &[u8], path: P) -> Poll<io::Result<usize>> {
        let path_bytes = match path.as_ref().to_str() {
            Some(s) => s,
            None => return Poll::Ready(Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid path"))),
        };
        let addr = match make_addr(path_bytes) {
            Ok(a) => a,
            Err(e) => return Poll::Ready(Err(e)),
        };
        unsafe {
            let n = libc::sendto(
                self.fd,
                buf.as_ptr() as *const libc::c_void,
                buf.len(),
                0,
                &addr as *const _ as *const _,
                std::mem::size_of::<libc::sockaddr_un>() as _,
            );
            if n < 0 {
                let e = io::Error::last_os_error();
                if e.kind() == io::ErrorKind::WouldBlock {
                    if let Some(rt) = try_current_rt() {
                        rt.reactor.wait_write(self.fd, cx.waker().clone());
                    }
                    Poll::Pending
                } else {
                    Poll::Ready(Err(e))
                }
            } else {
                Poll::Ready(Ok(n as usize))
            }
        }
    }

    pub fn as_raw_fd(&self) -> RawFd {
        self.fd
    }

    /// Returns the local socket address.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
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
            SocketAddr::from_pathname(&path)
        }
    }
}

impl Drop for UnixDatagram {
    fn drop(&mut self) {
        unsafe { libc::close(self.fd) };
    }
}

impl Unpin for UnixDatagram {}

impl AsyncRead for UnixDatagram {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        self.poll_recv(cx, buf)
    }
}

impl AsyncWrite for UnixDatagram {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        self.poll_send(cx, buf)
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

fn make_addr(path: &str) -> io::Result<libc::sockaddr_un> {
    let mut addr: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    addr.sun_family = libc::AF_UNIX as _;
    let path_bytes = path.as_bytes();
    if path_bytes.len() >= addr.sun_path.len() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "path too long"));
    }
    for (i, &b) in path_bytes.iter().enumerate() {
        addr.sun_path[i] = b as _;
    }
    addr.sun_path[path_bytes.len()] = 0;
    Ok(addr)
}
