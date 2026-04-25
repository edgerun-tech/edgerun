//! Process handling stub.

#![no_std]

pub struct Command;

impl Command {
    pub fn new(_prog: &str) -> Self { Self }
    pub fn arg(&mut self, _arg: &str) -> &mut Self { self }
    pub fn spawn(&self) -> Result<Child, Error> { Err(Error) }
}

pub struct Child;

impl Child {
    pub fn kill(&self) -> Result<(), Error> { Err(Error) }
    pub fn wait(&self) -> Result<ExitStatus, Error> { Err(Error) }
}

pub struct ExitStatus;

impl ExitStatus {
    pub fn code(&self) -> Option<i32> { None }
    pub fn success(&self) -> bool { false }
}

#[derive(Debug)]
pub struct Error;

impl Error { pub fn new() -> Self { Self } }