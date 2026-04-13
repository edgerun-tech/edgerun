//! AsyncRead / AsyncWrite traits and extension methods.

use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

// ===========================================================================
// Traits
// ===========================================================================

/// Async read trait.
pub trait AsyncRead {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>>;
}

/// Async write trait.
pub trait AsyncWrite {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>>;
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>>;
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>>;
}

// ===========================================================================
// Extension traits — async convenience methods
// ===========================================================================

pub trait AsyncReadExt: AsyncRead + Unpin {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> ReadFut<'a, Self>
    where
        Self: Sized,
    {
        ReadFut { s: self, buf }
    }
    fn read_exact<'a>(&'a mut self, buf: &'a mut [u8]) -> ReadExactFut<'a, Self>
    where
        Self: Sized,
    {
        ReadExactFut { s: self, buf, pos: 0 }
    }
    /// Read all bytes until EOF, appending to `buf`.
    fn read_to_end<'a>(&'a mut self, buf: &'a mut Vec<u8>) -> ReadToEndFut<'a, Self>
    where
        Self: Sized,
    {
        ReadToEndFut { s: self, buf }
    }
    /// Read all bytes until EOF and return as a `String`.
    fn read_to_string<'a>(&'a mut self) -> ReadToStringFut<'a, Self>
    where
        Self: Sized,
    {
        ReadToStringFut { s: self, buf: String::new() }
    }
    /// Create an adaptor that reads at most `limit` bytes.
    fn take(self, limit: u64) -> Take<Self>
    where
        Self: Sized,
    {
        Take { inner: self, limit, remaining: limit }
    }
    /// Chain this reader with another, reading from `self` then `other`.
    fn chain<R2>(self, other: R2) -> Chain<Self, R2>
    where
        Self: Sized,
        R2: AsyncRead + Unpin,
    {
        Chain { first: self, second: other, done_first: false }
    }

    /// Read all bytes until a newline (`\n`) is reached, appending to `buf`.
    ///
    /// The newline character is included in the buffer.
    /// Returns the number of bytes read (including the newline).
    ///
    /// # Performance
    /// This reads byte-by-byte. For efficient line reading, wrap your reader
    /// in a [`BufReader`](crate::BufReader) first.
    fn read_line<'a>(&'a mut self, buf: &'a mut String) -> ReadLineFut<'a, Self>
    where
        Self: Sized,
    {
        ReadLineFut { s: self, buf }
    }

    /// Returns a stream over the lines of this reader.
    fn lines(self) -> Lines<Self>
    where
        Self: Sized,
    {
        Lines { reader: self, buf: String::new() }
    }
}
impl<R: AsyncRead + Unpin> AsyncReadExt for R {}

pub trait AsyncWriteExt: AsyncWrite + Unpin {
    fn write_all<'a>(&'a mut self, buf: &'a [u8]) -> WriteAllFut<'a, Self>
    where
        Self: Sized,
    {
        WriteAllFut { s: self, buf, pos: 0 }
    }
    fn flush(&mut self) -> FlushFut<'_, Self>
    where
        Self: Sized,
    {
        FlushFut { s: self }
    }
    fn shutdown(&mut self) -> ShutdownFut<'_, Self>
    where
        Self: Sized,
    {
        ShutdownFut { s: self }
    }
}
impl<W: AsyncWrite + Unpin> AsyncWriteExt for W {}

// ===========================================================================
// Blanket impls for &mut references
// ===========================================================================

impl<R: AsyncRead + Unpin> AsyncRead for &mut R {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut **self.get_mut()).poll_read(cx, buf)
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for &mut W {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut **self.get_mut()).poll_write(cx, buf)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut **self.get_mut()).poll_flush(cx)
    }
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut **self.get_mut()).poll_shutdown(cx)
    }
}

// ===========================================================================
// Future types
// ===========================================================================

pub struct ReadFut<'a, R: Unpin> {
    s: &'a mut R,
    buf: &'a mut [u8],
}
impl<R: AsyncRead + Unpin> Future for ReadFut<'_, R> {
    type Output = io::Result<usize>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        Pin::new(&mut *this.s).poll_read(cx, this.buf)
    }
}

pub struct ReadExactFut<'a, R: Unpin> {
    s: &'a mut R,
    buf: &'a mut [u8],
    pos: usize,
}
impl<R: AsyncRead + Unpin> Future for ReadExactFut<'_, R> {
    type Output = io::Result<()>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        while this.pos < this.buf.len() {
            let n = match Pin::new(&mut *this.s).poll_read(cx, &mut this.buf[this.pos..]) {
                Poll::Ready(Ok(0)) => {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "early eof",
                    )));
                }
                Poll::Ready(Ok(n)) => n,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            };
            this.pos += n;
        }
        Poll::Ready(Ok(()))
    }
}

