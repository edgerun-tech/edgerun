//! TCP socket for bare-metal networking

#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use super::udp::SocketAddr;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum TcpState {
    #[default]
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    Closing,
    LastAck,
    TimeWait,
}

pub struct TcpSocket {
    local: SocketAddr,
    remote: SocketAddr,
    state: TcpState,
    backlog: u16,
    nonblocking: bool,
}

impl TcpSocket {
    pub fn new() -> Self {
        Self {
            local: SocketAddr::default(),
            remote: SocketAddr::default(),
            state: TcpState::Closed,
            backlog: 0,
            nonblocking: false,
        }
    }

    pub fn bind(&mut self, addr: SocketAddr) -> Result<(), TcpError> {
        self.local = addr;
        Ok(())
    }

    pub fn listen(&mut self, backlog: u16) -> Result<(), TcpError> {
        self.state = TcpState::Listen;
        self.backlog = backlog;
        Ok(())
    }

    pub fn accept(&self) -> Result<TcpSocket, TcpError> {
        let _ = self;
        Err(TcpError)
    }

    pub fn connect(&mut self, addr: SocketAddr) -> Result<(), TcpError> {
        self.remote = addr;
        self.state = TcpState::SynSent;
        Ok(())
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize, TcpError> {
        let _ = buf;
        Ok(buf.len())
    }

    pub fn recv(&self, buf: &mut [u8]) -> Result<usize, TcpError> {
        let _ = buf;
        Err(TcpError)
    }

    pub fn close(&mut self) {
        self.state = TcpState::Closed;
    }

    pub fn local_addr(&self) -> Option<SocketAddr> {
        Some(self.local)
    }

    pub fn remote_addr(&self) -> Option<SocketAddr> {
        if self.remote.1 != 0 { Some(self.remote) } else { None }
    }

    pub fn state(&self) -> TcpState {
        self.state
    }

    pub fn set_nonblocking(&mut self, nonblocking: bool) {
        self.nonblocking = nonblocking;
    }

    pub fn is_nonblocking(&self) -> bool {
        self.nonblocking
    }
}

impl Default for TcpSocket {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TcpError;

impl core::fmt::Display for TcpError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "TCP error")
    }
}