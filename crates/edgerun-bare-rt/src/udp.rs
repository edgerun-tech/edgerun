//! UDP socket stub.

#![no_std]

pub struct UdpSocket;

impl UdpSocket {
    pub fn new() -> Result<Self, Error> { Err(Error) }
    pub fn bind(&self, _: u16) -> Result<(), Error> { Err(Error) }
    pub fn recv(&self, _: &mut [u8]) -> Result<(usize, SocketAddr), Error> { Err(Error) }
    pub fn send(&self, _: &[u8], _: &SocketAddr) -> Result<usize, Error> { Err(Error) }
    pub fn close(&self) {}
}

#[derive(Clone, Copy, Debug)]
pub struct SocketAddr { pub ip: u32, pub port: u16 }

impl SocketAddr {
    pub fn new(ip: u32, port: u16) -> Self { Self { ip, port } }
}

#[derive(Debug)]
pub struct Error;

impl Error {
    pub fn last() -> Self { Self }
}

pub const EAGAIN: i32 = 11;