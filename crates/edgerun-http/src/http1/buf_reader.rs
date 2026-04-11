//! Buffered async reader for HTTP header parsing.
//!
//! Wraps an [`AsyncRead`] source with an internal buffer, providing
//! efficient line-by-line reading for HTTP headers and raw byte reading
//! for body data.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use edgerun_rt::{AsyncRead, AsyncWrite};

const DEFAULT_BUF_SIZE: usize = 8192;

/// A buffered async reader.
///
/// Wraps an `AsyncRead` and buffers data internally, allowing efficient
/// line-by-line reading (for HTTP headers) without per-byte syscalls.
pub struct BufReader<R> {
    inner: R,
    buf: Vec<u8>,
    /// Position in buf of the next byte to read.
    pos: usize,
    /// Number of valid bytes in buf.
    len: usize,
}

impl<R: AsyncRead + Unpin> BufReader<R> {
    /// Create a new BufReader with the default buffer size.
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            buf: vec![0u8; DEFAULT_BUF_SIZE],
            pos: 0,
            len: 0,
        }
    }

    /// Create a new BufReader with the specified buffer capacity.
    pub fn with_capacity(capacity: usize, inner: R) -> Self {
        Self {
            inner,
            buf: vec![0u8; capacity],
            pos: 0,
            len: 0,
        }
    }

    /// Get a reference to the inner reader.
    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    /// Get a mutable reference to the inner reader.
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Consume the BufReader, returning the inner reader.
    pub fn into_inner(self) -> R {
        self.inner
    }

    /// Read bytes from the inner reader into the buffer.
    fn fill_buf(&mut self, cx: &mut Context<'_>) -> Poll<std::io::Result<&[u8]>> {
        if self.pos < self.len {
            // Data already in buffer
            Poll::Ready(Ok(&self.buf[self.pos..self.len]))
        } else {
            // Need to read more
            self.pos = 0;
            self.len = 0;
            let n = {
                let pinned = Pin::new(&mut self.inner);
                match pinned.poll_read(cx, &mut self.buf) {
                    Poll::Ready(Ok(n)) => n,
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                    Poll::Pending => return Poll::Pending,
                }
            };
            self.len = n;
            Poll::Ready(Ok(&self.buf[..n]))
        }
    }

    /// Consume `amt` bytes from the buffer.
    fn consume(&mut self, amt: usize) {
        self.pos += amt;
    }

    /// Read a line (up to and including `\r\n` or `\n`).
    ///
    /// Returns the line WITHOUT the line terminator.
    /// Returns `Ok(None)` on EOF with no data.
    pub fn read_line(&mut self) -> ReadLineFut<'_, R> {
        ReadLineFut { reader: self }
    }

    /// Read exactly `n` bytes.
    pub fn read_exact<'a, 'b>(&'a mut self, buf: &'b mut [u8]) -> ReadExactFromBufFut<'a, 'b, R> {
        ReadExactFromBufFut { reader: self, out: buf, pos: 0 }
    }

    /// Read up to `buf.len()` bytes directly into `buf`.
    /// Returns the number of bytes read. `Ok(0)` means EOF.
    pub fn read<'a, 'b>(&'a mut self, buf: &'b mut [u8]) -> ReadFromBufFut<'a, 'b, R> {
        ReadFromBufFut { reader: self, out: buf }
    }
}

/// Future for [`BufReader::read_line`].
pub struct ReadLineFut<'a, R> {
    reader: &'a mut BufReader<R>,
}

impl<R: AsyncRead + Unpin> Future for ReadLineFut<'_, R> {
    type Output = std::io::Result<Option<String>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let mut line = Vec::new();

        loop {
            let poll_buf = this.reader.fill_buf(cx)?;
            let available = match poll_buf {
                Poll::Ready(buf) => buf,
                Poll::Pending => {
                    if !line.is_empty() {
                        return Poll::Ready(Ok(Some(
                            String::from_utf8(line).map_err(|_| std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "invalid UTF-8 in request line",
                            ))?,
                        )));
                    }
                    return Poll::Pending;
                }
            };

            if available.is_empty() {
                return Poll::Ready(if line.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(String::from_utf8(line).map_err(|_| std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "invalid UTF-8 in request line",
                    ))?))
                });
            }

            // Look for \n in available data
            if let Some(newline_pos) = available.iter().position(|&b| b == b'\n') {
                let consumed = newline_pos + 1;
                let line_end = if newline_pos > 0 && available[newline_pos - 1] == b'\r' {
                    newline_pos - 1
                } else {
                    newline_pos
                };
                line.extend_from_slice(&available[..line_end]);
                this.reader.consume(consumed);
                return Poll::Ready(Ok(Some(String::from_utf8(line).map_err(|_| std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "invalid UTF-8 in request line",
                ))?)));
            }

            // No newline yet — consume all available data and continue
            line.extend_from_slice(available);
            let avail_len = available.len();
            this.reader.consume(avail_len);
        }
    }
}

