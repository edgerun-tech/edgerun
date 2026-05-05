use std::collections::HashMap;
use std::io::{self, Write};
use std::mem;
use std::os::fd::RawFd;
use std::sync::{Arc, Mutex};

mod linux_abi {
    use core::ffi::c_void;

    pub const AF_INET: i32 = 2;
    pub const SOCK_STREAM: i32 = 1;
    pub const SOL_SOCKET: i32 = 1;
    pub const SO_REUSEADDR: i32 = 2;
    pub const F_GETFL: i32 = 3;
    pub const F_SETFL: i32 = 4;
    pub const O_NONBLOCK: i32 = 0x800;
    pub const EPOLLIN: u32 = 0x001;
    pub const EPOLLET: u32 = 0x8000_0000;
    pub const EPOLL_CTL_ADD: i32 = 1;
    pub const CLOCK_MONOTONIC: i32 = 1;
    pub const TFD_CLOEXEC: i32 = 0o2000000;
    pub const TFD_NONBLOCK: i32 = 0o0004000;

    pub type Socklen = u32;

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct InAddr {
        pub s_addr: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct Sockaddr {
        pub sa_family: u16,
        pub sa_data: [u8; 14],
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct SockaddrIn {
        pub sin_family: u16,
        pub sin_port: u16,
        pub sin_addr: InAddr,
        pub sin_zero: [u8; 8],
    }

    #[repr(C, packed)]
    #[derive(Clone, Copy)]
    pub struct EpollEvent {
        pub events: u32,
        pub u64: u64,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct Timespec {
        pub tv_sec: i64,
        pub tv_nsec: i64,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct Itimerspec {
        pub it_interval: Timespec,
        pub it_value: Timespec,
    }

    unsafe extern "C" {
        pub fn accept(fd: i32, addr: *mut Sockaddr, addrlen: *mut Socklen) -> i32;
        pub fn bind(fd: i32, addr: *const Sockaddr, len: Socklen) -> i32;
        pub fn close(fd: i32) -> i32;
        pub fn epoll_create1(flags: i32) -> i32;
        pub fn epoll_ctl(epfd: i32, op: i32, fd: i32, event: *mut EpollEvent) -> i32;
        pub fn epoll_wait(epfd: i32, events: *mut EpollEvent, maxevents: i32, timeout: i32)
            -> i32;
        pub fn fcntl(fd: i32, cmd: i32, arg: i32) -> i32;
        pub fn listen(fd: i32, backlog: i32) -> i32;
        pub fn read(fd: i32, buf: *mut c_void, count: usize) -> isize;
        pub fn recv(fd: i32, buf: *mut c_void, len: usize, flags: i32) -> isize;
        pub fn setsockopt(
            fd: i32,
            level: i32,
            optname: i32,
            optval: *const c_void,
            optlen: Socklen,
        ) -> i32;
        pub fn socket(domain: i32, ty: i32, protocol: i32) -> i32;
        pub fn timerfd_create(clockid: i32, flags: i32) -> i32;
        pub fn timerfd_settime(
            fd: i32,
            flags: i32,
            new_value: *const Itimerspec,
            old_value: *mut Itimerspec,
        ) -> i32;
    }
}

use linux_abi::*;

#[derive(Debug)]
pub enum EventSource {
    TcpListener {
        port: u16,
    },
    TcpConnection {
        fd: RawFd,
        sock_id: u32,
    },
    Timer {
        fd: RawFd,
        timer_id: u64,
        interval_ms: u64,
    },
}

pub struct EventLoop {
    epoll_fd: RawFd,
    sources: HashMap<RawFd, EventSource>,
    event_queue: Arc<Mutex<Vec<Vec<u8>>>>,
    next_sock_id: u32,
    next_timer_id: u64,
    verbose: bool,
}

impl EventLoop {
    pub fn new(event_queue: Arc<Mutex<Vec<Vec<u8>>>>, verbose: bool) -> io::Result<Self> {
        let epoll_fd = unsafe { epoll_create1(0) };
        if epoll_fd < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            epoll_fd,
            sources: HashMap::new(),
            event_queue,
            next_sock_id: 1,
            next_timer_id: 1,
            verbose,
        })
    }

    pub fn add_tcp_listener(&mut self, port: u16) -> io::Result<()> {
        let fd = unsafe { socket(AF_INET, SOCK_STREAM, 0) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }

        let opt: i32 = 1;
        unsafe {
            setsockopt(
                fd,
                SOL_SOCKET,
                SO_REUSEADDR,
                &opt as *const _ as *const _,
                4,
            );
        }

        let addr = SockaddrIn {
            sin_family: AF_INET as _,
            sin_port: port.to_be(),
            sin_addr: InAddr {
                s_addr: u32::from_be_bytes([127, 0, 0, 1]).to_be(),
            },
            sin_zero: [0; 8],
        };

        let ret = unsafe {
            bind(
                fd,
                (&addr as *const SockaddrIn).cast(),
                mem::size_of::<SockaddrIn>() as _,
            )
        };
        if ret < 0 {
            unsafe { close(fd) };
            return Err(io::Error::last_os_error());
        }

        if unsafe { listen(fd, 128) } < 0 {
            unsafe { close(fd) };
            return Err(io::Error::last_os_error());
        }

        let flags = unsafe { fcntl(fd, F_GETFL, 0) };
        if flags < 0 {
            unsafe { close(fd) };
            return Err(io::Error::last_os_error());
        }
        if unsafe { fcntl(fd, F_SETFL, flags | O_NONBLOCK) } < 0 {
            unsafe { close(fd) };
            return Err(io::Error::last_os_error());
        }

        let mut ev: EpollEvent = unsafe { mem::zeroed() };
        ev.events = EPOLLIN as _;
        ev.u64 = fd as u64;
        if unsafe { epoll_ctl(self.epoll_fd, EPOLL_CTL_ADD, fd, &mut ev) } < 0 {
            unsafe { close(fd) };
            return Err(io::Error::last_os_error());
        }

        self.sources.insert(fd, EventSource::TcpListener { port });

        if self.verbose {
            eprintln!("EventLoop: listening on 127.0.0.1:{}", port);
        }
        Ok(())
    }

    pub fn add_timer(&mut self, interval_ms: u64) -> io::Result<u64> {
        let fd = unsafe {
            timerfd_create(
                CLOCK_MONOTONIC,
                TFD_CLOEXEC | TFD_NONBLOCK,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }

        let itimerspec = Itimerspec {
            it_interval: Timespec {
                tv_sec: (interval_ms / 1000) as _,
                tv_nsec: ((interval_ms % 1000) * 1_000_000) as _,
            },
            it_value: Timespec {
                tv_sec: (interval_ms / 1000) as _,
                tv_nsec: ((interval_ms % 1000) * 1_000_000) as _,
            },
        };

        if unsafe { timerfd_settime(fd, 0, &itimerspec, std::ptr::null_mut()) } < 0 {
            unsafe { close(fd) };
            return Err(io::Error::last_os_error());
        }

        let mut ev: EpollEvent = unsafe { mem::zeroed() };
        ev.events = EPOLLIN as _;
        ev.u64 = fd as u64;
        if unsafe { epoll_ctl(self.epoll_fd, EPOLL_CTL_ADD, fd, &mut ev) } < 0 {
            unsafe { close(fd) };
            return Err(io::Error::last_os_error());
        }

        let timer_id = self.next_timer_id;
        self.next_timer_id += 1;

        self.sources.insert(
            fd,
            EventSource::Timer {
                fd,
                timer_id,
                interval_ms,
            },
        );

        if self.verbose {
            eprintln!("EventLoop: timer {} every {}ms", timer_id, interval_ms);
        }
        Ok(timer_id)
    }

    pub fn run_once(&mut self, timeout_ms: i32) -> io::Result<usize> {
        let mut events: [EpollEvent; 16] = unsafe { mem::zeroed() };
        let nfds = unsafe { epoll_wait(self.epoll_fd, events.as_mut_ptr(), 16, timeout_ms) };
        if nfds < 0 {
            return Err(io::Error::last_os_error());
        }

        let mut event_count = 0;
        for i in 0..nfds as usize {
            let fd = events[i].u64 as RawFd;
            if let Some(source) = self.sources.get(&fd) {
                match source {
                    EventSource::TcpListener { .. } => loop {
                        let mut addr: SockaddrIn = unsafe { mem::zeroed() };
                        let mut addrlen: Socklen = mem::size_of::<SockaddrIn>() as _;
                        let client_fd = unsafe {
                            accept(fd, (&mut addr as *mut SockaddrIn).cast(), &mut addrlen)
                        };
                        if client_fd < 0 {
                            break;
                        }

                        let sock_id = self.next_sock_id;
                        self.next_sock_id += 1;

                        let flags = unsafe { fcntl(client_fd, F_GETFL, 0) };
                        if flags >= 0 {
                            unsafe {
                                fcntl(client_fd, F_SETFL, flags | O_NONBLOCK)
                            };
                        }

                        self.push_network_connected(sock_id);

                        let mut ev: EpollEvent = unsafe { mem::zeroed() };
                        ev.events = (EPOLLIN | EPOLLET) as _;
                        ev.u64 = client_fd as u64;

                        let ctl_ret = unsafe {
                            epoll_ctl(self.epoll_fd, EPOLL_CTL_ADD, client_fd, &mut ev)
                        };
                        if ctl_ret >= 0 {
                            self.sources.insert(
                                client_fd,
                                EventSource::TcpConnection {
                                    fd: client_fd,
                                    sock_id,
                                },
                            );
                            event_count += 1;
                        } else {
                            unsafe { close(client_fd) };
                        }
                    },
                    EventSource::TcpConnection { fd, sock_id } => {
                        let fd = *fd;
                        let sock_id = *sock_id;
                        let mut buf = [0u8; 4096];
                        let n = unsafe { recv(fd, buf.as_mut_ptr() as *mut _, buf.len(), 0) };
                        if n > 0 {
                            self.push_network_received(sock_id, &buf[..n as usize]);
                            event_count += 1;
                        } else if n == 0 {
                            self.push_network_disconnected(sock_id);
                            unsafe { close(fd) };
                            self.sources.remove(&fd);
                            event_count += 1;
                        } else {
                            let err = io::Error::last_os_error();
                            if err.kind() == io::ErrorKind::WouldBlock {
                                continue;
                            }
                            self.push_network_error(sock_id);
                            unsafe { close(fd) };
                            self.sources.remove(&fd);
                            event_count += 1;
                        }
                    }
                    EventSource::Timer { timer_id, .. } => {
                        let timer_id = *timer_id;
                        let mut buf = [0u8; 8];
                        let _ = unsafe { read(fd, buf.as_mut_ptr() as *mut _, buf.len()) };
                        self.push_timer_fired(timer_id);
                        event_count += 1;
                    }
                }
            }
        }

        Ok(event_count)
    }

    fn push_network_connected(&self, sock_id: u32) {
        let mut data = Vec::with_capacity(5);
        data.extend_from_slice(&sock_id.to_le_bytes());
        data.push(1);
        self.push_event(1, &data);
    }

    fn push_network_disconnected(&self, sock_id: u32) {
        let mut data = Vec::with_capacity(5);
        data.extend_from_slice(&sock_id.to_le_bytes());
        data.push(2);
        self.push_event(1, &data);
    }

    fn push_network_received(&self, sock_id: u32, payload: &[u8]) {
        let mut data = Vec::with_capacity(9 + payload.len());
        data.extend_from_slice(&sock_id.to_le_bytes());
        data.push(3);
        data.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        data.extend_from_slice(payload);
        self.push_event(1, &data);
    }

    fn push_network_error(&self, sock_id: u32) {
        let mut data = Vec::with_capacity(5);
        data.extend_from_slice(&sock_id.to_le_bytes());
        data.push(4);
        self.push_event(1, &data);
    }

    fn push_timer_fired(&self, timer_id: u64) {
        let mut data = Vec::with_capacity(9);
        data.push(1);
        data.extend_from_slice(&timer_id.to_le_bytes());
        self.push_event(3, &data);
    }

    fn push_event(&self, event_type: u8, data: &[u8]) {
        let mut event = Vec::with_capacity(1 + data.len());
        event.push(event_type);
        event.extend_from_slice(data);
        let mut queue = self.event_queue.lock().unwrap();
        queue.push(event);
    }
}

impl Drop for EventLoop {
    fn drop(&mut self) {
        for (&fd, source) in &self.sources {
            match source {
                EventSource::TcpListener { .. } | EventSource::TcpConnection { .. } => {
                    unsafe { close(fd) };
                }
                EventSource::Timer { .. } => {
                    unsafe { close(fd) };
                }
            }
        }
        if self.epoll_fd >= 0 {
            unsafe { close(self.epoll_fd) };
        }
    }
}
