//! Async I/O primitives for bare-metal

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Debug)]
pub enum IoError {
    UnexpectedEof,
    WriteZero,
    Other(&'static str),
}

impl core::fmt::Display for IoError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnexpectedEof => write!(f, "unexpected end of file"),
            Self::WriteZero => write!(f, "write zero"),
            Self::Other(s) => write!(f, "{}", s),
        }
    }
}

impl core::error::Error for IoError {}

#[cfg(not(target_os = "none"))]
impl From<IoError> for std::io::Error {
    fn from(error: IoError) -> Self {
        match error {
            IoError::UnexpectedEof => std::io::ErrorKind::UnexpectedEof.into(),
            IoError::WriteZero => std::io::ErrorKind::WriteZero.into(),
            IoError::Other(message) => std::io::Error::other(message),
        }
    }
}

pub type Result<T> = core::result::Result<T, IoError>;

pub trait AsyncRead {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8])
        -> Poll<Result<usize>>;
}

pub trait AsyncWrite {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>>;
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>>;
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.poll_flush(cx)
    }
}

pub trait AsyncBufRead: AsyncRead {
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<&[u8]>>;
}

pub trait AsyncReadExt: AsyncRead + Unpin {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> ReadFut<'a, Self>
    where
        Self: Sized,
    {
        ReadFut { reader: self, buf }
    }

    fn read_exact<'a>(&'a mut self, buf: &'a mut [u8]) -> ReadExactFut<'a, Self>
    where
        Self: Sized,
    {
        ReadExactFut {
            reader: self,
            buf,
            pos: 0,
        }
    }

    fn read_to_end<'a>(&'a mut self, buf: &'a mut Vec<u8>) -> ReadToEndFut<'a, Self>
    where
        Self: Sized,
    {
        ReadToEndFut { reader: self, buf }
    }

    fn read_to_string<'a>(&'a mut self) -> ReadToStringFut<'a, Self>
    where
        Self: Sized,
    {
        ReadToStringFut {
            reader: self,
            buf: String::new(),
        }
    }
}

impl<R: AsyncRead + Unpin> AsyncReadExt for R {}

pub trait AsyncWriteExt: AsyncWrite + Unpin {
    fn write_all<'a>(&'a mut self, buf: &'a [u8]) -> WriteAllFut<'a, Self>
    where
        Self: Sized,
    {
        WriteAllFut {
            writer: self,
            buf,
            pos: 0,
        }
    }

    fn flush(&mut self) -> FlushFut<'_, Self>
    where
        Self: Sized,
    {
        FlushFut { writer: self }
    }

    fn shutdown(&mut self) -> ShutdownFut<'_, Self>
    where
        Self: Sized,
    {
        ShutdownFut { writer: self }
    }
}

impl<W: AsyncWrite + Unpin> AsyncWriteExt for W {}

pub async fn copy<R, W>(reader: &mut R, writer: &mut W) -> Result<u64>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut total = 0;
    let mut buf = [0u8; 8192];
    loop {
        let n = reader.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        writer.write_all(&buf[..n]).await?;
        total += n as u64;
    }
    Ok(total)
}

pub async fn copy_bidirectional<A, B>(a: &mut A, b: &mut B) -> Result<(u64, u64)>
where
    A: AsyncRead + AsyncWrite + Unpin,
    B: AsyncRead + AsyncWrite + Unpin,
{
    let mut buf_a = [0u8; 8192];
    let mut buf_b = [0u8; 8192];
    let mut a_to_b = 0;
    let mut b_to_a = 0;

    loop {
        let n = match a.read(&mut buf_a).await {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };
        if b.write_all(&buf_a[..n]).await.is_err() {
            break;
        }
        a_to_b += n as u64;

        let n = match b.read(&mut buf_b).await {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };
        if a.write_all(&buf_b[..n]).await.is_err() {
            break;
        }
        b_to_a += n as u64;
    }

    Ok((a_to_b, b_to_a))
}

impl<R: AsyncRead + Unpin> AsyncRead for &mut R {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        Pin::new(&mut **self.get_mut()).poll_read(cx, buf)
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for &mut W {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        Pin::new(&mut **self.get_mut()).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(&mut **self.get_mut()).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(&mut **self.get_mut()).poll_shutdown(cx)
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for Pin<&mut R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        self.get_mut().as_mut().poll_read(cx, buf)
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for Pin<&mut W> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        self.get_mut().as_mut().poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.get_mut().as_mut().poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.get_mut().as_mut().poll_shutdown(cx)
    }
}