pub struct WriteAllFut<'a, W: Unpin> {
    s: &'a mut W,
    buf: &'a [u8],
    pos: usize,
}
impl<W: AsyncWrite + Unpin> Future for WriteAllFut<'_, W> {
    type Output = io::Result<()>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        while this.pos < this.buf.len() {
            let n = match Pin::new(&mut *this.s).poll_write(cx, &this.buf[this.pos..]) {
                Poll::Ready(Ok(n)) => n,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            };
            this.pos += n;
        }
        Poll::Ready(Ok(()))
    }
}

pub struct FlushFut<'a, W: Unpin> {
    s: &'a mut W,
}
impl<W: AsyncWrite + Unpin> Future for FlushFut<'_, W> {
    type Output = io::Result<()>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        Pin::new(&mut *this.s).poll_flush(cx)
    }
}

pub struct ShutdownFut<'a, W: Unpin> {
    s: &'a mut W,
}
impl<W: AsyncWrite + Unpin> Future for ShutdownFut<'_, W> {
    type Output = io::Result<()>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        Pin::new(&mut *this.s).poll_shutdown(cx)
    }
}

// ===========================================================================
// read_to_end
// ===========================================================================

pub struct ReadToEndFut<'a, R: Unpin> {
    s: &'a mut R,
    buf: &'a mut Vec<u8>,
}
impl<R: AsyncRead + Unpin> Future for ReadToEndFut<'_, R> {
    type Output = io::Result<u64>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let mut tmp = [0u8; 4096];
        match Pin::new(&mut *this.s).poll_read(cx, &mut tmp) {
            Poll::Ready(Ok(0)) => Poll::Ready(Ok(0)),
            Poll::Ready(Ok(n)) => {
                this.buf.extend_from_slice(&tmp[..n]);
                // Continue reading.
                // We need to loop until EOF. Since this is a Future, we
                // return Pending if we read something and let the caller
                // poll again. For efficiency, use a loop with pending check.
                poll_read_to_end(Pin::new(&mut *this.s), this.buf, cx)
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

fn poll_read_to_end<R: AsyncRead + Unpin>(
    mut s: Pin<&mut R>,
    buf: &mut Vec<u8>,
    cx: &mut Context<'_>,
) -> Poll<io::Result<u64>> {
    let mut total: u64 = 0;
    loop {
        let mut tmp = [0u8; 4096];
        match s.as_mut().poll_read(cx, &mut tmp) {
            Poll::Ready(Ok(0)) => return Poll::Ready(Ok(total)),
            Poll::Ready(Ok(n)) => {
                buf.extend_from_slice(&tmp[..n]);
                total += n as u64;
            }
            Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
            Poll::Pending => {
                if total > 0 {
                    return Poll::Ready(Ok(total));
                }
                return Poll::Pending;
            }
        }
    }
}

// ===========================================================================
// read_to_string
// ===========================================================================

pub struct ReadToStringFut<'a, R: Unpin> {
    s: &'a mut R,
    buf: String,
}
impl<R: AsyncRead + Unpin> Future for ReadToStringFut<'_, R> {
    type Output = io::Result<String>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let mut bytes = Vec::new();
        match poll_read_to_end(Pin::new(&mut *this.s), &mut bytes, cx) {
            Poll::Ready(Ok(_)) => {
                let s = String::from_utf8(bytes)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                Poll::Ready(Ok(s))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

// ===========================================================================
// Take
// ===========================================================================

/// An adaptor that limits the number of bytes read from the underlying reader.
pub struct Take<R> {
    inner: R,
    limit: u64,
    remaining: u64,
}

impl<R: AsyncRead + Unpin> AsyncRead for Take<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let this = unsafe { self.get_unchecked_mut() };
        if this.remaining == 0 {
            return Poll::Ready(Ok(0));
        }
        let max = std::cmp::min(buf.len() as u64, this.remaining) as usize;
        match Pin::new(&mut this.inner).poll_read(cx, &mut buf[..max]) {
            Poll::Ready(Ok(n)) => {
                this.remaining -= n as u64;
                Poll::Ready(Ok(n))
            }
            other => other,
        }
    }
}

impl<R> Take<R> {
    pub fn limit(&self) -> u64 {
        self.limit
    }
    pub fn remaining(&self) -> u64 {
        self.remaining
    }
    pub fn into_inner(self) -> R {
        self.inner
    }
    pub fn get_ref(&self) -> &R {
        &self.inner
    }
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }
    /// Returns `true` if the limit has been reached.
    pub fn is_limit_reached(&self) -> bool {
        self.remaining == 0
    }
}

