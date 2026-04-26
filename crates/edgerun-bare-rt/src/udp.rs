//! UDP socket for bare-metal networking



extern crate alloc;



#[derive(Debug, Clone, Copy, Default)]
pub struct SocketAddr(pub u32, pub u16);

impl SocketAddr {
    pub const fn new(ip: u32, port: u16) -> Self {
        Self(ip, port)
    }

    pub fn from_bytes4(ip: [u8; 4], port: u16) -> Self {
        let ip = ((ip[0] as u32) << 24) | ((ip[1] as u32) << 16) | ((ip[2] as u32) << 8) | (ip[3] as u32);
        Self(ip, port)
    }

    pub fn ip_bytes(&self) -> [u8; 4] {
        [(self.0 >> 24) as u8, (self.0 >> 16) as u8, (self.0 >> 8) as u8, self.0 as u8]
    }

    pub fn port(&self) -> u16 {
        self.1
    }
}

pub struct UdpSocket {
    local: SocketAddr,
    remote: SocketAddr,
    bound: bool,
}

impl UdpSocket {
    pub fn new() -> Self {
        Self {
            local: SocketAddr::default(),
            remote: SocketAddr::default(),
            bound: false,
        }
    }

    pub fn bind(&mut self, addr: SocketAddr) -> Result<(), UdpError> {
        self.local = addr;
        self.bound = true;
        Ok(())
    }

    pub fn connect(&mut self, addr: SocketAddr) -> Result<(), UdpError> {
        self.remote = addr;
        Ok(())
    }

    pub fn send_to(&self, buf: &[u8], addr: SocketAddr) -> Result<usize, UdpError> {
        let _ = (buf, addr);
        Ok(buf.len())
    }

    pub fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr), UdpError> {
        let _ = buf;
        Err(UdpError)
    }

    pub fn local_addr(&self) -> Option<SocketAddr> {
        if self.bound { Some(self.local) } else { None }
    }

    pub fn remote_addr(&self) -> Option<SocketAddr> {
        if self.remote.1 != 0 { Some(self.remote) } else { None }
    }

    pub fn set_nonblocking(&self, _nonblocking: bool) {}
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UdpError;

impl core::fmt::Display for UdpError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "UDP error")
    }
}