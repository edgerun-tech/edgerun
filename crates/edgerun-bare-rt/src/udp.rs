//! Bare-metal UDP - uses syscalls directly.

#![no_std]

extern crate alloc;

pub const AF_INET: i32 = libc::AF_INET as i32;
pub const SOCK_DGRAM: i32 = libc::SOCK_DGRAM as i32;
pub const O_NONBLOCK: i32 = libc::O_NONBLOCK as i32;

pub struct UdpSocket {
    fd: i32,
}

impl UdpSocket {
    pub fn new() -> Result<Self, Error> {
        let fd = unsafe { libc::socket(AF_INET, SOCK_DGRAM, 0) };
        if fd < 0 {
            return Err(Error::last());
        }
        Ok(Self { fd })
    }

    pub fn set_nonblocking(&self) -> Result<(), Error> {
        let flags = unsafe { libc::fcntl(self.fd, libc::F_GETFL, 0) };
        if flags < 0 {
            return Err(Error::last());
        }
        let res = unsafe { libc::fcntl(self.fd, libc::F_SETFL, flags | O_NONBLOCK) };
        if res < 0 {
            return Err(Error::last());
        }
        Ok(())
    }

    pub fn bind(&self, port: u16) -> Result<(), Error> {
        let mut sin: libc::sockaddr_in = unsafe { core::mem::zeroed() };
        sin.sin_family = libc::AF_INET as u16;
        sin.sin_port = port.to_be();
        sin.sin_addr = libc::in_addr { s_addr: 0 };
        let res = unsafe {
            libc::bind(self.fd, &sin as *const _ as *const _, core::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t)
        };
        if res < 0 {
            Err(Error::last())
        } else {
            Ok(())
        }
    }

    pub fn recv(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr), Error> {
        let mut src: libc::sockaddr_in = unsafe { core::mem::zeroed() };
        let mut len: libc::socklen_t = core::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;
        let n = unsafe {
            libc::recvfrom(
                self.fd,
                buf.as_mut_ptr() as *mut _,
                buf.len(),
                0,
                &mut src as *mut _ as *mut _,
                &mut len,
            )
        };
        if n < 0 {
            return Err(Error::last());
        }
        let addr = SocketAddr::new(u32::from_be(src.sin_addr.s_addr), u16::from_be(src.sin_port));
        Ok((n as usize, addr))
    }

    pub fn send(&self, buf: &[u8], dest: &SocketAddr) -> Result<usize, Error> {
        let mut sin: libc::sockaddr_in = unsafe { core::mem::zeroed() };
        sin.sin_family = libc::AF_INET as u16;
        sin.sin_port = dest.port.to_be();
        sin.sin_addr = libc::in_addr { s_addr: dest.ip.to_be() };
        let n = unsafe {
            libc::sendto(
                self.fd,
                buf.as_ptr(),
                buf.len(),
                0,
                &sin as *const _ as *const _,
                core::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
            )
        };
        if n < 0 {
            Err(Error::last())
        } else {
            Ok(n as usize)
        }
    }

    pub fn close(&self) {
        unsafe { libc::close(self.fd) };
    }

    pub fn as_raw_fd(&self) -> i32 {
        self.fd
    }
}

impl Drop for UdpSocket {
    fn drop(&mut self) {
        self.close();
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SocketAddr {
    pub ip: u32,
    pub port: u16,
}

impl SocketAddr {
    pub fn new(ip: u32, port: u16) -> Self {
        Self { ip, port }
    }

    pub fn from_parts(a: u8, b: u8, c: u8, d: u8, port: u16) -> Self {
        let ip = ((a as u32) << 0) | ((b as u32) << 8) | ((c as u32) << 16) | ((d as u32) << 24);
        Self { ip, port }
    }
}

pub struct Error {
    code: i32,
}

impl Error {
    pub fn last() -> Self {
        Self {
            code: unsafe { libc::errno } as i32,
        }
    }

    pub fn code(&self) -> i32 {
        self.code
    }

    pub fn would_block(&self) -> bool {
        self.code == libc::EAGAIN as i32 || self.code == libc::EWOULDBLOCK as i32
    }
}

pub const EAGAIN: i32 = libc::EAGAIN as i32;