pub struct Cursor<T> {
    data: T,
    pos: usize,
}

impl<T> Cursor<T> {
    pub const fn new(data: T) -> Self {
        Self { data, pos: 0 }
    }

    pub fn position(&self) -> usize {
        self.pos
    }
}

impl AsyncRead for Cursor<alloc::vec::Vec<u8>> {
    fn poll_read(self: Pin<&mut Self>, _: &mut Context<'_>, buf: &mut [u8]) -> Poll<Result<usize>> {
        let this = self.get_mut();
        if this.pos >= this.data.len() {
            return Poll::Ready(Ok(0));
        }
        let n = buf.len().min(this.data.len() - this.pos);
        buf[..n].copy_from_slice(&this.data[this.pos..this.pos + n]);
        this.pos += n;
        Poll::Ready(Ok(n))
    }
}

impl AsyncWrite for Cursor<alloc::vec::Vec<u8>> {
    fn poll_write(self: Pin<&mut Self>, _: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        let this = self.get_mut();
        this.data.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }
}

impl AsyncBufRead for Cursor<alloc::vec::Vec<u8>> {
    fn poll_fill_buf(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<&[u8]>> {
        let this = self.get_mut();
        let remaining = &this.data[this.pos..];
        Poll::Ready(Ok(remaining))
    }
}

pub struct ReadFut<'a, R: Unpin> {
    reader: &'a mut R,
    buf: &'a mut [u8],
}

impl<R: AsyncRead + Unpin> Future for ReadFut<'_, R> {
    type Output = Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        Pin::new(&mut *this.reader).poll_read(cx, this.buf)
    }
}

pub struct ReadExactFut<'a, R: Unpin> {
    reader: &'a mut R,
    buf: &'a mut [u8],
    pos: usize,
}

impl<R: AsyncRead + Unpin> Future for ReadExactFut<'_, R> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        while this.pos < this.buf.len() {
            match Pin::new(&mut *this.reader).poll_read(cx, &mut this.buf[this.pos..]) {
                Poll::Ready(Ok(0)) => return Poll::Ready(Err(IoError::UnexpectedEof)),
                Poll::Ready(Ok(n)) => this.pos += n,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }
        Poll::Ready(Ok(()))
    }
}

pub struct ReadToEndFut<'a, R: Unpin> {
    reader: &'a mut R,
    buf: &'a mut Vec<u8>,
}

impl<R: AsyncRead + Unpin> Future for ReadToEndFut<'_, R> {
    type Output = Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let initial_len = this.buf.len();
        let mut chunk = [0u8; 1024];
        loop {
            match Pin::new(&mut *this.reader).poll_read(cx, &mut chunk) {
                Poll::Ready(Ok(0)) => return Poll::Ready(Ok(this.buf.len() - initial_len)),
                Poll::Ready(Ok(n)) => this.buf.extend_from_slice(&chunk[..n]),
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

pub struct ReadToStringFut<'a, R: Unpin> {
    reader: &'a mut R,
    buf: String,
}

impl<R: AsyncRead + Unpin> Future for ReadToStringFut<'_, R> {
    type Output = Result<String>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let mut bytes = Vec::new();
        let mut chunk = [0u8; 1024];
        loop {
            match Pin::new(&mut *this.reader).poll_read(cx, &mut chunk) {
                Poll::Ready(Ok(0)) => {
                    this.buf = String::from_utf8(bytes).map_err(|_| IoError::Other("utf8"))?;
                    return Poll::Ready(Ok(core::mem::take(&mut this.buf)));
                }
                Poll::Ready(Ok(n)) => bytes.extend_from_slice(&chunk[..n]),
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

pub struct WriteAllFut<'a, W: Unpin> {
    writer: &'a mut W,
    buf: &'a [u8],
    pos: usize,
}

impl<W: AsyncWrite + Unpin> Future for WriteAllFut<'_, W> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        while this.pos < this.buf.len() {
            match Pin::new(&mut *this.writer).poll_write(cx, &this.buf[this.pos..]) {
                Poll::Ready(Ok(0)) => return Poll::Ready(Err(IoError::WriteZero)),
                Poll::Ready(Ok(n)) => this.pos += n,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }
        Pin::new(&mut *this.writer).poll_flush(cx)
    }
}

