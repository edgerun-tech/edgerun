//! Streaming HTTP body support.
//!
//! [`Body`] represents an HTTP message body. It can be:
//! - **Buffered**: fully read into memory (from `Content-Length` or completed chunked read)
//! - **Streaming**: chunks arrive asynchronously (active chunked transfer)
//!
//! Use [`Body::reader()`] to get a [`BodyReader`] for async chunk-by-chunk reading,
//! or [`Body::collect()`] to read the entire body at once.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use edgerun_rt::mpsc::{self, Receiver, Sender};

// ---------------------------------------------------------------------------
// Body
// ---------------------------------------------------------------------------

/// An HTTP message body that can be read asynchronously.
pub struct Body {
    inner: BodyInner,
}

enum BodyInner {
    /// Entire body is in memory.
    Full(Vec<u8>),
    /// Streaming body backed by an mpsc channel.
    Stream(StreamBody),
}

struct StreamBody {
    rx: Receiver<Vec<u8>>,
}

impl Body {
    /// Create a streaming body with the given channel capacity.
    /// Returns the body and a [`BodySender`] to push data into it.
    pub fn new(capacity: usize) -> (Self, BodySender) {
        let (tx, rx) = mpsc::channel::<Vec<u8>>(capacity);
        (
            Body {
                inner: BodyInner::Stream(StreamBody { rx }),
            },
            BodySender { tx },
        )
    }

    /// Create a body that is already fully buffered.
    pub fn full(data: Vec<u8>) -> Self {
        Body {
            inner: BodyInner::Full(data),
        }
    }

    /// Create an empty body.
    pub fn empty() -> Self {
        Body {
            inner: BodyInner::Full(Vec::new()),
        }
    }

    /// True if the body is known to be empty.
    pub fn is_empty(&self) -> bool {
        match &self.inner {
            BodyInner::Full(data) => data.is_empty(),
            BodyInner::Stream(_) => false,
        }
    }

    /// Read the entire body into a single `Vec<u8>`.
    pub async fn collect(self) -> std::io::Result<Vec<u8>> {
        match self.inner {
            BodyInner::Full(data) => Ok(data),
            BodyInner::Stream(stream) => {
                let mut data = Vec::new();
                while let Some(chunk) = stream.rx.recv().await {
                    data.extend_from_slice(&chunk);
                }
                Ok(data)
            }
        }
    }

    /// Get a reader for async chunk-by-chunk reading.
    pub fn reader(self) -> BodyReader {
        match self.inner {
            BodyInner::Full(data) => BodyReader {
                inner: BodyReaderInner::Full { data, pos: 0 },
            },
            BodyInner::Stream(stream) => BodyReader {
                inner: BodyReaderInner::Stream { rx: stream.rx },
            },
        }
    }
}

// ---------------------------------------------------------------------------
// BodyReader
// ---------------------------------------------------------------------------

/// Async reader for a [`Body`].
///
/// Call [`read_chunk`](BodyReader::read_chunk) until it returns `Ok(None)`.
pub struct BodyReader {
    inner: BodyReaderInner,
}

enum BodyReaderInner {
    /// Buffered body — return slices from `pos`.
    Full { data: Vec<u8>, pos: usize },
    /// Streaming body — receive chunks from channel.
    Stream { rx: Receiver<Vec<u8>> },
}

impl BodyReader {
    /// Read the next chunk of body data.
    ///
    /// - `Ok(Some(data))` — body chunk received.
    /// - `Ok(None)` — body fully read (EOF).
    /// - `Err(e)` — I/O or channel error.
    pub fn read_chunk(&mut self) -> ReadChunkFut<'_> {
        ReadChunkFut { reader: self }
    }
}

/// Future for [`BodyReader::read_chunk`].
pub struct ReadChunkFut<'a> {
    reader: &'a mut BodyReader,
}

