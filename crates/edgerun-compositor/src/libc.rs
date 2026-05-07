//! Minimal Linux C ABI used by the compositor.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

pub use std::ffi::{c_char, c_int, c_uint, c_ulong, c_void};

pub type c_ushort = u16;
pub type off_t = i64;
pub type size_t = usize;
pub type ssize_t = isize;
pub type socklen_t = u32;
pub type sa_family_t = u16;
pub type Ioctl = c_ulong;
pub type sighandler_t = usize;

pub const AF_UNIX: c_int = 1;
pub const SOCK_STREAM: c_int = 1;
pub const SOCK_CLOEXEC: c_int = 0o2000000;

pub const SOL_SOCKET: c_int = 1;
pub const SCM_RIGHTS: c_int = 1;

pub const O_RDWR: c_int = 0o2;
pub const O_CREAT: c_int = 0o100;
pub const O_TRUNC: c_int = 0o1000;
pub const O_NONBLOCK: c_int = 0o4000;
pub const O_CLOEXEC: c_int = 0o2000000;

pub const F_GETFL: c_int = 3;
pub const F_SETFL: c_int = 4;

pub const PROT_READ: c_int = 0x1;
pub const PROT_WRITE: c_int = 0x2;
pub const MAP_SHARED: c_int = 0x01;
pub const MAP_PRIVATE: c_int = 0x02;
pub const MAP_FAILED: *mut c_void = !0usize as *mut c_void;

pub const MFD_CLOEXEC: c_uint = 0x0001;
pub const SYS_memfd_create: c_long = 319;

pub const MSG_NOSIGNAL: c_int = 0x4000;
pub const MSG_CMSG_CLOEXEC: c_int = 0x40000000;
pub const MSG_DONTWAIT: c_int = 0x40;
pub const MSG_TRUNC: c_int = 0x20;

pub const EAGAIN: c_int = 11;
pub const EWOULDBLOCK: c_int = EAGAIN;
pub const EINTR: c_int = 4;

pub const EPOLLIN: c_int = 0x001;
pub const EPOLLOUT: c_int = 0x004;
pub const EPOLLERR: c_int = 0x008;
pub const EPOLLHUP: c_int = 0x010;
pub const EPOLLET: c_int = 1 << 31;
pub const EPOLL_CLOEXEC: c_int = O_CLOEXEC;
pub const EPOLL_CTL_ADD: c_int = 1;
pub const EPOLL_CTL_DEL: c_int = 2;

pub const RTLD_LAZY: c_int = 0x00001;
pub const RTLD_GLOBAL: c_int = 0x00100;

pub const SIGUSR1: c_int = 10;
pub const SIGUSR2: c_int = 12;
pub const SIG_ERR: sighandler_t = usize::MAX;

