//! Async UDP socket.
//!
//! ## Fixes applied:
//! - Reusable `FdReadReady` / `FdWriteReady` futures instead of inline struct defs
//! - Proper IPv6 support in address conversion (already present, kept)

use std::io::{self};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket as StdUdp};
use std::os::unix::io::{AsRawFd, RawFd};
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use crate::reactor_fd_ready::{FdReadReady, FdWriteReady};
use crate::runtime::try_current_rt;

// ===========================================================================
// AsyncUdpSocket
// ===========================================================================

/// Async UDP socket with non-blocking I/O.
pub struct AsyncUdpSocket {
    fd: RawFd,
    refs: Arc<AtomicUsize>,
}

impl AsyncUdpSocket {
    pub fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        let socket = StdUdp::bind(addr)?;
        Self::from_std(socket)
    }

    /// Wrap an existing `std::net::UdpSocket` as an async socket.
    /// The socket is set to non-blocking mode.
    pub fn from_std(socket: StdUdp) -> io::Result<Self> {
        socket.set_nonblocking(true)?;
        let fd = socket.as_raw_fd();
        std::mem::forget(socket);
        Ok(Self {
            fd,
            refs: Arc::new(AtomicUsize::new(1)),
        })
    }

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

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        fd_local_addr(self.fd)
    }

    /// Returns the address the socket is connected to (if connected).
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        fd_peer_addr(self.fd)
    }

    /// Gets the value of the `SO_BROADCAST` option for this socket.
    pub fn broadcast(&self) -> io::Result<bool> {
        let mut opt: libc::c_int = 0;
        let mut optlen: libc::socklen_t = std::mem::size_of::<libc::c_int>() as _;
        let res = unsafe {
            libc::getsockopt(
                self.fd,
                libc::SOL_SOCKET,
                libc::SO_BROADCAST,
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

    /// Sets the value of the `SO_BROADCAST` option for this socket.
    pub fn set_broadcast(&self, broadcast: bool) -> io::Result<()> {
        let opt: libc::c_int = if broadcast { 1 } else { 0 };
        let res = unsafe {
            libc::setsockopt(
                self.fd,
                libc::SOL_SOCKET,
                libc::SO_BROADCAST,
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

    /// Gets the value of the `IP_TTL` option for this socket.
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

    /// Sets the value of the `IP_TTL` option for this socket.
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
                    self.reactor_wait_read(cx.waker().clone());
                    Poll::Pending
                } else {
                    Poll::Ready(Err(e))
                }
            } else if n == 0 {
                Poll::Ready(Ok((
                    0,
                    SocketAddr::V4(std::net::SocketAddrV4::new(
                        std::net::Ipv4Addr::UNSPECIFIED,
                        0,
                    )),
                )))
            } else {
                let storage = storage.assume_init();
                let addr = sockaddr_to_addr(&storage, addrlen);
                Poll::Ready(Ok((n as usize, addr)))
            }
        }
    }

    pub fn poll_send(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        unsafe {
            let n = libc::send(self.fd, buf.as_ptr() as *const libc::c_void, buf.len(), 0);
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

    pub fn poll_recv(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        unsafe {
            let n = libc::recv(self.fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len(), 0);
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

    // Async convenience methods using reusable waiter futures.
    pub async fn send_to(&self, buf: &[u8], target: SocketAddr) -> io::Result<usize> {
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
            let e = io::Error::last_os_error();
            if e.kind() == io::ErrorKind::WouldBlock {
                FdWriteReady::new(self.fd).await;
                continue;
            }
            return Err(e);
        }
    }

    pub async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
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
            let e = io::Error::last_os_error();
            if e.kind() == io::ErrorKind::WouldBlock {
                FdReadReady::new(self.fd).await;
                continue;
            }
            return Err(e);
        }
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
            refs: Arc::clone(&self.refs),
        }
    }
}

impl Unpin for AsyncUdpSocket {}

// ===========================================================================
// Address helpers
// ===========================================================================

fn fd_local_addr(fd: RawFd) -> io::Result<SocketAddr> {
    unsafe {
        let mut storage: std::mem::MaybeUninit<libc::sockaddr_storage> =
            std::mem::MaybeUninit::zeroed();
        let mut len = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
        let res = libc::getsockname(fd, storage.as_mut_ptr() as *mut libc::sockaddr, &mut len);
        if res < 0 {
            return Err(io::Error::last_os_error());
        }
        let storage = storage.assume_init();
        Ok(sockaddr_to_addr(&storage, len))
    }
}

fn fd_peer_addr(fd: RawFd) -> io::Result<SocketAddr> {
    unsafe {
        let mut storage: std::mem::MaybeUninit<libc::sockaddr_storage> =
            std::mem::MaybeUninit::zeroed();
        let mut len = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
        let res = libc::getpeername(fd, storage.as_mut_ptr() as *mut libc::sockaddr, &mut len);
        if res < 0 {
            return Err(io::Error::last_os_error());
        }
        let storage = storage.assume_init();
        Ok(sockaddr_to_addr(&storage, len))
    }
}

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
                std::ptr::write(storage.as_mut_ptr() as *mut libc::sockaddr_in, sin);
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
                std::ptr::write(storage.as_mut_ptr() as *mut libc::sockaddr_in6, sin6);
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

fn sockaddr_to_addr(storage: &libc::sockaddr_storage, len: libc::socklen_t) -> SocketAddr {
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
