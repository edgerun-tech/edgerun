//! Wayland server — Unix socket listener and message dispatcher.

use std::collections::HashMap;
use std::ffi::CString;
use std::io;
use std::os::fd::RawFd;

use crate::client::Client;

/// The Wayland server.
pub struct WaylandServer {
    /// Listening socket fd.
    listen_fd: RawFd,
    /// Connected clients.
    clients: HashMap<u32, Client>,
    /// Next client id.
    next_client_id: u32,
}

impl WaylandServer {
    /// Create a new Wayland server listening on the given socket path.
    pub fn new(socket_path: &str) -> io::Result<Self> {
        let fd = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM | libc::SOCK_CLOEXEC, 0) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }

        // Remove existing socket
        let _ = std::fs::remove_file(socket_path);

        // Bind
        let sockaddr = unix_socket_addr(socket_path);
        let ret = unsafe {
            libc::bind(
                fd,
                sockaddr.as_ptr() as *const libc::sockaddr,
                sockaddr.len() as u32,
            )
        };
        if ret < 0 {
            unsafe { libc::close(fd) };
            return Err(io::Error::last_os_error());
        }

        // Restrict permissions — only owner (and group) can connect.
        // 0o660 prevents other users from connecting to inject input events.
        let c_path = std::ffi::CString::new(socket_path).unwrap();
        unsafe { libc::chmod(c_path.as_ptr(), 0o660) };

        // Listen
        let ret = unsafe { libc::listen(fd, 128) };
        if ret < 0 {
            unsafe { libc::close(fd) };
            return Err(io::Error::last_os_error());
        }

        // Set non-blocking
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags >= 0 {
            unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
        }

        Ok(Self {
            listen_fd: fd,
            clients: HashMap::new(),
            next_client_id: 1,
        })
    }

    /// Accept a new client connection.
    pub fn accept_client(&mut self) -> io::Result<Option<u32>> {
        let client_fd = unsafe {
            libc::accept4(
                self.listen_fd,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                libc::SOCK_CLOEXEC,
            )
        };
        if client_fd < 0 {
            let err = io::Error::last_os_error();
            if err.kind() == io::ErrorKind::WouldBlock {
                return Ok(None);
            }
            return Err(err);
        }

        let id = self.next_client_id;
        self.next_client_id += 1;

        self.clients.insert(id, Client::new(id, client_fd));
        Ok(Some(id))
    }

    /// Get the listen fd for epoll.
    pub fn listen_fd(&self) -> RawFd {
        self.listen_fd
    }

    /// Get a mutable client.
    pub fn client_mut(&mut self, id: u32) -> Option<&mut Client> {
        self.clients.get_mut(&id)
    }

    /// Remove a disconnected client.
    pub fn remove_client(&mut self, id: u32) {
        self.clients.remove(&id);
    }
}

/// Create a Unix socket address for the given path.
fn unix_socket_addr(path: &str) -> Vec<u8> {
    // Build sockaddr_un manually
    let c_path = CString::new(path).unwrap();
    let path_bytes = c_path.as_bytes_with_nul();

    let mut addr = vec![0u8; std::mem::size_of::<libc::sockaddr_un>()];
    // sun_family = AF_UNIX
    let family_bytes = (libc::AF_UNIX as u16).to_le_bytes();
    addr[0] = family_bytes[0];
    addr[1] = family_bytes[1];

    // sun_path
    let max_path = std::mem::size_of::<libc::sockaddr_un>() - 2;
    let copy_len = path_bytes.len().min(max_path);
    addr[2..2 + copy_len].copy_from_slice(&path_bytes[..copy_len]);

    addr
}
