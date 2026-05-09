//! Minimal no_std byte I/O traits.
//!
//! These traits intentionally mirror the small, common subset of `std::io`
//! needed by protocol and serialization crates without depending on a runtime.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    UnexpectedEof,
    WriteZero,
    InvalidData,
    PermissionDenied,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    kind: ErrorKind,
    message: String,
}

impl Error {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn kind(&self) -> ErrorKind {
        self.kind.clone()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl core::error::Error for Error {}

pub type Result<T> = core::result::Result<T, Error>;

pub trait Read {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;

    fn read_exact(&mut self, mut buf: &mut [u8]) -> Result<()> {
        while !buf.is_empty() {
            match self.read(buf)? {
                0 => {
                    return Err(Error::new(
                        ErrorKind::UnexpectedEof,
                        "failed to fill whole buffer",
                    ));
                }
                n => {
                    let tmp = buf;
                    buf = &mut tmp[n..];
                }
            }
        }
        Ok(())
    }

    fn read_to_end(&mut self, out: &mut Vec<u8>) -> Result<usize> {
        let start_len = out.len();
        let mut buf = [0u8; 1024];
        loop {
            let n = self.read(&mut buf)?;
            if n == 0 {
                return Ok(out.len() - start_len);
            }
            out.extend_from_slice(&buf[..n]);
        }
    }

    fn read_to_string(&mut self, out: &mut String) -> Result<usize> {
        let mut bytes = Vec::new();
        let n = self.read_to_end(&mut bytes)?;
        let text = String::from_utf8(bytes).map_err(|_| {
            Error::new(ErrorKind::InvalidData, "stream did not contain valid UTF-8")
        })?;
        out.push_str(&text);
        Ok(n)
    }
}

pub trait Write {
    fn write(&mut self, buf: &[u8]) -> Result<usize>;

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }

    fn write_all(&mut self, mut buf: &[u8]) -> Result<()> {
        while !buf.is_empty() {
            match self.write(buf)? {
                0 => {
                    return Err(Error::new(
                        ErrorKind::WriteZero,
                        "failed to write whole buffer",
                    ));
                }
                n => buf = &buf[n..],
            }
        }
        Ok(())
    }
}

#[cfg(not(feature = "std"))]
impl Read for &[u8] {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let n = buf.len().min(self.len());
        buf[..n].copy_from_slice(&self[..n]);
        *self = &self[n..];
        Ok(n)
    }
}

#[cfg(not(feature = "std"))]
impl Write for Vec<u8> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.extend_from_slice(buf);
        Ok(buf.len())
    }
}

#[cfg(not(feature = "std"))]
impl<W: Write + ?Sized> Write for &mut W {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        (**self).write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        (**self).flush()
    }
}

#[cfg(all(feature = "std", not(target_os = "none")))]
impl<T: std::io::Read + ?Sized> Read for T {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        std::io::Read::read(self, buf).map_err(Error::from)
    }
}

#[cfg(all(feature = "std", not(target_os = "none")))]
impl<T: std::io::Write + ?Sized> Write for T {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        std::io::Write::write(self, buf).map_err(Error::from)
    }

    fn flush(&mut self) -> Result<()> {
        std::io::Write::flush(self).map_err(Error::from)
    }
}

#[cfg(all(feature = "std", not(target_os = "none")))]
impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        let kind = match error.kind() {
            std::io::ErrorKind::UnexpectedEof => ErrorKind::UnexpectedEof,
            std::io::ErrorKind::WriteZero => ErrorKind::WriteZero,
            std::io::ErrorKind::InvalidData => ErrorKind::InvalidData,
            _ => ErrorKind::Other,
        };
        Error::new(kind, error.to_string())
    }
}