/// Future for [`BufReader::read_exact`].
pub struct ReadExactFromBufFut<'a, 'b, R> {
    reader: &'a mut BufReader<R>,
    out: &'b mut [u8],
    pos: usize,
}

impl<R: AsyncRead + Unpin> Future for ReadExactFromBufFut<'_, '_, R> {
    type Output = std::io::Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let out_len = this.out.len();

        while this.pos < out_len {
            let available = match this.reader.fill_buf(cx)? {
                Poll::Ready(buf) => buf,
                Poll::Pending => return Poll::Pending,
            };

            if available.is_empty() {
                return Poll::Ready(Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "failed to fill buffer",
                )));
            }

            let to_copy = available.len().min(out_len - this.pos);
            this.out[this.pos..this.pos + to_copy].copy_from_slice(&available[..to_copy]);
            this.pos += to_copy;
            this.reader.consume(to_copy);
        }

        Poll::Ready(Ok(()))
    }
}

/// Future for [`BufReader::read`].
pub struct ReadFromBufFut<'a, 'b, R> {
    reader: &'a mut BufReader<R>,
    out: &'b mut [u8],
}

impl<R: AsyncRead + Unpin> Future for ReadFromBufFut<'_, '_, R> {
    type Output = std::io::Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };

        // First, try to serve from buffer
        if this.reader.pos < this.reader.len {
            let available = &this.reader.buf[this.reader.pos..this.reader.len];
            let to_copy = available.len().min(this.out.len());
            this.out[..to_copy].copy_from_slice(&available[..to_copy]);
            this.reader.pos += to_copy;
            return Poll::Ready(Ok(to_copy));
        }

        // Buffer empty — read from inner
        let n = {
            let pinned = Pin::new(&mut this.reader.inner);
            match pinned.poll_read(cx, this.out) {
                Poll::Ready(Ok(n)) => n,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        };

        Poll::Ready(Ok(n))
    }
}

// ---------------------------------------------------------------------------
// AsyncWrite impl — forwards to inner writer
// ---------------------------------------------------------------------------

impl<R: AsyncRead + AsyncWrite + Unpin> AsyncWrite for BufReader<R> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        Pin::new(&mut this.inner).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        Pin::new(&mut this.inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        Pin::new(&mut this.inner).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_rt::Runtime;
    use std::io::Cursor;

    #[test]
    fn test_read_line_crlf() {
        let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
        let data = b"GET / HTTP/1.1\r\nHost: example.com\r\n\r\n";

        // Wrap in a simple AsyncRead wrapper around a Cursor
        let cursor = Cursor::new(data.to_vec());
        let cursor_reader = CursorAsyncRead(cursor);

        let result = rt.block_on(async {
            let mut reader = BufReader::new(cursor_reader);
            let line1 = reader.read_line().await.unwrap();
            let line2 = reader.read_line().await.unwrap();
            let line3 = reader.read_line().await.unwrap();
            (line1, line2, line3)
        });

        assert_eq!(result.0, Some("GET / HTTP/1.1".to_string()));
        assert_eq!(result.1, Some("Host: example.com".to_string()));
        assert_eq!(result.2, Some("".to_string()));
    }

    /// AsyncRead wrapper around std::io::Cursor for testing.
    struct CursorAsyncRead(Cursor<Vec<u8>>);

    impl AsyncRead for CursorAsyncRead {
        fn poll_read(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<std::io::Result<usize>> {
            let this = self.get_mut();
            let available = &this.0.get_ref()[this.0.position() as usize..];
            let to_read = buf.len().min(available.len());
            buf[..to_read].copy_from_slice(&available[..to_read]);
            this.0.set_position(this.0.position() + to_read as u64);
            Poll::Ready(Ok(to_read))
        }
    }

    #[test]
    fn test_read_line_rejects_invalid_utf8() {
        let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
        // Invalid UTF-8: 0x80 is a continuation byte without a start byte
        let data = b"GET /path\x80 HTTP/1.1\r\n";

        let cursor = Cursor::new(data.to_vec());
        let cursor_reader = CursorAsyncRead(cursor);

        let result = rt.block_on(async {
            let mut reader = BufReader::new(cursor_reader);
            reader.read_line().await
        });

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidData);
    }
}