impl Future for ReadChunkFut<'_> {
    type Output = std::io::Result<Option<Vec<u8>>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };

        match &mut this.reader.inner {
            BodyReaderInner::Full { data, pos } => {
                if *pos >= data.len() {
                    Poll::Ready(Ok(None))
                } else {
                    let remaining = data.len() - *pos;
                    let chunk_size = remaining.min(8192);
                    let chunk = data[*pos..*pos + chunk_size].to_vec();
                    *pos += chunk_size;
                    Poll::Ready(Ok(Some(chunk)))
                }
            }
            BodyReaderInner::Stream { rx } => {
                // try_recv first
                match rx.try_recv() {
                    Ok(chunk) => Poll::Ready(Ok(Some(chunk))),
                    Err(edgerun_rt::mpsc::TryRecvError::Empty) => {
                        // Need to wait — poll the recv future
                        let raw: *const Receiver<Vec<u8>> = rx;
                        let fut = unsafe { (&*raw).recv() };
                        std::pin::pin!(fut).poll(cx).map(|r| match r {
                            Some(chunk) => Ok(Some(chunk)),
                            None => Ok(None),
                        })
                    }
                    Err(edgerun_rt::mpsc::TryRecvError::Disconnected) => Poll::Ready(Ok(None)),
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// BodySender (write side of a streaming body)
// ---------------------------------------------------------------------------

/// Write end of a streaming [`Body`].
///
/// Created alongside a `Body` via [`Body::new`].
/// Push chunks with [`send`](BodySender::send). The body ends when the
/// sender is dropped (channel closes).
pub struct BodySender {
    tx: Sender<Vec<u8>>,
}

impl BodySender {
    /// Send a data chunk into the body stream.
    ///
    /// Returns `Err` if the body reader has been dropped.
    pub fn send(&mut self, data: Vec<u8>) -> std::io::Result<()> {
        match self.tx.try_send(data) {
            Ok(()) => Ok(()),
            Err(_) => Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "body reader was dropped",
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// AsyncBodyReader — reads body from any AsyncRead source
// ---------------------------------------------------------------------------

use edgerun_rt::AsyncRead;

/// Reads an HTTP body from an [`AsyncRead`] source.
///
/// Supports Content-Length-based reading, chunked transfer encoding,
/// and read-until-EOF modes.
pub struct AsyncBodyReader<R> {
    reader: R,
    /// Remaining bytes to read (None for unknown-length/EOF).
    remaining: Option<u64>,
    /// True if this is chunked transfer encoding.
    chunked: bool,
    // Chunked transfer encoding state:
    chunk_buf: Vec<u8>,
    chunk_remaining: usize,
    /// Where we are in chunked parsing.
    state: ChunkState,
    /// State for trailer line detection across poll calls.
    trailer_empty_line: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChunkState {
    /// Reading the chunk-size line ("<hex>;ext\r\n").
    ChunkSize,
    /// Reading chunk data bytes.
    ChunkData,
    /// Reading trailing \r\n after chunk data.
    ChunkCrlf,
    /// Reading trailer headers after the 0-length chunk.
    Trailers,
    /// Body fully read.
    Done,
}

impl<R: AsyncRead + Unpin> AsyncBodyReader<R> {
    /// Create a body reader for a known Content-Length.
    pub fn with_length(reader: R, len: u64) -> Self {
        Self {
            reader,
            remaining: Some(len),
            chunked: false,
            chunk_buf: Vec::new(),
            chunk_remaining: 0,
            state: ChunkState::ChunkData,
            trailer_empty_line: true,
        }
    }

    /// Create a body reader for chunked transfer encoding.
    pub fn chunked(reader: R) -> Self {
        Self {
            reader,
            remaining: None,
            chunked: true,
            chunk_buf: Vec::new(),
            chunk_remaining: 0,
            state: ChunkState::ChunkSize,
            trailer_empty_line: true,
        }
    }

    /// Create a body reader that reads until EOF.
    pub fn until_eof(reader: R) -> Self {
        Self {
            reader,
            remaining: None,
            chunked: false,
            chunk_buf: Vec::new(),
            chunk_remaining: 0,
            state: ChunkState::ChunkData,
            trailer_empty_line: true,
        }
    }

    /// Read body data into the provided buffer.
    ///
    /// Returns `Ok(n)` where n is the number of bytes read, or `Ok(0)` when
    /// the body is fully read.
    pub fn read<'a, 'b>(&'a mut self, buf: &'b mut [u8]) -> ReadBodyFut<'a, 'b, R> {
        ReadBodyFut { reader: self, buf }
    }

    /// Read the entire body into a `Vec<u8>`.
    pub fn collect(self) -> CollectBodyFut<R> {
        let capacity = self.remaining.map(|r| r as usize).unwrap_or(0);
        CollectBodyFut {
            reader: self,
            data: Vec::with_capacity(capacity.min(65536)),
            read_buf: vec![0u8; 8192],
        }
    }

    /// Returns true if the body is fully consumed.
    pub fn is_done(&self) -> bool {
        self.state == ChunkState::Done || self.remaining == Some(0)
    }
}

/// Future for [`AsyncBodyReader::read`].
pub struct ReadBodyFut<'a, 'b, R> {
    reader: &'a mut AsyncBodyReader<R>,
    buf: &'b mut [u8],
}

impl<R: AsyncRead + Unpin> Future for ReadBodyFut<'_, '_, R> {
    type Output = std::io::Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };

        if !this.reader.chunked {
            if let Some(0) = this.reader.remaining {
                return Poll::Ready(Ok(0));
            }

            let max_read = this
                .reader
                .remaining
                .map(|r| (r as usize).min(this.buf.len()))
                .unwrap_or(this.buf.len());

            if max_read == 0 {
                this.reader.remaining = Some(0);
                return Poll::Ready(Ok(0));
            }

            let n = {
                let pinned = Pin::new(&mut this.reader.reader);
                pinned.poll_read(cx, &mut this.buf[..max_read])?
            };

            match n {
                Poll::Ready(0) => {
                    this.reader.remaining = Some(0);
                    Poll::Ready(Ok(0))
                }
                Poll::Ready(n) => {
                    if let Some(ref mut rem) = this.reader.remaining {
                        *rem -= n as u64;
                    }
                    Poll::Ready(Ok(n))
                }
                Poll::Pending => Poll::Pending,
            }
        } else {
            this.poll_chunked_read(cx)
        }
    }
}

impl<R: AsyncRead + Unpin> ReadBodyFut<'_, '_, R> {
    fn poll_chunked_read(&mut self, cx: &mut Context<'_>) -> Poll<std::io::Result<usize>> {
        loop {
            match self.reader.state {
                ChunkState::ChunkSize => {
                    // Read chunk-size line
                    // We need to find \r\n — use a small buffer
                    let mut line = Vec::with_capacity(32);
                    loop {
                        let mut byte = [0u8; 1];
                        let n = {
                            let pinned = Pin::new(&mut self.reader.reader);
                            match pinned.poll_read(cx, &mut byte) {
                                Poll::Ready(Ok(n)) => n,
                                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                                Poll::Pending => return Poll::Pending,
                            }
                        };
                        if n == 0 {
                            return Poll::Ready(Err(std::io::Error::new(
                                std::io::ErrorKind::UnexpectedEof,
                                "unexpected EOF reading chunk size",
                            )));
                        }
                        if byte[0] == b'\n' {
                            break;
                        }
                        line.push(byte[0]);
                    }
                    // Remove trailing \r
                    if line.last() == Some(&b'\r') {
                        line.pop();
                    }

                    let size_hex = std::str::from_utf8(&line).map_err(|_| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "invalid chunk size encoding",
                        )
                    })?;
                    let size_str = size_hex.split(';').next().unwrap_or(size_hex).trim();
                    let chunk_size = usize::from_str_radix(size_str, 16).map_err(|_| {
                        std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid chunk size")
                    })?;

                    if chunk_size == 0 {
                        self.reader.state = ChunkState::Trailers;
                        continue;
                    }

                    self.reader.chunk_remaining = chunk_size;
                    self.reader.state = ChunkState::ChunkData;
                    // Fall through to read data
                }
                ChunkState::ChunkData => {
                    let to_read = self.reader.chunk_remaining.min(self.buf.len());
                    if to_read == 0 {
                        self.reader.state = ChunkState::ChunkCrlf;
                        continue;
                    }
                    let n = {
                        let pinned = Pin::new(&mut self.reader.reader);
                        match pinned.poll_read(cx, &mut self.buf[..to_read]) {
                            Poll::Ready(Ok(n)) => n,
                            Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                            Poll::Pending => return Poll::Pending,
                        }
                    };
                    if n == 0 {
                        return Poll::Ready(Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "unexpected EOF reading chunk data",
                        )));
                    }
                    self.reader.chunk_remaining -= n;
                    if self.reader.chunk_remaining == 0 {
                        self.reader.state = ChunkState::ChunkCrlf;
                    }
                    return Poll::Ready(Ok(n));
                }
                ChunkState::ChunkCrlf => {
                    // Skip the \r\n after chunk data
                    let mut crlf_buf = [0u8; 2];
                    let n = {
                        let pinned = Pin::new(&mut self.reader.reader);
                        match pinned.poll_read(cx, &mut crlf_buf) {
                            Poll::Ready(Ok(n)) => n,
                            Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                            Poll::Pending => return Poll::Pending,
                        }
                    };
                    if n == 0 {
                        return Poll::Ready(Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "unexpected EOF after chunk data",
                        )));
                    }
                    self.reader.state = ChunkState::ChunkSize;
                    // Loop back to read next chunk size
                }
                ChunkState::Trailers => {
                    // Read trailer headers until a blank line.
                    // Trailer headers are discarded (not exposed to application).
                    // RFC 9112: trailers are terminated by a blank line.
                    loop {
                        let mut byte = [0u8; 1];
                        let n = {
                            let pinned = Pin::new(&mut self.reader.reader);
                            match pinned.poll_read(cx, &mut byte) {
                                Poll::Ready(Ok(n)) => n,
                                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                                Poll::Pending => return Poll::Pending,
                            }
                        };
                        if n == 0 {
                            self.reader.state = ChunkState::Done;
                            return Poll::Ready(Ok(0));
                        }
                        match byte[0] {
                            b'\n' if self.reader.trailer_empty_line => {
                                self.reader.state = ChunkState::Done;
                                return Poll::Ready(Ok(0));
                            }
                            b'\n' => {
                                self.reader.trailer_empty_line = true;
                            }
                            b'\r' => {}
                            _ => {
                                self.reader.trailer_empty_line = false;
                            }
                        }
                    }
                }
                ChunkState::Done => {
                    return Poll::Ready(Ok(0));
                }
            }
        }
    }
}

