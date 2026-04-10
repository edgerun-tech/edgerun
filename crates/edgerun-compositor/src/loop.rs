//! Epoll-based event loop for the compositor.

use std::collections::HashMap;
use std::io;
use std::os::fd::RawFd;

/// Event types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventSource {
    /// Wayland server listen socket.
    WaylandListen,
    /// Wayland client socket.
    WaylandClient(u32),
    /// DRM device fd.
    DrmDevice,
    /// evdev input device.
    EvdevDevice(u32),
}

/// The epoll event loop.
pub struct EventLoop {
    epoll_fd: RawFd,
    /// Map from fd to event source.
    fd_to_source: HashMap<RawFd, EventSource>,
    /// Reusable buffer for epoll events — avoids per-wait() allocation.
    events_buf: Vec<libc::epoll_event>,
}

impl EventLoop {
    /// Create a new event loop.
    pub fn new() -> io::Result<Self> {
        let epoll_fd = unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) };
        if epoll_fd < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(Self {
            epoll_fd,
            fd_to_source: HashMap::new(),
            events_buf: vec![libc::epoll_event { events: 0, u64: 0 }; 64],
        })
    }

    /// Register a fd for reading.
    pub fn add_read(&mut self, fd: RawFd, source: EventSource) -> io::Result<()> {
        self.add_read_with_flags(fd, source, (libc::EPOLLIN | libc::EPOLLHUP | libc::EPOLLERR) as u32)
    }

    fn add_read_with_flags(&mut self, fd: RawFd, source: EventSource, events: u32) -> io::Result<()> {
        let mut event = libc::epoll_event {
            events,
            u64: fd as u64,
        };

        let ret = unsafe { libc::epoll_ctl(self.epoll_fd, libc::EPOLL_CTL_ADD, fd, &mut event) };
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        self.fd_to_source.insert(fd, source);
        Ok(())
    }

    /// Register a fd for reading with edge-triggered epoll (only fires on state change).
    /// Useful for DRM fds which are always reported as readable in level-triggered mode.
    pub fn add_read_edge_triggered(&mut self, fd: RawFd, source: EventSource) -> io::Result<()> {
        self.add_read_with_flags(fd, source, (libc::EPOLLIN | libc::EPOLLET | libc::EPOLLHUP | libc::EPOLLERR) as u32)
    }

    /// Register a fd for writing.
    pub fn add_write(&mut self, fd: RawFd, source: EventSource) -> io::Result<()> {
        let mut event = libc::epoll_event {
            events: (libc::EPOLLOUT | libc::EPOLLHUP | libc::EPOLLERR) as u32,
            u64: fd as u64,
        };

        let ret = unsafe { libc::epoll_ctl(self.epoll_fd, libc::EPOLL_CTL_ADD, fd, &mut event) };
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        self.fd_to_source.insert(fd, source);
        Ok(())
    }

    /// Register a fd for both reading and writing.
    pub fn add_rw(&mut self, fd: RawFd, source: EventSource) -> io::Result<()> {
        let mut event = libc::epoll_event {
            events: (libc::EPOLLIN | libc::EPOLLOUT | libc::EPOLLHUP | libc::EPOLLERR) as u32,
            u64: fd as u64,
        };

        let ret = unsafe { libc::epoll_ctl(self.epoll_fd, libc::EPOLL_CTL_ADD, fd, &mut event) };
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        self.fd_to_source.insert(fd, source);
        Ok(())
    }

    /// Remove a fd from epoll.
    pub fn remove(&mut self, fd: RawFd) -> io::Result<()> {
        let ret = unsafe { libc::epoll_ctl(self.epoll_fd, libc::EPOLL_CTL_DEL, fd, std::ptr::null_mut()) };
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }
        self.fd_to_source.remove(&fd);
        Ok(())
    }

    /// Wait for events. Returns a list of (source, events) that are ready.
    /// Blocks for up to `timeout_ms` milliseconds.
    pub fn wait(&mut self, timeout_ms: i32) -> io::Result<Vec<(EventSource, u32)>> {
        let nfds = unsafe {
            libc::epoll_wait(self.epoll_fd, self.events_buf.as_mut_ptr(), self.events_buf.len() as i32, timeout_ms)
        };

        if nfds < 0 {
            let err = io::Error::last_os_error();
            if err.kind() == io::ErrorKind::Interrupted {
                return Ok(Vec::new()); // EINTR
            }
            return Err(err);
        }

        let mut ready = Vec::with_capacity(nfds as usize);
        for i in 0..nfds as usize {
            let fd = self.events_buf[i].u64 as RawFd;
            if let Some(&source) = self.fd_to_source.get(&fd) {
                ready.push((source, self.events_buf[i].events));
            }
        }

        Ok(ready)
    }

    /// Get the epoll fd (for embedding in another loop).
    pub fn epoll_fd(&self) -> RawFd {
        self.epoll_fd
    }
}

impl Drop for EventLoop {
    fn drop(&mut self) {
        if self.epoll_fd >= 0 {
            unsafe { libc::close(self.epoll_fd) };
        }
    }
}
