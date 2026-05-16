//! Shared protocol utilities — line reader and ESMTP extension list.

use crate::prelude::*;

#[cfg(not(target_os = "none"))]
use std::io;

#[cfg(not(target_os = "none"))]
use crate::rt::{AsyncRead, AsyncReadExt};

pub use edgerun_protocols::smtp::protocol::ESMTP_EXTENSIONS;

/// Owned line reader — wraps any `AsyncRead + Unpin` and reads `\r\n`-delimited lines.
///
/// Used by the SMTP client which stores the reader as a field.
/// For the server session loop, prefer [`crate::server::read_line`] to avoid ownership issues.
#[cfg(not(target_os = "none"))]
pub struct SmtpReader<R> {
    reader: R,
}

#[cfg(not(target_os = "none"))]
impl<R: AsyncRead + Unpin> SmtpReader<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    /// Read the next line (until `\r\n`). Returns `None` on EOF.
    pub async fn read_line(&mut self) -> io::Result<Option<String>> {
        crate::server::read_line(&mut self.reader).await
    }

    /// Consume the reader and return the inner reader.
    pub fn into_inner(self) -> R {
        self.reader
    }
}
