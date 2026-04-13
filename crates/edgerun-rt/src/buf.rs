//! Buffered async I/O — `BufReader` and `BufWriter`.
//!
//! Wraps any `AsyncRead` or `AsyncWrite` with an in-memory buffer
//! to reduce the number of raw read/write calls.

use std::io::{self};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::io_traits::{AsyncRead, AsyncWrite, AsyncWriteExt};

// ===========================================================================
// BufReader
// ===========================================================================

/// Wraps an `AsyncRead` with a buffer for efficiency.
pub struct BufReader<R> {
    inner: R,
    buf: Vec<u8>,
    pos: usize,
}

impl<R: AsyncRead + Unpin> BufReader<R> {
    /// Create a new `BufReader` with default capacity (8 KB).
    pub fn new(inner: R) -> Self {
        Self::with_capacity(8192, inner)
    }

    /// Create a new `BufReader` with the specified buffer capacity.
    pub fn with_capacity(cap: usize, inner: R) -> Self {
        Self {
            inner,
            buf: vec![0u8; cap],
            pos: 0,
        }
    }

    /// Gets a reference to the underlying reader.
    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    /// Gets a mutable reference to the underlying reader.
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Consumes the `BufReader`, returning the underlying reader.
    pub fn into_inner(self) -> R {
        self.inner
    }

    /// Returns the number of bytes available in the buffer.
    pub fn buffered(&self) -> usize {
        self.pos
    }

    /// Returns a reference to the buffered data without advancing.
    pub fn fill_buf(&mut self) -> &[u8] {
        &self.buf[..self.pos]
    }

    /// Consumes `amt` bytes from the buffer.
    pub fn consume(&mut self, amt: usize) {
        let amt = std::cmp::min(amt, self.pos);
        self.pos -= amt;
        if self.pos > 0 {
            self.buf.copy_within(amt..amt + self.pos, 0);
        }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for BufReader<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let this = unsafe { self.get_unchecked_mut() };

        // If buffer is empty, refill from inner reader.
        if this.pos == 0 {
            let cap = this.buf.len();
            match Pin::new(&mut this.inner).poll_read(cx, &mut this.buf[..cap]) {
                Poll::Ready(Ok(0)) => return Poll::Ready(Ok(0)),
                Poll::Ready(Ok(n)) => this.pos = n,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }

        let n = std::cmp::min(buf.len(), this.pos);
        buf[..n].copy_from_slice(&this.buf[..n]);
        this.pos -= n;
        if this.pos > 0 {
            this.buf.copy_within(n..n + this.pos, 0);
        }
        Poll::Ready(Ok(n))
    }
}

impl<R: AsyncRead + Unpin + AsyncWrite> AsyncWrite for BufReader<R> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        unsafe { self.map_unchecked_mut(|s| &mut s.inner) }.poll_write(cx, buf)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        unsafe { self.map_unchecked_mut(|s| &mut s.inner) }.poll_flush(cx)
    }
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        unsafe { self.map_unchecked_mut(|s| &mut s.inner) }.poll_shutdown(cx)
    }
}

// ===========================================================================
// BufWriter
// ===========================================================================

/// Wraps an `AsyncWrite` with a buffer for efficiency.
pub struct BufWriter<W> {
    inner: W,
    buf: Vec<u8>,
    capacity: usize,
}

impl<W: AsyncWrite + Unpin> BufWriter<W> {
    /// Create a new `BufWriter` with default capacity (8 KB).
    pub fn new(inner: W) -> Self {
        Self::with_capacity(8192, inner)
    }

    /// Create a new `BufWriter` with the specified buffer capacity.
    pub fn with_capacity(cap: usize, inner: W) -> Self {
        Self {
            inner,
            buf: Vec::with_capacity(cap),
            capacity: cap,
        }
    }

    /// Gets a reference to the underlying writer.
    pub fn get_ref(&self) -> &W {
        &self.inner
    }

    /// Gets a mutable reference to the underlying writer.
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    /// Returns the underlying writer without flushing.
    ///
    /// Any buffered data is lost. Call [`Self::flush()`] first if
    /// you need to preserve buffered data.
    pub fn into_inner_without_flush(self) -> W {
        self.inner
    }

    /// Returns the number of bytes currently buffered.
    pub fn buffered(&self) -> usize {
        self.buf.len()
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for BufWriter<W> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = unsafe { self.get_unchecked_mut() };

        // If adding would exceed capacity, try to flush.
        if this.buf.len() + buf.len() > this.capacity && !this.buf.is_empty() {
            match Pin::new(&mut this.inner).poll_write(cx, &this.buf) {
                Poll::Ready(Ok(n)) => { this.buf.drain(..n); }
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }

        // Direct write if larger than buffer and buffer is empty.
        if buf.len() >= this.capacity && this.buf.is_empty() {
            return Pin::new(&mut this.inner).poll_write(cx, buf);
        }

        // Buffer the write.
        let available = this.capacity - this.buf.len();
        let n = std::cmp::min(buf.len(), available);
        this.buf.extend_from_slice(&buf[..n]);
        Poll::Ready(Ok(n))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = unsafe { self.get_unchecked_mut() };
        if this.buf.is_empty() {
            return Pin::new(&mut this.inner).poll_flush(cx);
        }
        match Pin::new(&mut this.inner).poll_write(cx, &this.buf) {
            Poll::Ready(Ok(n)) => {
                this.buf.drain(..n);
                if this.buf.is_empty() {
                    Pin::new(&mut this.inner).poll_flush(cx)
                } else {
                    Poll::Pending
                }
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = unsafe { self.get_unchecked_mut() };
        if !this.buf.is_empty() {
            match Pin::new(&mut this.inner).poll_write(cx, &this.buf) {
                Poll::Ready(Ok(n)) => {
                    this.buf.drain(..n);
                    if !this.buf.is_empty() {
                        return Poll::Pending;
                    }
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }
        Pin::new(&mut this.inner).poll_shutdown(cx)
    }
}
