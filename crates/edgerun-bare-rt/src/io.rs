//! Async I/O primitives for bare-metal

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

pub type Result<T> = core::result::Result<T, IoError>;

pub trait AsyncRead {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<Result<usize>>;
}

pub trait AsyncWrite {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>>;
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>>;
}

pub trait AsyncBufRead: AsyncRead {
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<&[u8]>>;
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
    ReadToEnd { reader, buf: alloc::vec::Vec::new() }
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
    WriteAll { writer, buf: data.to_vec(), written: 0 }
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
    ReadExact { reader, buf: alloc::vec::Vec::new(), remaining: n }
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
        let mut buf = [0u8; 1024];
        loop {
            match Pin::new(&mut this.buf).poll_fill_buf(cx) {
                Poll::Ready(Ok(s)) => {
                    if let Some(i) = s.iter().position(|&b| b == b'\n') {
                        return Poll::Ready(Ok(Some(alloc::string::String::from_utf8_lossy(&s[..i]).into())));
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
    Lines { buf, line: alloc::vec::Vec::new() }
}