/// Future for [`AsyncBodyReader::collect`].
pub struct CollectBodyFut<R> {
    reader: AsyncBodyReader<R>,
    data: Vec<u8>,
    read_buf: Vec<u8>,
}

impl<R: AsyncRead + Unpin> Future for CollectBodyFut<R> {
    type Output = std::io::Result<Vec<u8>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };

        loop {
            let n = {
                let pinned = Pin::new(&mut this.reader.reader);
                match pinned.poll_read(cx, &mut this.read_buf) {
                    Poll::Ready(Ok(n)) => n,
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                    Poll::Pending => return Poll::Pending,
                }
            };

            if n == 0 {
                let data = std::mem::take(&mut this.data);
                return Poll::Ready(Ok(data));
            }

            this.data.extend_from_slice(&this.read_buf[..n]);

            if let Some(rem) = &mut this.reader.remaining {
                *rem -= n as u64;
                if *rem == 0 {
                    let data = std::mem::take(&mut this.data);
                    return Poll::Ready(Ok(data));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_rt::Runtime;

    #[test]
    fn test_body_full_collect() {
        let body = Body::full(b"hello world".to_vec());
        let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
        let result = rt.block_on(body.collect());
        assert_eq!(result.unwrap(), b"hello world");
    }

    #[test]
    fn test_body_empty() {
        let body = Body::empty();
        assert!(body.is_empty());
        let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
        let result = rt.block_on(body.collect());
        assert_eq!(result.unwrap(), b"");
    }

    #[test]
    fn test_body_reader_buffered() {
        let body = Body::full(b"abc123".to_vec());
        let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
        let result = rt.block_on(async {
            let mut reader = body.reader();
            let mut all = Vec::new();
            while let Some(chunk) = reader.read_chunk().await.unwrap() {
                all.extend_from_slice(&chunk);
            }
            all
        });
        assert_eq!(result, b"abc123");
    }
}
