//! IMAP parser compatibility re-exports plus runtime reader adapter.

use crate::prelude::*;
use std::io;

use crate::rt::AsyncReadExt;

pub use edgerun_protocols::imap::parser::*;

/// Reads IMAP commands line-by-line from an async reader.
///
/// The parser and format helpers live in `edgerun-protocols::imap`; this type
/// is runtime-owned because it depends on async I/O.
pub struct ImapReader<R> {
    reader: R,
    line_buf: String,
}

impl<R: crate::rt::AsyncRead + Unpin> ImapReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            line_buf: String::with_capacity(1024),
        }
    }

    pub async fn read_line(&mut self) -> io::Result<Option<String>> {
        self.line_buf.clear();
        loop {
            let mut buf = [0u8; 1];
            let n = match self.reader.read(&mut buf).await {
                Ok(0) => {
                    if self.line_buf.is_empty() {
                        return Ok(None);
                    }
                    return Ok(Some(core::mem::take(&mut self.line_buf)));
                }
                Ok(n) => n,
                Err(e) => return Err(e),
            };
            if n == 0 {
                continue;
            }
            if buf[0] == b'\n' {
                if self.line_buf.ends_with('\r') {
                    self.line_buf.pop();
                }
                return Ok(Some(core::mem::take(&mut self.line_buf)));
            }
            self.line_buf.push(buf[0] as char);
        }
    }

    pub async fn read_exact_bytes(&mut self, n: usize) -> io::Result<Vec<u8>> {
        let mut buf = vec![0u8; n];
        self.reader.read_exact(&mut buf).await?;
        Ok(buf)
    }

    pub async fn read_exact_string(&mut self, n: usize) -> io::Result<String> {
        let bytes = self.read_exact_bytes(n).await?;
        String::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}
