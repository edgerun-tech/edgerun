//! Per-client Wayland connection state.

use std::io;
use std::os::fd::RawFd;

use crate::wire;
use crate::wire::decode::parse_message;
use crate::wire::encode::encode;
use crate::wire::fd::{recv_with_fds, send_with_fds};

/// A connected Wayland client.
///
/// Uses a single reusable send buffer with a read cursor to avoid
/// per-message Vec allocations and partial-send copies.
pub struct Client {
    /// Client id (sequential).
    pub id: u32,
    /// Unix socket fd.
    pub fd: RawFd,
    /// Receive buffer (pending data).
    recv_buf: Vec<u8>,
    /// Pending file descriptors from last recv.
    pending_fds: Vec<i32>,
    /// Linear send buffer.
    send_buf: Vec<u8>,
    /// Read cursor within send_buf.
    send_cursor: usize,
    /// Whether the client has been disconnected.
    pub disconnected: bool,
}

impl Client {
    pub fn new(id: u32, fd: RawFd) -> Self {
        // Set the socket to non-blocking
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags >= 0 {
            unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
        }
        Self {
            id,
            fd,
            recv_buf: Vec::with_capacity(4096),
            pending_fds: Vec::new(),
            send_buf: Vec::with_capacity(8192),
            send_cursor: 0,
            disconnected: false,
        }
    }

    /// Receive data from the client.
    pub fn recv(&mut self) -> io::Result<()> {
        let mut buf = [0u8; 4096];
        let (n, fds) = recv_with_fds(self.fd, &mut buf)?;

        if n == 0 {
            self.disconnected = true;
            return Ok(());
        }

        self.recv_buf.extend_from_slice(&buf[..n]);
        if !fds.is_empty() {
            self.pending_fds.extend(fds);
        }

        Ok(())
    }

    /// Parse all complete messages from the receive buffer.
    pub fn drain_messages(&mut self) -> Vec<wire::Message> {
        let mut messages = Vec::new();

        loop {
            let fds = std::mem::take(&mut self.pending_fds);
            match parse_message(&self.recv_buf, fds) {
                Ok(Some((msg, consumed))) => {
                    // Remove consumed bytes
                    self.recv_buf.drain(..consumed);
                    messages.push(msg);
                }
                Ok(None) => break, // need more data
                Err(_) => {
                    // Protocol error — disconnect client
                    self.disconnected = true;
                    break;
                }
            }
        }

        messages
    }

    /// Queue a message for sending.
    /// Messages without FDs are appended to the linear send buffer.
    /// Messages with FDs are sent immediately via send_with_fds.
    pub fn send_message(&mut self, msg: wire::Message) {
        let data = encode(&msg);

        if !msg.fds.is_empty() {
            // Flush any pending data before sending with FDs
            let _ = self.flush();

            // Send with fds immediately
            let fds: Vec<RawFd> = msg.fds.iter().map(|&f| f).collect();
            if let Err(e) = send_with_fds(self.fd, &data, &fds) {
                eprintln!("[edgerun-compositor] send_with_fds FAILED: {}", e);
                self.disconnected = true;
                return;
            }
        } else {
            self.send_buf.extend_from_slice(&data);
        }
    }

    /// Flush the send queue.
    /// Uses the linear buffer + cursor approach — no per-message allocations.
    pub fn flush(&mut self) -> io::Result<()> {
        let pending_len = self.send_buf.len() - self.send_cursor;
        if pending_len == 0 {
            return Ok(());
        }

        let data = &self.send_buf[self.send_cursor..];
        match unsafe {
            libc::send(
                self.fd,
                data.as_ptr() as *const libc::c_void,
                data.len(),
                libc::MSG_NOSIGNAL,
            )
        } {
            n if n >= 0 => {
                let sent = n as usize;
                self.send_cursor += sent;

                // If we've sent everything, compact the buffer
                if self.send_cursor >= self.send_buf.len() {
                    self.send_buf.clear();
                    self.send_cursor = 0;
                }
                // If we've sent more than half the buffer, compact to avoid growth
                else if self.send_cursor > self.send_buf.len() / 2 {
                    self.send_buf.drain(..self.send_cursor);
                    self.send_cursor = 0;
                }
            }
            _ => {
                let err = io::Error::last_os_error();
                if err.kind() == io::ErrorKind::WouldBlock {
                    return Ok(()); // try again later
                }
                self.disconnected = true;
                return Err(err);
            }
        }
        Ok(())
    }

    /// Get the socket fd.
    pub fn fd(&self) -> RawFd {
        self.fd
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        unsafe { libc::close(self.fd) };
        // Close any pending fds
        for &fd in &self.pending_fds {
            unsafe { libc::close(fd) };
        }
    }
}