pub struct FlushFut<'a, W: Unpin> {
    writer: &'a mut W,
}

impl<W: AsyncWrite + Unpin> Future for FlushFut<'_, W> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut *self.get_mut().writer).poll_flush(cx)
    }
}

pub struct ShutdownFut<'a, W: Unpin> {
    writer: &'a mut W,
}

impl<W: AsyncWrite + Unpin> Future for ShutdownFut<'_, W> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut *self.get_mut().writer).poll_shutdown(cx)
    }
}

pub struct BufReader<R> {
    inner: R,
    buf: Vec<u8>,
    pos: usize,
}

impl<R: AsyncRead + Unpin> BufReader<R> {
    pub fn new(inner: R) -> Self {
        Self::with_capacity(8192, inner)
    }

    pub fn with_capacity(cap: usize, inner: R) -> Self {
        Self {
            inner,
            buf: vec![0; cap],
            pos: 0,
        }
    }

    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    pub fn into_inner(self) -> R {
        self.inner
    }

    pub fn buffered(&self) -> usize {
        self.pos
    }

    pub fn fill_buf(&mut self) -> &[u8] {
        &self.buf[..self.pos]
    }

    pub fn consume(&mut self, amt: usize) {
        let amt = amt.min(self.pos);
        self.pos -= amt;
        if self.pos > 0 {
            self.buf.copy_within(amt..amt + self.pos, 0);
        }
    }

    pub fn read_line(&mut self) -> BufReadLineFut<'_, R> {
        self.read_line_max(usize::MAX)
    }

    pub fn read_line_max(&mut self, max_size: usize) -> BufReadLineFut<'_, R> {
        BufReadLineFut {
            reader: self,
            max_size,
            line: Vec::new(),
        }
    }

    fn fill_buf_poll(&mut self, cx: &mut Context<'_>) -> Poll<Result<&[u8]>> {
        if self.pos > 0 {
            return Poll::Ready(Ok(&self.buf[..self.pos]));
        }
        let cap = self.buf.len();
        match Pin::new(&mut self.inner).poll_read(cx, &mut self.buf[..cap]) {
            Poll::Ready(Ok(n)) => {
                self.pos = n;
                Poll::Ready(Ok(&self.buf[..n]))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for BufReader<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        out: &mut [u8],
    ) -> Poll<Result<usize>> {
        let this = self.get_mut();
        if this.pos > 0 {
            let n = out.len().min(this.pos);
            out[..n].copy_from_slice(&this.buf[..n]);
            this.consume(n);
            return Poll::Ready(Ok(n));
        }
        Pin::new(&mut this.inner).poll_read(cx, out)
    }
}

impl<R: AsyncRead + AsyncWrite + Unpin> AsyncWrite for BufReader<R> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        Pin::new(&mut self.get_mut().inner).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}

pub struct BufReadLineFut<'a, R> {
    reader: &'a mut BufReader<R>,
    max_size: usize,
    line: Vec<u8>,
}

