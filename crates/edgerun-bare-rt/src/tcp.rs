//! TCP socket stub.


pub struct TcpSocket;

impl TcpSocket {
    pub fn new() -> Result<Self, Error> { Err(Error) }
    pub fn bind(&self, _: u16) -> Result<(), Error> { Err(Error) }
    pub fn listen(&self, _: i32) -> Result<(), Error> { Err(Error) }
    pub fn accept(&self) -> Result<(i32, SocketAddr), Error> { Err(Error) }
    pub fn connect(&self, _: &SocketAddr) -> Result<(), Error> { Err(Error) }
    pub fn read(&self, _: &mut [u8]) -> Result<usize, Error> { Err(Error) }
    pub fn write(&self, _: &[u8]) -> Result<usize, Error> { Err(Error) }
    pub fn close(&self) {}
    pub fn as_raw_fd(&self) -> i32 { 0 }
}

#[derive(Clone, Copy, Debug)]
pub struct SocketAddr { pub ip: u32, pub port: u16 }

impl SocketAddr {
    pub fn new(ip: u32, port: u16) -> Self { Self { ip, port } }
}

#[derive(Debug)]
pub struct Error;
impl Error { pub fn last() -> Self { Self } }
pub const EAGAIN: i32 = 11;
pub const EINPROGRESS: i32 = 115;