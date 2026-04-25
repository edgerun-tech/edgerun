//! Unix socketpair stub.


pub fn socketpair() -> Result<(i32, i32), Error> { Err(Error) }

#[derive(Debug)]
pub struct Error;
impl Error { pub fn last() -> Self { Self } }