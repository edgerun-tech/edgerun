//! Minimal Linux-style syscall dispatch for bare OCI processes.

use crate::prelude::*;

pub const OCI_LINUX_SYS_WRITE: u64 = 1;
pub const OCI_LINUX_SYS_EXIT: u64 = 60;
pub const OCI_LINUX_SYS_EXIT_GROUP: u64 = 231;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OciSyscallError {
    BadAddress,
    Unsupported(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OciSyscallAction {
    Return(i64),
    Exit(i32),
}

pub trait OciSyscallMemory {
    fn read_bytes(&self, addr: u64, len: usize, out: &mut [u8]) -> Result<(), OciSyscallError>;
}

pub trait OciSyscallSink {
    fn write_fd(&mut self, fd: u64, bytes: &[u8]) -> Result<usize, OciSyscallError>;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OciX86_64SyscallFrame {
    pub rax: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rdx: u64,
    pub r10: u64,
    pub r8: u64,
    pub r9: u64,
}

impl OciX86_64SyscallFrame {
    pub const fn number(&self) -> u64 {
        self.rax
    }

    pub const fn args(&self) -> [u64; 6] {
        [self.rdi, self.rsi, self.rdx, self.r10, self.r8, self.r9]
    }

    pub fn set_return(&mut self, value: i64) {
        self.rax = value as u64;
    }
}

pub fn dispatch_linux_syscall<M: OciSyscallMemory, S: OciSyscallSink>(
    memory: &M,
    sink: &mut S,
    scratch: &mut [u8],
    number: u64,
    args: [u64; 6],
) -> Result<OciSyscallAction, OciSyscallError> {
    match number {
        OCI_LINUX_SYS_WRITE => dispatch_write(memory, sink, scratch, args[0], args[1], args[2]),
        OCI_LINUX_SYS_EXIT | OCI_LINUX_SYS_EXIT_GROUP => Ok(OciSyscallAction::Exit(args[0] as i32)),
        number => Err(OciSyscallError::Unsupported(number)),
    }
}

pub fn dispatch_x86_64_linux_syscall_frame<M: OciSyscallMemory, S: OciSyscallSink>(
    memory: &M,
    sink: &mut S,
    scratch: &mut [u8],
    frame: &mut OciX86_64SyscallFrame,
) -> Result<OciSyscallAction, OciSyscallError> {
    let action = dispatch_linux_syscall(memory, sink, scratch, frame.number(), frame.args())?;
    if let OciSyscallAction::Return(value) = action {
        frame.set_return(value);
    }
    Ok(action)
}

fn dispatch_write<M: OciSyscallMemory, S: OciSyscallSink>(
    memory: &M,
    sink: &mut S,
    scratch: &mut [u8],
    fd: u64,
    mut addr: u64,
    mut len: u64,
) -> Result<OciSyscallAction, OciSyscallError> {
    let mut written = 0usize;
    while len != 0 {
        let chunk_len = core::cmp::min(scratch.len() as u64, len) as usize;
        if chunk_len == 0 {
            break;
        }
        memory.read_bytes(addr, chunk_len, &mut scratch[..chunk_len])?;
        let n = sink.write_fd(fd, &scratch[..chunk_len])?;
        written = written.saturating_add(n);
        if n < chunk_len {
            break;
        }
        addr = addr
            .checked_add(chunk_len as u64)
            .ok_or(OciSyscallError::BadAddress)?;
        len -= chunk_len as u64;
    }
    Ok(OciSyscallAction::Return(written as i64))
}

#[derive(Debug, Default)]
pub struct OciBufferSyscallSink {
    pub writes: Vec<(u64, Vec<u8>)>,
}

impl OciSyscallSink for OciBufferSyscallSink {
    fn write_fd(&mut self, fd: u64, bytes: &[u8]) -> Result<usize, OciSyscallError> {
        self.writes.push((fd, bytes.to_vec()));
        Ok(bytes.len())
    }
}

pub struct OciSliceSyscallMemory<'a> {
    pub base: u64,
    pub bytes: &'a [u8],
}

impl OciSyscallMemory for OciSliceSyscallMemory<'_> {
    fn read_bytes(&self, addr: u64, len: usize, out: &mut [u8]) -> Result<(), OciSyscallError> {
        let offset = addr
            .checked_sub(self.base)
            .ok_or(OciSyscallError::BadAddress)?;
        let offset = usize::try_from(offset).map_err(|_| OciSyscallError::BadAddress)?;
        let end = offset.checked_add(len).ok_or(OciSyscallError::BadAddress)?;
        let Some(input) = self.bytes.get(offset..end) else {
            return Err(OciSyscallError::BadAddress);
        };
        let Some(output) = out.get_mut(..len) else {
            return Err(OciSyscallError::BadAddress);
        };
        output.copy_from_slice(input);
        Ok(())
    }
}
