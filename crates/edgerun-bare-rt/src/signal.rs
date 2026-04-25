//! Signal handling stub.

#![no_std]

#[derive(Debug, Clone, Copy)]
pub enum SignalKind {
    Interrupt,
    Termination,
    Child,
}

impl SignalKind {
    pub fn from_raw(_sig: i32) -> Option<Self> { None }
    pub fn to_raw(&self) -> i32 {
        match self {
            SignalKind::Interrupt => 2,
            SignalKind::Termination => 15,
            SignalKind::Child => 17,
        }
    }
}

pub fn signal(_sig: SignalKind, _handler: fn(SignalKind)) -> Result<(), Error> {
    Err(Error)
}

pub fn ignore(_sig: SignalKind) -> Result<(), Error> { Ok(()) }
pub fn default(_sig: SignalKind) -> Result<(), Error> { Ok(()) }

#[derive(Debug)]
pub struct Error;

impl Error { pub fn new() -> Self { Self } }