//! Bare-metal Unix domain sockets.

#![no_std]

pub const AF_UNIX: i32 = libc::AF_UNIX as i32;
pub const SOCK_STREAM: i32 = libc::SOCK_STREAM as i32;
pub const SOCK_DGRAM: i32 = libc::SOCK_DGRAM as i32;
pub const O_NONBLOCK: i32 = libc::O_NONBLOCK as i32;

pub struct UnixSocket {
    fd: i32,
}

impl UnixSocket {
    pub fn new() -> Result<Self, Error> {
        let fd = unsafe { libc::socket(AF_UNIX, SOCK_STREAM, 0) };
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

    pub fn bind(&self, path: &str) -> Result<(), Error> {
        let mut addr: libc::sockaddr_un = unsafe { core::mem::zeroed() };
        let path_bytes = path.as_bytes();
        let len = path_bytes.len().min(108);
        unsafe {
            core::ptr::copy_nonoverlapping(
                path_bytes.as_ptr(),
                addr.sun_path.as_mut_ptr() as *mut u8,
                len,
            );
        }
        addr.sun_family = libc::AF_UNIX as u16;
        let res = unsafe {
            libc::bind(self.fd, &addr as *const _, (2 + len) as libc::socklen_t)
        };
        if res < 0 {
            return Err(Error::last());
        }
        Ok(())
    }

    pub fn listen(&self, backlog: i32) -> Result<(), Error> {
        let res = unsafe { libc::listen(self.fd, backlog) };
        if res < 0 { Err(Error::last()) } else { Ok(()) }
    }

    pub fn accept(&self) -> Result<i32, Error> {
        let mut addr: libc::sockaddr_un = unsafe { core::mem::zeroed() };
        let mut len = core::mem::size_of::<libc::sockaddr_un>() as libc::socklen_t;
        let fd = unsafe { libc::accept(self.fd, &mut addr as *mut _, &mut len) };
        if fd < 0 { return Err(Error::last()); }
        unsafe { libc::fcntl(fd, libc::F_SETFL, O_NONBLOCK) };
        Ok(fd)
    }

    pub fn connect(&self, path: &str) -> Result<(), Error> {
        let mut addr: libc::sockaddr_un = unsafe { core::mem::zeroed() };
        let path_bytes = path.as_bytes();
        let len = path_bytes.len().min(108);
        unsafe {
            core::ptr::copy_nonoverlapping(path_bytes.as_ptr(), addr.sun_path.as_mut_ptr() as *mut u8, len);
        }
        let res = unsafe { libc::connect(self.fd, &addr as *const _, (2 + len) as libc::socklen_t) };
        if res < 0 { Err(Error::last()) } else { Ok(()) }
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize, Error> {
        let res = unsafe { libc::read(self.fd, buf.as_mut_ptr() as *mut _, buf.len()) };
        if res < 0 { Err(Error::last()) } else { Ok(res as usize) }
    }

    pub fn write(&self, buf: &[u8]) -> Result<usize, Error> {
        let res = unsafe { libc::write(self.fd, buf.as_ptr(), buf.len()) };
        if res < 0 { Err(Error::last()) } else { Ok(res as usize) }
    }

    pub fn close(&self) { unsafe { libc::close(self.fd) }; }
}

impl Drop for UnixSocket {
    fn drop(&mut self) { self.close(); }
}

pub struct Error { code: i32 }

impl Error {
    pub fn last() -> Self { Self { code: unsafe { libc::errno } as i32 } }
    pub fn code(&self) -> i32 { self.code }
}