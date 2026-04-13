//! Async UDP socket using the edgerun-rt epoll reactor.
//!
//! Provides non-blocking `send_to` and `recv_from` futures that
//! integrate with the reactor's waker system — no busy-waiting,
//! no threads blocked on I/O.

use std::io::{self};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket as StdUdp};
use std::os::unix::io::{AsRawFd, RawFd};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use crate::sync::Mutex;
use std::task::{Context, Poll, Waker};

// ===========================================================================
// AsyncUdpSocket
// ===========================================================================

/// Async UDP socket with non-blocking I/O.
///
/// Uses `Arc` internally so the socket can be shared across tasks.
/// Each poll registers with the reactor for read or write readiness.
pub struct AsyncUdpSocket {
    fd: RawFd,
    read_waker: Mutex<Option<Waker>>,
    write_waker: Mutex<Option<Waker>>,
    refs: Arc<AtomicUsize>,
}

impl AsyncUdpSocket {
    /// Bind to the given address and create a non-blocking UDP socket.
    pub fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        let socket = StdUdp::bind(addr)?;
        socket.set_nonblocking(true)?;
        let fd = socket.as_raw_fd();
        std::mem::forget(socket);
        Ok(Self {
            fd,
            read_waker: Mutex::new(None),
            write_waker: Mutex::new(None),
            refs: Arc::new(AtomicUsize::new(1)),
        })
    }

    /// Connect the socket to a remote address (filters incoming packets).
    pub fn connect<A: ToSocketAddrs>(&self, addr: A) -> io::Result<()> {
        let addr = addr
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no addresses"))?;
        unsafe {
            let res = libc::connect(
                self.fd,
                &socket_addr_to_sockaddr(&addr) as *const _ as *const libc::sockaddr,
                sockaddr_len(&addr),
            );
            if res < 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }

    /// Local address of this socket.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        Self::fd_local_addr(self.fd)
    }

    /// Send data to the given address.
    ///
    /// Returns `Poll::Ready(Ok(n))` with bytes sent, or
    /// `Poll::Pending` if the socket would block.
    pub fn poll_send_to(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
        target: SocketAddr,
    ) -> Poll<io::Result<usize>> {
        unsafe {
            let addr = socket_addr_to_sockaddr(&target);
            let addrlen = sockaddr_len(&target);
            let n = libc::sendto(
                self.fd,
                buf.as_ptr() as *const libc::c_void,
                buf.len(),
                0,
                &addr as *const _ as *const libc::sockaddr,
                addrlen,
            );
            if n < 0 {
                let e = io::Error::last_os_error();
                if e.kind() == io::ErrorKind::WouldBlock {
                    *self.write_waker.lock() = Some(cx.waker().clone());
                    crate::register_fd_write(self.fd, cx.waker().clone());
                    Poll::Pending
                } else {
                    Poll::Ready(Err(e))
                }
            } else {
                Poll::Ready(Ok(n as usize))
            }
        }
    }

    /// Receive data and get the sender's address.
    ///
    /// Returns `Poll::Ready(Ok((n, addr)))` with bytes read and source, or
    /// `Poll::Pending` if no data is available.
    pub fn poll_recv_from(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<(usize, SocketAddr)>> {
        unsafe {
            let mut storage: std::mem::MaybeUninit<libc::sockaddr_storage> =
                std::mem::MaybeUninit::zeroed();
            let mut addrlen = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
            let n = libc::recvfrom(
                self.fd,
                buf.as_mut_ptr() as *mut libc::c_void,
                buf.len(),
                0,
                storage.as_mut_ptr() as *mut libc::sockaddr,
                &mut addrlen,
            );
            if n < 0 {
                let e = io::Error::last_os_error();
                if e.kind() == io::ErrorKind::WouldBlock {
                    *self.read_waker.lock() = Some(cx.waker().clone());
                    crate::register_fd_read(self.fd, cx.waker().clone());
                    Poll::Pending
                } else {
                    Poll::Ready(Err(e))
                }
            } else if n == 0 {
                Poll::Ready(Ok((0, SocketAddr::V4(std::net::SocketAddrV4::new(
                    std::net::Ipv4Addr::UNSPECIFIED,
                    0,
                )))))
            } else {
                let storage = storage.assume_init();
                let addr = sockaddr_to_addr(&storage, addrlen);
                Poll::Ready(Ok((n as usize, addr)))
            }
        }
    }

    /// Send data on a connected socket.
    pub fn poll_send(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        unsafe {
            let n = libc::send(
                self.fd,
                buf.as_ptr() as *const libc::c_void,
                buf.len(),
                0,
            );
            if n < 0 {
                let e = io::Error::last_os_error();
                if e.kind() == io::ErrorKind::WouldBlock {
                    *self.write_waker.lock() = Some(cx.waker().clone());
                    crate::register_fd_write(self.fd, cx.waker().clone());
                    Poll::Pending
                } else {
                    Poll::Ready(Err(e))
                }
            } else {
                Poll::Ready(Ok(n as usize))
            }
        }
    }

    /// Receive data on a connected socket.
    pub fn poll_recv(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        unsafe {
            let n = libc::recv(
                self.fd,
                buf.as_mut_ptr() as *mut libc::c_void,
                buf.len(),
                0,
            );
            if n < 0 {
                let e = io::Error::last_os_error();
                if e.kind() == io::ErrorKind::WouldBlock {
                    *self.read_waker.lock() = Some(cx.waker().clone());
                    crate::register_fd_read(self.fd, cx.waker().clone());
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

    // --- helper: local_addr from raw fd ---
    fn fd_local_addr(fd: RawFd) -> io::Result<SocketAddr> {
        unsafe {
            let mut storage: std::mem::MaybeUninit<libc::sockaddr_storage> =
                std::mem::MaybeUninit::zeroed();
            let mut len = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
            let res = libc::getsockname(
                fd,
                storage.as_mut_ptr() as *mut libc::sockaddr,
                &mut len,
            );
            if res < 0 {
                return Err(io::Error::last_os_error());
            }
            let storage = storage.assume_init();
            Ok(sockaddr_to_addr(&storage, len))
        }
    }
}

impl Drop for AsyncUdpSocket {
    fn drop(&mut self) {
        if self.refs.fetch_sub(1, Ordering::AcqRel) == 1 {
            unsafe { libc::close(self.fd) };
        }
    }
}

impl Clone for AsyncUdpSocket {
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

impl Unpin for AsyncUdpSocket {}

// ===========================================================================
// Address conversion helpers
// ===========================================================================

fn socket_addr_to_sockaddr(addr: &SocketAddr) -> libc::sockaddr_storage {
    let mut storage: std::mem::MaybeUninit<libc::sockaddr_storage> =
        std::mem::MaybeUninit::zeroed();
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
            unsafe {
                std::ptr::write(
                    storage.as_mut_ptr() as *mut libc::sockaddr_in,
                    sin,
                );
            }
        }
        SocketAddr::V6(v6) => {
            let sin6 = libc::sockaddr_in6 {
                sin6_family: libc::AF_INET6 as u16,
                sin6_port: v6.port().to_be(),
                sin6_flowinfo: 0,
                sin6_addr: libc::in6_addr {
                    s6_addr: v6.ip().octets(),
                },
                sin6_scope_id: v6.scope_id(),
            };
            unsafe {
                std::ptr::write(
                    storage.as_mut_ptr() as *mut libc::sockaddr_in6,
                    sin6,
                );
            }
        }
    }
    unsafe { storage.assume_init() }
}

fn sockaddr_len(addr: &SocketAddr) -> libc::socklen_t {
    match addr {
        SocketAddr::V4(_) => std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
        SocketAddr::V6(_) => std::mem::size_of::<libc::sockaddr_in6>() as libc::socklen_t,
    }
}

fn sockaddr_to_addr(
    storage: &libc::sockaddr_storage,
    _len: libc::socklen_t,
) -> SocketAddr {
    unsafe {
        let family = storage.ss_family;
        if family as i32 == libc::AF_INET {
            let sin: libc::sockaddr_in =
                std::ptr::read(storage as *const _ as *const libc::sockaddr_in);
            SocketAddr::V4(std::net::SocketAddrV4::new(
                std::net::Ipv4Addr::from(sin.sin_addr.s_addr.to_ne_bytes()),
                u16::from_be(sin.sin_port),
            ))
        } else if family as i32 == libc::AF_INET6 {
            let sin6: libc::sockaddr_in6 =
                std::ptr::read(storage as *const _ as *const libc::sockaddr_in6);
            SocketAddr::V6(std::net::SocketAddrV6::new(
                std::net::Ipv6Addr::from(sin6.sin6_addr.s6_addr),
                u16::from_be(sin6.sin6_port),
                sin6.sin6_flowinfo,
                sin6.sin6_scope_id,
            ))
        } else {
            SocketAddr::V4(std::net::SocketAddrV4::new(
                std::net::Ipv4Addr::UNSPECIFIED,
                0,
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// Async convenience methods (syscall + reactor registration)
// ---------------------------------------------------------------------------

impl AsyncUdpSocket {
    /// Async send-to. Registers with the epoll reactor on WouldBlock.
    pub async fn send_to(&self, buf: &[u8], target: SocketAddr) -> std::io::Result<usize> {
        struct FdWaiter { fd: libc::c_int }
        impl std::future::Future for FdWaiter {
            type Output = ();
            fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<()> {
                crate::register_fd_write(self.fd, cx.waker().clone());
                std::task::Poll::Pending
            }
        }

        loop {
            let addr = socket_addr_to_sockaddr(&target);
            let addrlen = sockaddr_len(&target);
            let n = unsafe {
                libc::sendto(
                    self.fd,
                    buf.as_ptr() as *const libc::c_void,
                    buf.len(),
                    0,
                    &addr as *const _ as *const libc::sockaddr,
                    addrlen,
                )
            };
            if n >= 0 {
                return Ok(n as usize);
            }
            let e = std::io::Error::last_os_error();
            if e.kind() == std::io::ErrorKind::WouldBlock {
                FdWaiter { fd: self.fd }.await;
                continue;
            }
            return Err(e);
        }
    }

    /// Async recv-from. Registers with the epoll reactor on WouldBlock.
    pub async fn recv_from(&self, buf: &mut [u8]) -> std::io::Result<(usize, SocketAddr)> {
        struct FdWaiter { fd: libc::c_int }
        impl std::future::Future for FdWaiter {
            type Output = ();
            fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<()> {
                crate::register_fd_read(self.fd, cx.waker().clone());
                std::task::Poll::Pending
            }
        }

        loop {
            let mut storage: std::mem::MaybeUninit<libc::sockaddr_storage> =
                std::mem::MaybeUninit::zeroed();
            let mut addrlen = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
            let n = unsafe {
                libc::recvfrom(
                    self.fd,
                    buf.as_mut_ptr() as *mut libc::c_void,
                    buf.len(),
                    0,
                    storage.as_mut_ptr() as *mut libc::sockaddr,
                    &mut addrlen,
                )
            };
            if n >= 0 {
                let storage = unsafe { storage.assume_init() };
                let addr = sockaddr_to_addr(&storage, addrlen);
                return Ok((n as usize, addr));
            }
            let e = std::io::Error::last_os_error();
            if e.kind() == std::io::ErrorKind::WouldBlock {
                FdWaiter { fd: self.fd }.await;
                continue;
            }
            return Err(e);
        }
    }
}
