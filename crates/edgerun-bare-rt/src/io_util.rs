//! Async I/O utilities — `copy`, `copy_bidirectional`, `empty`, `sink`, `repeat` (no_std).

#![no_std]

extern crate alloc;

use core::pin::Pin;
use core::task::{Context, Poll};

use crate::io_traits::{AsyncRead, AsyncWrite, Error};

pub async fn copy<R, W>(reader: &mut R, writer: &mut W) -> Result<u64, Error>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut total: u64 = 0;
    let mut buf = [0u8; 8192];
    loop {
        let n = match Pin::new(&mut *reader).poll_read(&mut Context::<' static>::<'_>, &mut buf) {
            Poll::Ready(Ok(0)) => break,
            Poll::Ready(Ok(n)) => n,
            Poll::Ready(Err(e)) => return Err(e),
            Poll::Pending => continue,
        };
        if n == 0 {
            break;
        }
        match Pin::new(&mut *writer).poll_write(&mut Context::<' static>::<'_>, &buf[..n]) {
            Poll::Ready(Ok(())) => {},
            Poll::Ready(Err(e)) => return Err(e),
            Poll::Pending => {},
        }
        total += n as u64;
    }
    Ok(total)
}

pub fn empty() -> Empty {
    Empty { _p: () }
}

pub struct Empty {
    _p: (),
}

impl AsyncRead for Empty {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &mut [u8],
    ) -> Poll<Result<usize, Error>> {
        Poll::Ready(Ok(0))
    }
}

pub fn sink() -> Sink {
    Sink { _p: () }
}

pub struct Sink {
    _p: (),
}

impl AsyncWrite for Sink {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }
}

pub fn repeat(byte: u8) -> Repeat {
    Repeat { byte }
}

pub struct Repeat {
    byte: u8,
}

impl AsyncRead for Repeat {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize, Error>> {
        for b in buf.iter_mut() {
            *b = self.byte;
        }
        Poll::Ready(Ok(buf.len()))
    }
}