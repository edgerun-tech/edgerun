//! Async `Cursor` — in-memory reader/writer over `&[u8]` and `Vec<u8>`.
//!
//! Wraps a byte slice or vector as an `AsyncRead` (and optionally
//! `AsyncWrite` for `Vec<u8>`), similar to `std::io::Cursor`.

use std::io::{self, SeekFrom};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::io_traits::{AsyncRead, AsyncWrite};

// ===========================================================================
// Cursor<T> — generic over inner type
// ===========================================================================

/// An async cursor over an in-memory buffer.
///
/// For `&[u8]` it implements `AsyncRead` only.
/// For `Vec<u8>` it implements both `AsyncRead` and `AsyncWrite`.
pub struct Cursor<T> {
    inner: T,
    pos: u64,
}

impl<T: AsRef<[u8]>> Cursor<T> {
    /// Create a new cursor over the given buffer.
    pub fn new(inner: T) -> Self {
        Self { inner, pos: 0 }
    }

    /// Returns the current position.
    pub fn position(&self) -> u64 {
        self.pos
    }

    /// Sets the current position.
    pub fn set_position(&mut self, pos: u64) {
        self.pos = pos;
    }

    /// Gets a reference to the underlying buffer.
    pub fn get_ref(&self) -> &T {
        &self.inner
    }

    /// Gets a mutable reference to the underlying buffer.
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Consumes the cursor, returning the underlying buffer.
    pub fn into_inner(self) -> T {
        self.inner
    }

    /// Returns the total length of the underlying buffer.
    fn len(&self) -> u64 {
        self.inner.as_ref().len() as u64
    }

    /// Seeks to a position within the buffer.
    pub fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let len = self.len();
        let new_pos = match pos {
            SeekFrom::Start(n) => n,
            SeekFrom::End(n) => {
                if n >= 0 {
                    (len as i64 + n) as u64
                } else {
                    (len as i64 + n).try_into().map_err(|_| {
                        io::Error::new(io::ErrorKind::InvalidInput, "seek before start")
                    })?
                }
            }
            SeekFrom::Current(n) => {
                let new = self.pos as i64 + n;
                new.try_into()
                    .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "seek before start"))?
            }
        };
        self.pos = new_pos.min(len);
        Ok(self.pos)
    }
}

impl<T: AsRef<[u8]> + Unpin> AsyncRead for Cursor<T> {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let this = unsafe { self.get_unchecked_mut() };
        let slice = this.inner.as_ref();
        let start = this.pos as usize;
        if start >= slice.len() {
            return Poll::Ready(Ok(0));
        }
        let available = &slice[start..];
        let n = std::cmp::min(buf.len(), available.len());
        buf[..n].copy_from_slice(&available[..n]);
        this.pos += n as u64;
        Poll::Ready(Ok(n))
    }
}

// ===========================================================================
// Cursor<Vec<u8>> — AsyncWrite support
// ===========================================================================

impl AsyncWrite for Cursor<Vec<u8>> {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = unsafe { self.get_unchecked_mut() };
        let pos = this.pos as usize;

        // If writing past the end, extend the vector.
        if pos >= this.inner.len() {
            this.inner.resize(pos, 0);
        }

        let end = pos + buf.len();
        if end > this.inner.len() {
            this.inner.resize(end, 0);
        }

        this.inner[pos..pos + buf.len()].copy_from_slice(buf);
        this.pos += buf.len() as u64;
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
