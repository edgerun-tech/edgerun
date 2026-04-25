//! Unix socket stub.


pub struct UnixSocket;
impl UnixSocket {
    pub fn new() -> Result<Self, Error> { Err(Error) }
    pub fn bind(&self, _: &str) -> Result<(), Error> { Err(Error) }
    pub fn listen(&self, _: i32) -> Result<(), Error> { Err(Error) }
    pub fn accept(&self) -> Result<i32, Error> { Err(Error) }
    pub fn connect(&self, _: &str) -> Result<(), Error> { Err(Error) }
    pub fn read(&self, _: &mut [u8]) -> Result<usize, Error> { Err(Error) }
    pub fn write(&self, _: &[u8]) -> Result<usize, Error> { Err(Error) }
    pub fn close(&self) {}
}

#[derive(Debug)]
pub struct Error;
impl Error { pub fn last() -> Self { Self } }