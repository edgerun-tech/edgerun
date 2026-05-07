//! Runtime compatibility layer for email protocols during bare-rt migration.

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::io;

pub use edgerun_rt::{
    sleep, spawn, spawn_blocking, timeout, AsyncTcpListener, AsyncTcpStream, CancellationToken,
    ConnectFuture, Duration, Instant, JoinError, JoinHandle,
};

pub(crate) fn bare_io(error: edgerun_rt::IoError) -> io::Error {
    match error {
        edgerun_rt::IoError::UnexpectedEof => io::Error::new(io::ErrorKind::UnexpectedEof, error),
        edgerun_rt::IoError::WriteZero => io::Error::new(io::ErrorKind::WriteZero, error),
        edgerun_rt::IoError::Other(_) => io::Error::other(error),
    }
}

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

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>>;
}

impl<T: edgerun_rt::AsyncRead + Unpin> AsyncRead for T {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        edgerun_rt::AsyncRead::poll_read(self, cx, buf).map_err(bare_io)
    }
}

impl<T: edgerun_rt::AsyncWrite + Unpin> AsyncWrite for T {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        edgerun_rt::AsyncWrite::poll_write(self, cx, buf).map_err(bare_io)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        edgerun_rt::AsyncWrite::poll_flush(self, cx).map_err(bare_io)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        edgerun_rt::AsyncWrite::poll_shutdown(self, cx).map_err(bare_io)
    }
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

pub struct ReadFut<'a, R: ?Sized> {
    reader: &'a mut R,
    buf: &'a mut [u8],
}

impl<R: AsyncRead + Unpin + ?Sized> Future for ReadFut<'_, R> {
    type Output = io::Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let reader = unsafe { Pin::new_unchecked(&mut *this.reader) };
        reader.poll_read(cx, this.buf)
    }
}

pub struct ReadExactFut<'a, R: ?Sized> {
    reader: &'a mut R,
    buf: &'a mut [u8],
    pos: usize,
}

impl<R: AsyncRead + Unpin + ?Sized> Future for ReadExactFut<'_, R> {
    type Output = io::Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        while self.pos < self.buf.len() {
            let pos = self.pos;
            let this = unsafe { self.as_mut().get_unchecked_mut() };
            let reader = unsafe { Pin::new_unchecked(&mut *this.reader) };
            match reader.poll_read(cx, &mut this.buf[pos..]) {
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
    writer: &'a mut W,
    buf: &'a [u8],
    pos: usize,
}

impl<W: AsyncWrite + Unpin + ?Sized> Future for WriteAllFut<'_, W> {
    type Output = io::Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        while self.pos < self.buf.len() {
            let pos = self.pos;
            let this = unsafe { self.as_mut().get_unchecked_mut() };
            let writer = unsafe { Pin::new_unchecked(&mut *this.writer) };
            match writer.poll_write(cx, &this.buf[pos..]) {
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
    writer: &'a mut W,
}

impl<W: AsyncWrite + Unpin + ?Sized> Future for FlushFut<'_, W> {
    type Output = io::Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let writer = unsafe { Pin::new_unchecked(&mut *this.writer) };
        writer.poll_flush(cx)
    }
}

pub struct ShutdownFut<'a, W: ?Sized> {
    writer: &'a mut W,
}

impl<W: AsyncWrite + Unpin + ?Sized> Future for ShutdownFut<'_, W> {
    type Output = io::Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let writer = unsafe { Pin::new_unchecked(&mut *this.writer) };
        writer.poll_shutdown(cx)
    }
}

pub struct Mutex<T>(edgerun_rt::Mutex<T>);

impl<T> Mutex<T> {
    pub fn new(value: T) -> Self {
        Self(edgerun_rt::Mutex::new(value))
    }

    pub fn lock(&self) -> LockFuture<'_, T> {
        LockFuture { mutex: &self.0 }
    }
}

pub struct LockFuture<'a, T> {
    mutex: &'a edgerun_rt::Mutex<T>,
}

impl<'a, T> Future for LockFuture<'a, T> {
    type Output = edgerun_rt::MutexGuard<'a, T>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.mutex.lock())
    }
}

pub struct RwLock<T>(edgerun_rt::RwLock<T>);

impl<T> RwLock<T> {
    pub fn new(value: T) -> Self {
        Self(edgerun_rt::RwLock::new(value))
    }

    pub fn read(&self) -> ReadLockFuture<'_, T> {
        ReadLockFuture { lock: &self.0 }
    }

    pub fn write(&self) -> WriteLockFuture<'_, T> {
        WriteLockFuture { lock: &self.0 }
    }
}

pub struct ReadLockFuture<'a, T> {
    lock: &'a edgerun_rt::RwLock<T>,
}

impl<'a, T> Future for ReadLockFuture<'a, T> {
    type Output = edgerun_rt::RwLockReadGuard<'a, T>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.lock.read())
    }
}

pub struct WriteLockFuture<'a, T> {
    lock: &'a edgerun_rt::RwLock<T>,
}

impl<'a, T> Future for WriteLockFuture<'a, T> {
    type Output = edgerun_rt::RwLockWriteGuard<'a, T>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.lock.write())
    }
}