impl<R: AsyncRead + Unpin> Future for BufReadLineFut<'_, R> {
    type Output = Result<Option<String>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        loop {
            match this.reader.fill_buf_poll(cx) {
                Poll::Ready(Ok(available)) => {
                    if available.is_empty() {
                        if this.line.is_empty() {
                            return Poll::Ready(Ok(None));
                        }
                        let line = core::mem::take(&mut this.line);
                        return Poll::Ready(
                            String::from_utf8(line)
                                .map(Some)
                                .map_err(|_| IoError::Other("utf8")),
                        );
                    }

                    if let Some(newline_pos) = available.iter().position(|&b| b == b'\n') {
                        let consumed = newline_pos + 1;
                        let line_end = if newline_pos > 0 && available[newline_pos - 1] == b'\r' {
                            newline_pos - 1
                        } else {
                            newline_pos
                        };
                        let chunk = available[..line_end].to_vec();
                        this.reader.consume(consumed);
                        this.line.extend_from_slice(&chunk);
                        let line = core::mem::take(&mut this.line);
                        return Poll::Ready(
                            String::from_utf8(line)
                                .map(Some)
                                .map_err(|_| IoError::Other("utf8")),
                        );
                    }

                    if this.line.len() + available.len() > this.max_size {
                        return Poll::Ready(Err(IoError::Other("line too long")));
                    }
                    let chunk = available.to_vec();
                    let consumed = available.len();
                    this.reader.consume(consumed);
                    this.line.extend_from_slice(&chunk);
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

pub struct ReadToEnd<R> {
    reader: R,
    buf: alloc::vec::Vec<u8>,
}

impl<R: AsyncRead + Unpin> Unpin for ReadToEnd<R> {}

impl<R: AsyncRead + Unpin> Future for ReadToEnd<R> {
    type Output = Result<alloc::vec::Vec<u8>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().get_mut();
        let mut chunk = [0u8; 1024];
        loop {
            match Pin::new(&mut this.reader).poll_read(cx, &mut chunk) {
                Poll::Ready(Ok(0)) => return Poll::Ready(Ok(core::mem::take(&mut this.buf))),
                Poll::Ready(Ok(n)) => {
                    this.buf.extend_from_slice(&chunk[..n]);
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

pub fn read_to_end<R>(reader: R) -> ReadToEnd<R>
where
    R: AsyncRead + Unpin,
{
    ReadToEnd {
        reader,
        buf: alloc::vec::Vec::new(),
    }
}

pub struct WriteAll<W> {
    writer: W,
    buf: alloc::vec::Vec<u8>,
    written: usize,
}

impl<W: AsyncWrite + Unpin> Unpin for WriteAll<W> {}

impl<W: AsyncWrite + Unpin> Future for WriteAll<W> {
    type Output = Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().get_mut();
        while this.written < this.buf.len() {
            match Pin::new(&mut this.writer).poll_write(cx, &this.buf[this.written..]) {
                Poll::Ready(Ok(n)) => this.written += n,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }
        Pin::new(&mut this.writer).poll_flush(cx)
    }
}

pub fn write_all<W>(writer: W, data: &[u8]) -> WriteAll<W>
where
    W: AsyncWrite + Unpin,
{
    WriteAll {
        writer,
        buf: data.to_vec(),
        written: 0,
    }
}

pub struct ReadExact<R> {
    reader: R,
    buf: alloc::vec::Vec<u8>,
    remaining: usize,
}

impl<R: AsyncRead + Unpin> Unpin for ReadExact<R> {}

impl<R: AsyncRead + Unpin> Future for ReadExact<R> {
    type Output = Result<alloc::vec::Vec<u8>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().get_mut();
        if this.remaining == 0 {
            return Poll::Ready(Ok(core::mem::take(&mut this.buf)));
        }
        let mut chunk = [0u8; 1024];
        match Pin::new(&mut this.reader).poll_read(cx, &mut chunk) {
            Poll::Ready(Ok(0)) => Poll::Ready(Err(IoError::UnexpectedEof)),
            Poll::Ready(Ok(n)) => {
                this.buf.extend_from_slice(&chunk[..n]);
                this.remaining -= n;
                Poll::Pending
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub fn read_exact<R>(reader: R, n: usize) -> ReadExact<R> {
    ReadExact {
        reader,
        buf: alloc::vec::Vec::new(),
        remaining: n,
    }
}

pub struct Lines<B> {
    buf: B,
    line: alloc::vec::Vec<u8>,
}

impl<B: AsyncBufRead + Unpin> Unpin for Lines<B> {}

impl<B: AsyncBufRead + Unpin> Future for Lines<B> {
    type Output = Result<Option<alloc::string::String>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().get_mut();
        loop {
            match Pin::new(&mut this.buf).poll_fill_buf(cx) {
                Poll::Ready(Ok(s)) => {
                    if let Some(i) = s.iter().position(|&b| b == b'\n') {
                        return Poll::Ready(Ok(Some(
                            alloc::string::String::from_utf8_lossy(&s[..i]).into(),
                        )));
                    } else {
                        this.line.extend_from_slice(s);
                        this.line.push(b'\n');
                    }
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

pub fn lines<B>(buf: B) -> Lines<B>
where
    B: AsyncBufRead + Unpin,
{
    Lines {
        buf,
        line: alloc::vec::Vec::new(),
    }
}
