//! TCP socket for bare-metal networking

extern crate alloc;

use super::ip::{
    IpAddr, IpStack, TcpHeader, ETH_TYPE_IPV4, IP_PROTO_TCP, TCP_FLAG_ACK, TCP_FLAG_PSH,
};
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

#[derive(Debug, Clone, Copy)]
pub struct TcpSocket {
    local: SocketAddr,
    remote: SocketAddr,
    state: TcpState,
    backlog: u16,
    nonblocking: bool,
    seq: u32,
    ack: u32,
}

impl TcpSocket {
    pub fn new() -> Self {
        Self {
            local: SocketAddr::default(),
            remote: SocketAddr::default(),
            state: TcpState::Closed,
            backlog: 0,
            nonblocking: false,
            seq: 0,
            ack: 0,
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

    pub fn send(
        &mut self,
        data: &[u8],
        stack: &IpStack,
        dst_ip: IpAddr,
        dst_mac: [u8; 6],
    ) -> Result<usize, TcpError> {
        if self.state != TcpState::Established && self.state != TcpState::SynReceived {
            return Err(TcpError);
        }
        let available = packet_data_capacity();
        let to_send = data.len().min(available);
        let ip_len = (20 + 20 + to_send) as u16;
        let mut packet = [0u8; 1514];
        let eth = stack.eth_header(dst_mac, ETH_TYPE_IPV4);
        eth.to_slice(&mut packet);
        let ip = stack.ip_header(dst_ip, IP_PROTO_TCP, ip_len);
        ip.to_slice(&mut packet[14..]);
        let tcp = TcpHeader {
            src_port: self.local.1,
            dst_port: self.remote.1,
            seq: self.seq,
            ack: self.ack,
            flags: TCP_FLAG_PSH | TCP_FLAG_ACK,
            window: 65535,
            checksum: 0,
            urgent: 0,
        };
        tcp.to_slice(&mut packet[34..]);
        let payload_start = 54;
        if to_send > 0 {
            packet[payload_start..payload_start + to_send].copy_from_slice(&data[..to_send]);
        }
        self.seq = self.seq.wrapping_add(to_send as u32);
        Ok(to_send)
    }

    pub fn recv(&mut self, data: &mut [u8]) -> Result<usize, TcpError> {
        let _ = data;
        Err(TcpError)
    }

    pub fn close(&mut self) {
        self.state = TcpState::Closed;
    }

    pub fn local_addr(&self) -> Option<SocketAddr> {
        Some(self.local)
    }

    pub fn remote_addr(&self) -> Option<SocketAddr> {
        if self.remote.1 != 0 {
            Some(self.remote)
        } else {
            None
        }
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

fn packet_data_capacity() -> usize {
    1514 - 54
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

pub struct TcpListener {
    local: SocketAddr,
    backlog: u16,
    connections: [TcpSocket; 8],
    conn_count: usize,
}

impl TcpListener {
    pub fn new() -> Self {
        Self {
            local: SocketAddr::default(),
            backlog: 0,
            connections: [TcpSocket::new(); 8],
            conn_count: 0,
        }
    }

    pub fn bind(&mut self, addr: SocketAddr) -> Result<(), TcpError> {
        self.local = addr;
        Ok(())
    }

    pub fn listen(&mut self, backlog: u16) -> Result<(), TcpError> {
        self.backlog = backlog;
        Ok(())
    }

    pub fn accept(&mut self) -> Result<TcpSocket, TcpError> {
        if self.conn_count > 0 {
            self.conn_count -= 1;
            let conn = self.connections[self.conn_count];
            self.connections[self.conn_count] = TcpSocket::new();
            Ok(conn)
        } else {
            Err(TcpError)
        }
    }

    pub fn local_addr(&self) -> Option<SocketAddr> {
        Some(self.local)
    }
}
