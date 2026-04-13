//! Shared protocol utilities — line reader and ESMTP extension list.

use std::io;

use edgerun_rt::AsyncReadExt;

/// Default ESMTP extensions advertised in EHLO.
pub const ESMTP_EXTENSIONS: &[&str] = &[
    "PIPELINING",
    "SIZE 35882577",
    "8BITMIME",
    "ENHANCEDSTATUSCODES",
    "SMTPUTF8",
];

// ===========================================================================
// SmtpReader
// ===========================================================================

/// Reads SMTP commands / responses line-by-line from an async stream.
pub struct SmtpReader<R> {
    reader: R,
    line_buf: String,
}

impl<R: edgerun_rt::AsyncRead + Unpin> SmtpReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            line_buf: String::with_capacity(1024),
        }
    }

    /// Read the next line (until `\r\n`). Returns `None` on EOF.
    pub async fn read_line(&mut self) -> io::Result<Option<String>> {
        self.line_buf.clear();
        loop {
            let mut buf = [0u8; 1];
            let n = match self.reader.read(&mut buf).await {
                Ok(0) => {
                    if self.line_buf.is_empty() {
                        return Ok(None);
                    }
                    return Ok(Some(std::mem::take(&mut self.line_buf)));
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
                return Ok(Some(std::mem::take(&mut self.line_buf)));
            }
            self.line_buf.push(buf[0] as char);
        }
    }
}
