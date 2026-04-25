//! Bare-metal TCP - uses syscalls directly.

#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicI32, Ordering};

pub const AF_INET: i32 = libc::AF_INET as i32;
pub const SOCK_STREAM: i32 = libc::SOCK_STREAM as i32;
pub const O_NONBLOCK: i32 = libc::O_NONBLOCK as i32;
pub const O_RDWR: i32 = libc::O_RDWR as i32;

#[derive(Clone, Debug)]
pub struct TcpSocket {
    fd: i32,
}

impl TcpSocket {
    pub fn new() -> Result<Self, Error> {
        let fd = unsafe { libc::socket(AF_INET, SOCK_STREAM, 0) };
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

    pub fn bind(&self, addr: &SocketAddr) -> Result<(), Error> {
        let mut sin: libc::sockaddr_in = unsafe { core::mem::zeroed() };
        sin.sin_family = libc::AF_INET as u16;
        sin.sin_port = addr.port.to_be();
        sin.sin_addr = libc::in_addr { s_addr: addr.ip };
        let res = unsafe { libc::bind(self.fd, &sin as *const _ as *const _, core::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t) };
        if res < 0 {
            Err(Error::last())
        } else {
            Ok(())
        }
    }

    pub fn listen(&self, backlog: i32) -> Result<(), Error> {
        let res = unsafe { libc::listen(self.fd, backlog) };
        if res < 0 {
            Err(Error::last())
        } else {
            Ok(())
        }
    }

    pub fn accept(&self) -> Result<(i32, SocketAddr), Error> {
        let mut addr: libc::sockaddr_in = unsafe { core::mem::zeroed() };
        let mut len: libc::socklen_t = core::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;
        let fd = unsafe { libc::accept(self.fd, &mut addr as *mut _ as *mut _, &mut len) };
        if fd < 0 {
            return Err(Error::last());
        }
        unsafe {
            libc::fcntl(fd, libc::F_SETFL, O_NONBLOCK);
        }
        let addr = SocketAddr::new(
            u32::from_be(addr.sin_addr.s_addr),
            u16::from_be(addr.sin_port),
        );
        Ok((fd, addr))
    }

    pub fn connect(&self, addr: &SocketAddr) -> Result<(), Error> {
        let mut sin: libc::sockaddr_in = unsafe { core::mem::zeroed() };
        sin.sin_family = libc::AF_INET as u16;
        sin.sin_port = addr.port.to_be();
        sin.sin_addr = libc::in_addr { s_addr: addr.ip.to_be() };
        let res = unsafe { libc::connect(self.fd, &sin as *const _ as *const _, core::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t) };
        if res < 0 {
            let err = Error::last();
            if err.code == EINPROGRESS {
                Ok(())
            } else {
                Err(err)
            }
        } else {
            Ok(())
        }
    }
        Ok(())
    }

    pub fn bind(&self, addr: &SocketAddr) -> Result<(), Error> {
        let addr = addr as *const SocketAddr as *const libc::sockaddr;
        let res = unsafe { libc::bind(self.fd, addr, core::mem::size_of::<SocketAddr>() as libc::socklen_t) };
        if res < 0 {
            Err(Error::last())
        } else {
            Ok(())
        }
    }

    pub fn listen(&self, backlog: i32) -> Result<(), Error> {
        let res = unsafe { libc::listen(self.fd, backlog) };
        if res < 0 {
            Err(Error::last())
        } else {
            Ok(())
        }
    }

    pub fn accept(&self) -> Result<(i32, SocketAddr), Error> {
        let mut addr: libc::sockaddr_in = unsafe { core::mem::zeroed() };
        let mut len: libc::socklen_t = core::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;
        let fd = unsafe { libc::accept(self.fd, &mut addr as *mut _ as *mut _, &mut len) };
        if fd < 0 {
            return Err(Error::last());
        }
        unsafe {
            libc::fcntl(fd, libc::F_SETFL, O_NONBLOCK);
        }
        let addr = SocketAddr::new(addr.sin_addr.s_addr, addr.sin_port);
        Ok((fd, addr))
    }

    pub fn connect(&self, addr: &SocketAddr) -> Result<(), Error> {
        let addr = addr as *const SocketAddr as *const libc::sockaddr;
        let res = unsafe { libc::connect(self.fd, addr, core::mem::size_of::<SocketAddr>() as libc::socklen_t) };
        if res < 0 {
            let err = Error::last();
            if err.code == libc::EINPROGRESS as i32 {
                Ok(())
            } else {
                Err(err)
            }
        } else {
            Ok(())
        }
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize, Error> {
        let res = unsafe { libc::read(self.fd, buf.as_mut_ptr() as *mut _, buf.len()) };
        if res < 0 {
            Err(Error::last())
        } else {
            Ok(res as usize)
        }
    }

    pub fn write(&self, buf: &[u8]) -> Result<usize, Error> {
        let res = unsafe { libc::write(self.fd, buf.as_ptr(), buf.len()) };
        if res < 0 {
            Err(Error::last())
        } else {
            Ok(res as usize)
        }
    }

    pub fn close(&self) {
        unsafe { libc::close(self.fd) };
    }

    pub fn as_raw_fd(&self) -> i32 {
        self.fd
    }
}

impl Drop for TcpSocket {
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
pub const EINPROGRESS: i32 = libc::EINPROGRESS as i32;