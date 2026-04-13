//! TcpSocket — builder-pattern for TCP connections.
//!
//! Provides fine-grained control over TCP socket options before
//! connecting or binding, similar to `tokio::net::TcpSocket`.

use std::io;
use std::net::SocketAddr;
use std::os::unix::io::{AsRawFd, RawFd};
use std::sync::Arc;

use crate::tcp::AsyncTcpStream;
use crate::tcp::AsyncTcpListener;
use crate::runtime::current_rt;

/// A builder for TCP sockets.
///
/// # Example
/// ```ignore
/// let socket = TcpSocket::new_v4()?;
/// socket.set_reuseaddr(true)?;
/// let stream = socket.connect("127.0.0.1:8080".parse()?).await?;
/// ```
pub struct TcpSocket {
    domain: libc::c_int,
    reuse_addr: bool,
    reuse_port: bool,
    ttl: Option<u32>,
    nodelay: bool,
    send_buffer_size: Option<usize>,
    recv_buffer_size: Option<usize>,
}

impl TcpSocket {
    /// Create a new IPv4 TCP socket.
    pub fn new_v4() -> io::Result<Self> {
        Ok(Self {
            domain: libc::AF_INET,
            reuse_addr: true,
            reuse_port: false,
            ttl: None,
            nodelay: false,
            send_buffer_size: None,
            recv_buffer_size: None,
        })
    }

    /// Create a new IPv6 TCP socket.
    pub fn new_v6() -> io::Result<Self> {
        Ok(Self {
            domain: libc::AF_INET6,
            reuse_addr: true,
            reuse_port: false,
            ttl: None,
            nodelay: false,
            send_buffer_size: None,
            recv_buffer_size: None,
        })
    }

    /// Set `SO_REUSEADDR` on the socket.
    pub fn set_reuseaddr(&mut self, value: bool) -> &mut Self {
        self.reuse_addr = value;
        self
    }

    /// Set `SO_REUSEPORT` on the socket.
    pub fn set_reuseport(&mut self, value: bool) -> &mut Self {
        self.reuse_port = value;
        self
    }

    /// Set `IP_TTL` / `IPV6_UNICAST_HOPS` on the socket.
    pub fn set_ttl(&mut self, ttl: u32) -> &mut Self {
        self.ttl = Some(ttl);
        self
    }

    /// Set `TCP_NODELAY` on the socket.
    pub fn set_nodelay(&mut self, nodelay: bool) -> &mut Self {
        self.nodelay = nodelay;
        self
    }

    /// Set `SO_SNDBUF` on the socket.
    pub fn set_send_buffer_size(&mut self, size: usize) -> &mut Self {
        self.send_buffer_size = Some(size);
        self
    }

    /// Set `SO_RCVBUF` on the socket.
    pub fn set_recv_buffer_size(&mut self, size: usize) -> &mut Self {
        self.recv_buffer_size = Some(size);
        self
    }

    /// Build a TCP listener bound to the given address.
    pub fn bind(self, addr: SocketAddr) -> io::Result<AsyncTcpListener> {
        let fd = self.create_socket()?;
        if let Err(e) = self.apply_socket_opts(fd) {
            unsafe { libc::close(fd) };
            return Err(e);
        }

        let sa = socket_addr_to_sockaddr(&addr);
        let addrlen = sockaddr_len(&addr);
        let res = unsafe { libc::bind(fd, &sa as *const _ as *const _, addrlen) };
        if res < 0 {
            let e = io::Error::last_os_error();
            unsafe { libc::close(fd) };
            return Err(e);
        }

        let res = unsafe { libc::listen(fd, 1024) };
        if res < 0 {
            let e = io::Error::last_os_error();
            unsafe { libc::close(fd) };
            return Err(e);
        }

        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 {
            unsafe { libc::close(fd) };
            return Err(io::Error::last_os_error());
        }
        if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
            unsafe { libc::close(fd) };
            return Err(io::Error::last_os_error());
        }

        let rt = current_rt();
        rt.reactor.get_or_register_fd(fd);