pub type c_long = i64;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct iovec {
    pub iov_base: *mut c_void,
    pub iov_len: size_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct msghdr {
    pub msg_name: *mut c_void,
    pub msg_namelen: socklen_t,
    pub msg_iov: *mut iovec,
    pub msg_iovlen: size_t,
    pub msg_control: *mut c_void,
    pub msg_controllen: size_t,
    pub msg_flags: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct cmsghdr {
    pub cmsg_len: size_t,
    pub cmsg_level: c_int,
    pub cmsg_type: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct sockaddr {
    pub sa_family: sa_family_t,
    pub sa_data: [c_char; 14],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct sockaddr_un {
    pub sun_family: sa_family_t,
    pub sun_path: [c_char; 108],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct epoll_event {
    pub events: u32,
    pub u64: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct stat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_nlink: u64,
    pub st_mode: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub __pad0: c_int,
    pub st_rdev: u64,
    pub st_size: off_t,
    pub st_blksize: i64,
    pub st_blocks: i64,
    pub st_atime: i64,
    pub st_atime_nsec: i64,
    pub st_mtime: i64,
    pub st_mtime_nsec: i64,
    pub st_ctime: i64,
    pub st_ctime_nsec: i64,
    pub __unused: [i64; 3],
}

pub unsafe fn CMSG_LEN(len: c_uint) -> c_uint {
    (std::mem::size_of::<cmsghdr>() + len as usize) as c_uint
}

pub unsafe fn CMSG_DATA(cmsg: *const cmsghdr) -> *mut u8 {
    (cmsg as *mut u8).add(align(std::mem::size_of::<cmsghdr>()))
}

pub unsafe fn CMSG_FIRSTHDR(msg: *const msghdr) -> *mut cmsghdr {
    if (*msg).msg_controllen < std::mem::size_of::<cmsghdr>() {
        std::ptr::null_mut()
    } else {
        (*msg).msg_control.cast::<cmsghdr>()
    }
}

pub unsafe fn CMSG_NXTHDR(msg: *const msghdr, cmsg: *const cmsghdr) -> *mut cmsghdr {
    let next = (cmsg as usize)
        .saturating_add(align((*cmsg).cmsg_len))
        .saturating_sub((*msg).msg_control as usize);
    if next + std::mem::size_of::<cmsghdr>() > (*msg).msg_controllen {
        std::ptr::null_mut()
    } else {
        ((*msg).msg_control as *mut u8).add(next).cast::<cmsghdr>()
    }
}

const fn align(len: usize) -> usize {
    let align = std::mem::size_of::<usize>();
    (len + align - 1) & !(align - 1)
}

unsafe extern "C" {
    pub fn accept4(fd: c_int, addr: *mut sockaddr, len: *mut socklen_t, flags: c_int) -> c_int;
    pub fn bind(fd: c_int, addr: *const sockaddr, len: socklen_t) -> c_int;
    pub fn chmod(path: *const c_char, mode: u32) -> c_int;
    pub fn close(fd: c_int) -> c_int;
    pub fn connect(fd: c_int, addr: *const sockaddr, len: socklen_t) -> c_int;
    pub fn dlclose(handle: *mut c_void) -> c_int;
    pub fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    pub fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    pub fn dup(fd: c_int) -> c_int;
    pub fn epoll_create1(flags: c_int) -> c_int;
    pub fn epoll_ctl(epfd: c_int, op: c_int, fd: c_int, event: *mut epoll_event) -> c_int;
    pub fn epoll_wait(
        epfd: c_int,
        events: *mut epoll_event,
        maxevents: c_int,
        timeout: c_int,
    ) -> c_int;
    pub fn fcntl(fd: c_int, cmd: c_int, ...) -> c_int;
    pub fn fstat(fd: c_int, statbuf: *mut stat) -> c_int;
    pub fn ftruncate(fd: c_int, length: off_t) -> c_int;
    pub fn getuid() -> u32;
    pub fn ioctl(fd: c_int, request: Ioctl, ...) -> c_int;
    pub fn listen(fd: c_int, backlog: c_int) -> c_int;
    pub fn memfd_create(name: *const c_char, flags: c_uint) -> c_int;
    pub fn mmap(
        addr: *mut c_void,
        len: size_t,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        offset: off_t,
    ) -> *mut c_void;
    pub fn munmap(addr: *mut c_void, len: size_t) -> c_int;
    pub fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    pub fn pwrite(fd: c_int, buf: *const c_void, count: size_t, offset: off_t) -> ssize_t;
    pub fn read(fd: c_int, buf: *mut c_void, count: size_t) -> ssize_t;
    pub fn recvmsg(fd: c_int, msg: *mut msghdr, flags: c_int) -> ssize_t;
    pub fn send(fd: c_int, buf: *const c_void, len: size_t, flags: c_int) -> ssize_t;
    pub fn sendmsg(fd: c_int, msg: *const msghdr, flags: c_int) -> ssize_t;
    pub fn signal(signum: c_int, handler: sighandler_t) -> sighandler_t;
    pub fn socket(domain: c_int, ty: c_int, protocol: c_int) -> c_int;
    pub fn syscall(num: c_long, ...) -> c_long;
    pub fn unlink(path: *const c_char) -> c_int;
    pub fn write(fd: c_int, buf: *const c_void, count: size_t) -> ssize_t;
}
