//! Buffered async I/O — `BufReader` and `BufWriter`.
//!
//! Wraps any `AsyncRead` or `AsyncWrite` with an in-memory buffer
//! to reduce the number of raw read/write calls.

use std::future::Future;
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

    /// Read a line (up to and including `\r\n` or `\n`).
    ///
    /// Returns the line WITHOUT the line terminator.
    /// Returns `Ok(None)` on EOF with no data.
    pub fn read_line(&mut self) -> ReadLineFut<'_, R> {
        ReadLineFut {
            reader: self,
            max_size: usize::MAX,
            line: Vec::new(),
        }
    }

    /// Read a line with a maximum size limit.
    ///
    /// If the line exceeds `max_size` bytes, returns an error with
    /// `ErrorKind::InvalidData`.
    pub fn read_line_max(&mut self, max_size: usize) -> ReadLineFut<'_, R> {
        ReadLineFut {
            reader: self,
            max_size,
            line: Vec::new(),
        }
    }

    /// Read exactly `n` bytes.
    pub fn read_exact<'a, 'b>(&'a mut self, buf: &'b mut [u8]) -> ReadExactFut<'a, 'b, R> {
        ReadExactFut {
            reader: self,
            out: buf,
            out_pos: 0,
        }
    }

    /// Read up to `buf.len()` bytes.
    /// Returns the number of bytes read. `Ok(0)` means EOF.
    pub fn read<'a, 'b>(&'a mut self, buf: &'b mut [u8]) -> ReadFut<'a, 'b, R> {
        ReadFut {
            reader: self,
            out: buf,
        }
    }

    /// Refill the internal buffer from the inner reader.
    /// Returns `Poll::Ready(Ok(&[u8]))` with available data,
    /// `Poll::Ready(Ok(&[]))` on EOF, or `Poll::Pending`.
    fn fill_buf_poll(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<&[u8]>> {
        if self.pos > 0 {
            Poll::Ready(Ok(&self.buf[..self.pos]))
        } else {
            let cap = self.buf.len();
            match Pin::new(&mut self.inner).poll_read(cx, &mut self.buf[..cap]) {
                Poll::Ready(Ok(0)) => Poll::Ready(Ok(&[])),
                Poll::Ready(Ok(n)) => {
                    self.pos = n;
                    Poll::Ready(Ok(&self.buf[..n]))
                }
                Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                Poll::Pending => Poll::Pending,
            }
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

        // Serve from buffer first.
        if this.pos > 0 {
            let n = std::cmp::min(buf.len(), this.pos);
            buf[..n].copy_from_slice(&this.buf[..n]);
            this.pos -= n;
            if this.pos > 0 {
                this.buf.copy_within(n..n + this.pos, 0);
            }
            return Poll::Ready(Ok(n));
        }

        // Buffer empty — read from inner.
        let cap = this.buf.len();
        match Pin::new(&mut this.inner).poll_read(cx, &mut this.buf[..cap]) {
            Poll::Ready(Ok(0)) => Poll::Ready(Ok(0)),
            Poll::Ready(Ok(n)) => {
                this.pos = n;
                let n2 = std::cmp::min(buf.len(), n);
                buf[..n2].copy_from_slice(&this.buf[..n2]);
                this.pos -= n2;
                if this.pos > 0 {
                    this.buf.copy_within(n2..n2 + this.pos, 0);
                }
                Poll::Ready(Ok(n2))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<R: AsyncRead + Unpin + AsyncWrite> AsyncWrite for BufReader<R> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
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
// ReadLineFut
// ===========================================================================

/// Future for [`BufReader::read_line`] and [`BufReader::read_line_max`].
pub struct ReadLineFut<'a, R> {
    reader: &'a mut BufReader<R>,
    max_size: usize,
    line: Vec<u8>,
}

impl<R: AsyncRead + Unpin> Future for ReadLineFut<'_, R> {
    type Output = io::Result<Option<String>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let max_size = self.max_size;

        loop {
            let line_len = self.line.len();

            match self.reader.fill_buf_poll(cx)? {
                Poll::Ready(available) => {
                    if available.is_empty() {
                        return Poll::Ready(if line_len == 0 {
                            Ok(None)
                        } else {
                            let line = std::mem::take(&mut self.line);
                            Ok(Some(String::from_utf8(line).map_err(|_| {
                                io::Error::new(io::ErrorKind::InvalidData, "invalid UTF-8 in line")
                            })?))
                        });
                    }

                    // Check for newline.
                    if let Some(newline_pos) = available.iter().position(|&b| b == b'\n') {
                        let consumed = newline_pos + 1;
                        let has_cr = newline_pos > 0 && available[newline_pos - 1] == b'\r';
                        let line_end = if has_cr { newline_pos - 1 } else { newline_pos };
                        let data: Vec<u8> = available[..line_end].to_vec();
                        self.reader.consume(consumed);
                        self.line.extend_from_slice(&data);
                        let line = std::mem::take(&mut self.line);
                        return Poll::Ready(Ok(Some(String::from_utf8(line).map_err(|_| {
                            io::Error::new(io::ErrorKind::InvalidData, "invalid UTF-8 in line")
                        })?)));
                    }

                    // No newline — check size limit.
                    let avail_len = available.len();
                    if line_len + avail_len > max_size {
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "line too long",
                        )));
                    }

                    // Copy data and consume.
                    let data: Vec<u8> = available.to_vec();
                    self.reader.consume(avail_len);
                    self.line.extend_from_slice(&data);
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

// ===========================================================================
// ReadExactFut
// ===========================================================================

/// Future for [`BufReader::read_exact`].
pub struct ReadExactFut<'a, 'b, R> {
    reader: &'a mut BufReader<R>,
    out: &'b mut [u8],
    out_pos: usize,
}

impl<R: AsyncRead + Unpin> Future for ReadExactFut<'_, '_, R> {
    type Output = io::Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let out_len = this.out.len();

        while this.out_pos < out_len {
            match this.reader.fill_buf_poll(cx)? {
                Poll::Ready(available) => {
                    if available.is_empty() {
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "failed to fill buffer",
                        )));
                    }
                    let to_copy = available.len().min(out_len - this.out_pos);
                    this.out[this.out_pos..this.out_pos + to_copy]
                        .copy_from_slice(&available[..to_copy]);
                    this.out_pos += to_copy;
                    this.reader.consume(to_copy);
                }
                Poll::Pending => return Poll::Pending,
            }
        }

        Poll::Ready(Ok(()))
    }
}

// ===========================================================================
// ReadFut
// ===========================================================================

/// Future for [`BufReader::read`].
pub struct ReadFut<'a, 'b, R> {
    reader: &'a mut BufReader<R>,
    out: &'b mut [u8],
}

impl<R: AsyncRead + Unpin> Future for ReadFut<'_, '_, R> {
    type Output = io::Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };

        // Serve from buffer first.
        if this.reader.pos > 0 {
            let available = &this.reader.buf[..this.reader.pos];
            let to_copy = available.len().min(this.out.len());
            this.out[..to_copy].copy_from_slice(&available[..to_copy]);
            this.reader.pos -= to_copy;
            if this.reader.pos > 0 {
                this.reader
                    .buf
                    .copy_within(to_copy..to_copy + this.reader.pos, 0);
            }
            return Poll::Ready(Ok(to_copy));
        }

        // Buffer empty — read from inner directly.
        let cap = this.reader.buf.len();
        match Pin::new(&mut this.reader.inner).poll_read(cx, this.out) {
            Poll::Ready(Ok(n)) => Poll::Ready(Ok(n)),
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
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
                Poll::Ready(Ok(n)) => {
                    this.buf.drain(..n);
                }
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