        Ok(AsyncTcpListener::from_fd(fd))
    }

    /// Connect to the given address asynchronously.
    ///
    /// Uses non-blocking connect with epoll-driven readiness.
    pub async fn connect(self, addr: SocketAddr) -> io::Result<Arc<AsyncTcpStream>> {
        let fd = self.create_socket()?;
        if let Err(e) = self.apply_socket_opts(fd) {
            unsafe { libc::close(fd) };
            return Err(e);
        }

        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 {
            unsafe { libc::close(fd) };
            return Err(io::Error::last_os_error());
        }
        if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
            unsafe { libc::close(fd) };
            return Err(io::Error::last_os_error());
        }

        let sa = socket_addr_to_sockaddr(&addr);
        let addrlen = sockaddr_len(&addr);
        let res = unsafe { libc::connect(fd, &sa as *const _ as *const _, addrlen) };
        if res < 0 {
            let e = io::Error::last_os_error();
            // EINPROGRESS and EWOULDBLOCK both mean non-blocking connect started.
            if e.kind() != io::ErrorKind::WouldBlock
                && e.raw_os_error() != Some(libc::EINPROGRESS)
            {
                unsafe { libc::close(fd) };
                return Err(e);
            }
            // Connect in progress — wait for completion.
            wait_for_connect(fd).await?;
        }

        let rt = current_rt();
        rt.reactor.get_or_register_fd(fd);

        Ok(Arc::new(AsyncTcpStream::from_fd(fd)))
    }

    fn create_socket(&self) -> io::Result<RawFd> {
        let fd = unsafe {
            libc::socket(self.domain, libc::SOCK_STREAM | libc::SOCK_CLOEXEC, 0)
        };
        if fd < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(fd)
        }
    }

    fn apply_socket_opts(&self, fd: RawFd) -> io::Result<()> {
        if self.reuse_addr {
            let opt: libc::c_int = 1;
            if unsafe { libc::setsockopt(fd, libc::SOL_SOCKET, libc::SO_REUSEADDR, &opt as *const _ as *const _, std::mem::size_of_val(&opt) as _) } < 0 {
                return Err(io::Error::last_os_error());
            }
        }
        if self.reuse_port {
            let opt: libc::c_int = 1;
            if unsafe { libc::setsockopt(fd, libc::SOL_SOCKET, libc::SO_REUSEPORT, &opt as *const _ as *const _, std::mem::size_of_val(&opt) as _) } < 0 {
                return Err(io::Error::last_os_error());
            }
        }
        if let Some(ttl) = self.ttl {
            let opt = ttl as libc::c_int;
            let (level, optname) = if self.domain == libc::AF_INET6 {
                (libc::IPPROTO_IPV6, libc::IPV6_UNICAST_HOPS)
            } else {
                (libc::IPPROTO_IP, libc::IP_TTL)
            };
            if unsafe { libc::setsockopt(fd, level, optname, &opt as *const _ as *const _, std::mem::size_of_val(&opt) as _) } < 0 {
                return Err(io::Error::last_os_error());
            }
        }
        if self.nodelay {
            let opt: libc::c_int = 1;
            if unsafe { libc::setsockopt(fd, libc::IPPROTO_TCP, libc::TCP_NODELAY, &opt as *const _ as *const _, std::mem::size_of_val(&opt) as _) } < 0 {
                return Err(io::Error::last_os_error());
            }
        }
        if let Some(size) = self.send_buffer_size {
            let opt = size as libc::c_int;
            if unsafe { libc::setsockopt(fd, libc::SOL_SOCKET, libc::SO_SNDBUF, &opt as *const _ as *const _, std::mem::size_of_val(&opt) as _) } < 0 {
                return Err(io::Error::last_os_error());
            }
        }
        if let Some(size) = self.recv_buffer_size {
            let opt = size as libc::c_int;
            if unsafe { libc::setsockopt(fd, libc::SOL_SOCKET, libc::SO_RCVBUF, &opt as *const _ as *const _, std::mem::size_of_val(&opt) as _) } < 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }
}

/// Wait for a non-blocking connect to complete.
///
/// Uses reactor-based async I/O: register for write readiness, then on
/// wakeup check `SO_ERROR`. Returns when the connect either succeeds
/// or fails with a concrete error.
async fn wait_for_connect(fd: RawFd) -> io::Result<()> {
    use crate::reactor_fd_ready::FdWriteReady;

    let rt = current_rt();
    rt.reactor.get_or_register_fd(fd);

    loop {
        let mut err: libc::c_int = 0;
        let mut errlen: libc::socklen_t = std::mem::size_of::<libc::c_int>() as _;
        let res = unsafe {
            libc::getsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_ERROR,
                &mut err as *mut _ as *mut _,
                &mut errlen,
            )
        };
        if res < 0 {
            return Err(io::Error::last_os_error());
        }
        if err == 0 {
            // Connect succeeded.
            return Ok(());
        }
        // Connect still in progress or failed — wait for write readiness
        // before checking again. `FdWriteReady` registers with the reactor
        // and returns `()` when the fd is write-ready.
        FdWriteReady::new(fd).await;
    }
}

fn socket_addr_to_sockaddr(addr: &SocketAddr) -> libc::sockaddr_storage {
    let mut storage: libc::sockaddr_storage = unsafe { std::mem::zeroed() };
    match addr {
        SocketAddr::V4(v4) => {
            let sin = unsafe { &mut *(&mut storage as *mut _ as *mut libc::sockaddr_in) };
            sin.sin_family = libc::AF_INET as _;
            sin.sin_port = v4.port().to_be();
            sin.sin_addr.s_addr = u32::from_ne_bytes(v4.ip().octets()).to_be();
        }
        SocketAddr::V6(v6) => {
            let sin6 = unsafe { &mut *(&mut storage as *mut _ as *mut libc::sockaddr_in6) };
            sin6.sin6_family = libc::AF_INET6 as _;
            sin6.sin6_port = v6.port().to_be();
            sin6.sin6_addr.s6_addr = v6.ip().octets();
            sin6.sin6_flowinfo = v6.flowinfo();
            sin6.sin6_scope_id = v6.scope_id();
        }
    }
    storage
}

fn sockaddr_len(addr: &SocketAddr) -> libc::socklen_t {
    match addr {
        SocketAddr::V4(_) => std::mem::size_of::<libc::sockaddr_in>() as _,
        SocketAddr::V6(_) => std::mem::size_of::<libc::sockaddr_in6>() as _,
    }
}
