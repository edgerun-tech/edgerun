//! AsyncRead / AsyncWrite traits and extension methods (no_std).


extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Other,
    UnexpectedEof,
    InvalidData,
    WouldBlock,
}

impl Error {
    pub const fn new(kind: ErrorKind) -> Self {
        Self { kind }
    }
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

pub trait AsyncRead {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize, Error>>;
}

pub trait AsyncWrite {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>>;
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>>;
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>>;
}

impl<R: AsyncRead + Unpin> AsyncRead for &mut R {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize, Error>> {
        Pin::new(&mut **self.get_mut()).poll_read(cx, buf)
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for &mut W {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        Pin::new(&mut **self.get_mut()).poll_write(cx, buf)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Pin::new(&mut **self.get_mut()).poll_flush(cx)
    }
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Pin::new(&mut **self.get_mut()).poll_shutdown(cx)
    }
}

impl<T: AsyncRead + Unpin> AsyncRead for Pin<&mut T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize, Error>> {
        Pin::new(&mut **self).poll_read(cx, buf)
    }
}

impl<T: AsyncWrite + Unpin> AsyncWrite for Pin<&mut T> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        Pin::new(&mut **self).poll_write(cx, buf)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Pin::new(&mut **self).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Pin::new(&mut **self).poll_shutdown(cx)
    }
}

pub struct ReadFut<'a, R: Unpin> {
    s: &'a mut R,
    buf: &'a mut [u8],
}
impl<R: AsyncRead + Unpin> Future for ReadFut<'_, R> {
    type Output = Result<usize, Error>;
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
    type Output = Result<(), Error>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        while this.pos < this.buf.len() {
            let n = match Pin::new(&mut *this.s).poll_read(cx, &mut this.buf[this.pos..]) {
                Poll::Ready(Ok(0)) => {
                    return Poll::Ready(Err(Error::new(ErrorKind::UnexpectedEof)));
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
    type Output = Result<(), Error>;
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
    type Output = Result<(), Error>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        Pin::new(&mut *this.s).poll_flush(cx)
    }
}

pub struct ShutdownFut<'a, W: Unpin> {
    s: &'a mut W,
}
impl<W: AsyncWrite + Unpin> Future for ShutdownFut<'_, W> {
    type Output = Result<(), Error>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        Pin::new(&mut *this.s).poll_shutdown(cx)
    }
}

pub struct ReadToEndFut<'a, R: Unpin> {
    s: &'a mut R,
    buf: &'a mut Vec<u8>,
}
impl<R: AsyncRead + Unpin> Future for ReadToEndFut<'_, R> {
    type Output = Result<u64, Error>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        poll_read_to_end(Pin::new(&mut *this.s), this.buf, cx)
    }
}

fn poll_read_to_end<R: AsyncRead + Unpin>(
    mut s: Pin<&mut R>,
    buf: &mut Vec<u8>,
    cx: &mut Context<'_>,
) -> Poll<Result<u64, Error>> {
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

pub struct Take<R> {
    inner: R,
    remaining: u64,
}

impl<R: AsyncRead + Unpin> AsyncRead for Take<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize, Error>> {
        let this = unsafe { self.get_unchecked_mut() };
        if this.remaining == 0 {
            return Poll::Ready(Ok(0));
        }
        let max = core::cmp::min(buf.len() as u64, this.remaining) as usize;
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
    pub fn remaining(&self) -> u64 {
        self.remaining
    }
    pub fn into_inner(self) -> R {
        self.inner
    }
}

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
    ) -> Poll<Result<usize, Error>> {
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

// ===========================================================================
// AsyncReadExt
// ===========================================================================

pub trait AsyncReadExt: AsyncRead + Sized {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> ReadFut<'a, Self>
    where
        Self: Unpin,
    {
        ReadFut { s: self, buf }
    }

    fn read_exact<'a>(&'a mut self, buf: &'a mut [u8]) -> ReadExactFut<'a, Self>
    where
        Self: Unpin,
    {
        ReadExactFut {
            s: self,
            buf,
            pos: 0,
        }
    }

    fn read_to_end<'a>(&'a mut self, buf: &'a mut Vec<u8>) -> ReadToEndFut<'a, Self>
    where
        Self: Unpin,
    {
        ReadToEndFut { s: self, buf }
    }

    fn take(self, limit: u64) -> Take<Self> {
        Take {
            inner: self,
            remaining: limit,
        }
    }

    fn chain<R2>(self, other: R2) -> Chain<Self, R2>
    where
        R2: AsyncRead + Unpin,
    {
        Chain {
            first: self,
            second: other,
            done_first: false,
        }
    }
}

impl<R: AsyncRead + Sized> AsyncReadExt for R {}

pub trait AsyncWriteExt: AsyncWrite + Sized {
    fn write_all<'a>(&'a mut self, buf: &'a [u8]) -> WriteAllFut<'a, Self>
    where
        Self: Unpin,
    {
        WriteAllFut {
            s: self,
            buf,
            pos: 0,
        }
    }

    fn flush(&mut self) -> FlushFut<'_, Self>
    where
        Self: Unpin,
    {
        FlushFut { s: self }
    }

    fn shutdown(&mut self) -> ShutdownFut<'_, Self>
    where
        Self: Unpin,
    {
        ShutdownFut { s: self }
    }
}

impl<W: AsyncWrite + Sized> AsyncWriteExt for W {}