// ===========================================================================
// Chain
// ===========================================================================

/// Chains two readers, reading from the first until EOF, then the second.
pub struct Chain<R1, R2> {
    first: R1,
    second: R2,
    done_first: bool,
}

impl<R1: AsyncRead + Unpin, R2: AsyncRead + Unpin> AsyncRead for Chain<R1, R2> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let this = unsafe { self.get_unchecked_mut() };
        if !this.done_first {
            match Pin::new(&mut this.first).poll_read(cx, buf) {
                Poll::Ready(Ok(0)) => {
                    this.done_first = true;
                    return Pin::new(&mut this.second).poll_read(cx, buf);
                }
                other => return other,
            }
        }
        Pin::new(&mut this.second).poll_read(cx, buf)
    }
}

// ===========================================================================
// read_line
// ===========================================================================

pub struct ReadLineFut<'a, R: Unpin> {
    s: &'a mut R,
    buf: &'a mut String,
}

impl<R: AsyncRead + Unpin> Future for ReadLineFut<'_, R> {
    type Output = io::Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };

        // Check if we already have a newline in the buffer.
        if let Some(pos) = this.buf.as_bytes().iter().position(|&b| b == b'\n') {
            let len = pos + 1;
            return Poll::Ready(Ok(len));
        }

        let start_len = this.buf.len();
        let mut byte_buf = [0u8; 1];
        match Pin::new(&mut *this.s).poll_read(cx, &mut byte_buf) {
            Poll::Ready(Ok(0)) => {
                // EOF — return what we have.
                if this.buf.len() > start_len {
                    return Poll::Ready(Ok(this.buf.len() - start_len));
                }
                Poll::Ready(Ok(0))
            }
            Poll::Ready(Ok(1)) => {
                this.buf.push(byte_buf[0] as char);
                if byte_buf[0] == b'\n' {
                    return Poll::Ready(Ok(this.buf.len() - start_len));
                }
                // No newline — return Pending so we get woken for more data.
                Poll::Pending
            }
            Poll::Ready(Ok(_)) => unreachable!("read of 1-byte buffer returned > 1"),
            Poll::Ready(Err(e)) => {
                if this.buf.len() > start_len {
                    return Poll::Ready(Ok(this.buf.len() - start_len));
                }
                Poll::Ready(Err(e))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

// ===========================================================================
// Lines
// ===========================================================================

/// An async iterator over the lines of an `AsyncRead`.
pub struct Lines<R> {
    reader: R,
    buf: String,
}

impl<R: AsyncRead + Unpin> Lines<R> {
    /// Read the next line.
    pub async fn next_line(&mut self) -> io::Result<Option<String>> {
        self.buf.clear();
        let n = self.reader.read_line(&mut self.buf).await?;
        if n == 0 {
            Ok(None)
        } else {
            // Strip trailing \n or \r\n.
            let line = if self.buf.ends_with('\n') {
                self.buf.pop();
                if self.buf.ends_with('\r') {
                    self.buf.pop();
                }
                std::mem::take(&mut self.buf)
            } else {
                std::mem::take(&mut self.buf)
            };
            Ok(Some(line))
        }
    }

    /// Consumes the `Lines` and returns the underlying reader.
    pub fn into_reader(self) -> R {
        self.reader
    }
}

// ===========================================================================
// poll_fn — create a Future from a closure
// ===========================================================================

/// Creates a new future from a closure that takes a `Context` and returns `Poll`.
///
/// This is useful for creating one-off futures without defining a separate
/// struct. The closure is called each time the future is polled.
///
/// # Example
/// ```ignore
/// use edgerun_rt::poll_fn;
/// let val = poll_fn(|cx| {
///     // do some work
///     std::task::Poll::Ready(42)
/// }).await;
/// ```
pub fn poll_fn<T, F>(f: F) -> PollFn<F>
where
    F: FnMut(&mut Context<'_>) -> Poll<T>,
{
    PollFn { f }
}

pub struct PollFn<F> {
    f: F,
}

impl<T, F> Future for PollFn<F>
where
    F: FnMut(&mut Context<'_>) -> Poll<T>,
{
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let this = unsafe { self.get_unchecked_mut() };
        (this.f)(cx)
    }
}

impl<F> Unpin for PollFn<F> {}
