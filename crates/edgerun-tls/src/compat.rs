//! Async I/O compatibility for TLS over edgerun-rt transports.

use crate::std::io;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub trait AsyncRead {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>>;
}

pub trait AsyncWrite {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>>;

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>>;

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

impl<T: edgerun_rt::AsyncRead + Unpin> AsyncRead for T {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        edgerun_rt::AsyncRead::poll_read(self, cx, buf).map_err(io::Error::from)
    }
}

impl<T: edgerun_rt::AsyncWrite + Unpin> AsyncWrite for T {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        edgerun_rt::AsyncWrite::poll_write(self, cx, buf).map_err(io::Error::from)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        edgerun_rt::AsyncWrite::poll_flush(self, cx).map_err(io::Error::from)
    }
}

pub trait AsyncReadExt: AsyncRead + Unpin {
    fn read_exact<'a>(&'a mut self, buf: &'a mut [u8]) -> ReadExactFut<'a, Self>
    where
        Self: Sized,
    {
        ReadExactFut {
            stream: self,
            buf,
            pos: 0,
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
            stream: self,
            buf,
            pos: 0,
        }
    }

    fn flush(&mut self) -> FlushFut<'_, Self>
    where
        Self: Sized,
    {
        FlushFut { stream: self }
    }
}

impl<W: AsyncWrite + Unpin> AsyncWriteExt for W {}

pub struct ReadExactFut<'a, R: ?Sized> {
    stream: &'a mut R,
    buf: &'a mut [u8],
    pos: usize,
}

impl<R: AsyncRead + Unpin + ?Sized> Future for ReadExactFut<'_, R> {
    type Output = io::Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        while self.pos < self.buf.len() {
            let pos = self.pos;
            let this = unsafe { self.as_mut().get_unchecked_mut() };
            let stream = unsafe { Pin::new_unchecked(&mut *this.stream) };
            match stream.poll_read(cx, &mut this.buf[pos..]) {
                Poll::Ready(Ok(0)) => {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "failed to fill whole buffer",
                    )));
                }
                Poll::Ready(Ok(n)) => self.pos += n,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }
        Poll::Ready(Ok(()))
    }
}

pub struct WriteAllFut<'a, W: ?Sized> {
    stream: &'a mut W,
    buf: &'a [u8],
    pos: usize,
}

impl<W: AsyncWrite + Unpin + ?Sized> Future for WriteAllFut<'_, W> {
    type Output = io::Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        while self.pos < self.buf.len() {
            let pos = self.pos;
            let this = unsafe { self.as_mut().get_unchecked_mut() };
            let stream = unsafe { Pin::new_unchecked(&mut *this.stream) };
            match stream.poll_write(cx, &this.buf[pos..]) {
                Poll::Ready(Ok(0)) => {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "failed to write whole buffer",
                    )));
                }
                Poll::Ready(Ok(n)) => self.pos += n,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }
        Poll::Ready(Ok(()))
    }
}

pub struct FlushFut<'a, W: ?Sized> {
    stream: &'a mut W,
}

impl<W: AsyncWrite + Unpin + ?Sized> Future for FlushFut<'_, W> {
    type Output = io::Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let stream = unsafe { Pin::new_unchecked(&mut *this.stream) };
        stream.poll_flush(cx)
    }
}
