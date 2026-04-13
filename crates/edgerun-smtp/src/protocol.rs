//! Shared protocol utilities — line reader and ESMTP extension list.

use std::io;

use edgerun_rt::{AsyncRead, AsyncReadExt};

/// Default ESMTP extensions advertised in EHLO.
///
/// NOTE: PIPELINING is NOT advertised because the server processes commands
/// synchronously (one at a time). Advertising PIPELINING when the server
/// cannot handle pipelined commands is a protocol violation (RFC 2931).
pub const ESMTP_EXTENSIONS: &[&str] = &[
    "SIZE 35882577",
    "8BITMIME",
    "ENHANCEDSTATUSCODES",
    "SMTPUTF8",
    "CHUNKING",
];

/// Reads a single SMTP line (until `\r\n`) from an async reader.
///
/// Returns `None` on EOF. The line does NOT include the `\r\n`.
pub async fn read_smtp_line<R: AsyncRead + Unpin>(reader: &mut R) -> io::Result<Option<String>> {
    let mut line_buf = String::with_capacity(1024);
    loop {
        let mut buf = [0u8; 1];
        let n = match reader.read(&mut buf).await {
            Ok(0) => {
                if line_buf.is_empty() {
                    return Ok(None);
                }
                return Ok(Some(line_buf));
            }
            Ok(n) => n,
            Err(e) => return Err(e),
        };
        if n == 0 {
            continue;
        }
        if buf[0] == b'\n' {
            if line_buf.ends_with('\r') {
                line_buf.pop();
            }
            return Ok(Some(line_buf));
        }
        line_buf.push(buf[0] as char);
    }
}

/// Owned line reader — wraps any `AsyncRead + Unpin` and reads `\r\n`-delimited lines.
///
/// Used by the SMTP client which stores the reader as a field.
/// For the server session loop, prefer [`read_smtp_line`] to avoid ownership issues.
pub struct SmtpReader<R> {
    reader: R,
}

impl<R: AsyncRead + Unpin> SmtpReader<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    /// Read the next line (until `\r\n`). Returns `None` on EOF.
    pub async fn read_line(&mut self) -> io::Result<Option<String>> {
        read_smtp_line(&mut self.reader).await
    }

    /// Consume the reader and return the inner reader.
    pub fn into_inner(self) -> R {
        self.reader
    }
}
