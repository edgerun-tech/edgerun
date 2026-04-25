//! Buffered async I/O — `BufReader` and `BufWriter` (no_std).

#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::String;
use core::pin::Pin;
use core::task::{Context, Poll};

use crate::io_traits::{AsyncRead, AsyncWrite, Error};

impl<R: AsyncRead + Unpin> AsyncRead for BufReader<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize, Error>> {
        let this = unsafe { self.get_unchecked_mut() };

        if this.pos > 0 {
            let n = core::cmp::min(buf.len(), this.pos);
            buf[..n].copy_from_slice(&this.buf[..n]);
            this.pos -= n;
            if this.pos > 0 {
                this.buf.copy_within(n..n + this.pos, 0);
            }
            return Poll::Ready(Ok(n));
        }

        let cap = this.buf.len();
        match Pin::new(&mut this.inner).poll_read(cx, &mut this.buf[..cap]) {
            Poll::Ready(Ok(0)) => Poll::Ready(Ok(0)),
            Poll::Ready(Ok(n)) => {
                this.pos = n;
                let n2 = core::cmp::min(buf.len(), n);
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

impl<W: AsyncWrite + Unpin> AsyncWrite for BufWriter<W> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        let this = unsafe { self.get_unchecked_mut() };

        if this.buf.len() + buf.len() > this.capacity && !this.buf.is_empty() {
            match Pin::new(&mut this.inner).poll_write(cx, &this.buf) {
                Poll::Ready(Ok(n)) => {
                    this.buf.drain(..n);
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }

        if buf.len() >= this.capacity && this.buf.is_empty() {
            return Pin::new(&mut this.inner).poll_write(cx, buf);
        }

        let available = this.capacity - this.buf.len();
        let n = core::cmp::min(buf.len(), available);
        this.buf.extend_from_slice(&buf[..n]);
        Poll::Ready(Ok(n))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
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

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
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
            buf: Vec::with_capacity(cap),
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
        let amt = core::cmp::min(amt, self.pos);
        self.pos -= amt;
        if self.pos > 0 {
            self.buf.copy_within(amt..amt + self.pos, 0);
        }
    }

    fn fill_buf_poll(&mut self, cx: &mut Context<'_>) -> Poll<Result<&[u8], Error>> {
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

    pub fn read_line(&mut self) -> ReadLineFut<'_, R> {
        ReadLineFut {
            reader: self,
            max_size: usize::MAX,
            line: Vec::new(),
        }
    }

    pub fn read_line_max(&mut self, max_size: usize) -> ReadLineFut<'_, R> {
        ReadLineFut {
            reader: self,
            max_size,
            line: Vec::new(),
        }
    }
}

use alloc::string::String;
use core::future::Future;

pub struct ReadLineFut<'a, R> {
    reader: &'a mut BufReader<R>,
    max_size: usize,
    line: Vec<u8>,
}

impl<R: AsyncRead + Unpin> Future for ReadLineFut<'_, R> {
    type Output = Result<Option<String>, Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let max_size = self.max_size;

        loop {
            let line_len = self.line.len();

            match self.reader.fill_buf_poll(cx) {
                Poll::Ready(Ok(available)) => {
                    if available.is_empty() {
                        return Poll::Ready(if line_len == 0 {
                            Ok(None)
                        } else {
                            let line = core::mem::take(&mut self.line);
                            Ok(Some(
                                String::from_utf8(line)
                                    .map_err(|_| Error::new(crate::io_traits::ErrorKind::InvalidData))?,
                            ))
                        });
                    }

                    if let Some(newline_pos) = available.iter().position(|&b| b == b'\n') {
                        let consumed = newline_pos + 1;
                        let has_cr = newline_pos > 0 && available[newline_pos - 1] == b'\r';
                        let line_end = if has_cr { newline_pos - 1 } else { newline_pos };
                        let data: Vec<u8> = available[..line_end].to_vec();
                        self.reader.consume(consumed);
                        self.line.extend_from_slice(&data);
                        let line = core::mem::take(&mut self.line);
                        return Poll::Ready(Ok(Some(
                            String::from_utf8(line)
                                .map_err(|_| Error::new(crate::io_traits::ErrorKind::InvalidData))?,
                        )));
                    }

                    let avail_len = available.len();
                    if line_len + avail_len > max_size {
                        return Poll::Ready(Err(Error::new(crate::io_traits::ErrorKind::InvalidData)));
                    }

                    let data: Vec<u8> = available.to_vec();
                    self.reader.consume(avail_len);
                    self.line.extend_from_slice(&data);
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

pub struct BufWriter<W> {
    inner: W,
    buf: Vec<u8>,
    capacity: usize,
}

impl<W: AsyncWrite + Unpin> BufWriter<W> {
    pub fn new(inner: W) -> Self {
        Self::with_capacity(8192, inner)
    }

    pub fn with_capacity(cap: usize, inner: W) -> Self {
        Self {
            inner,
            buf: Vec::with_capacity(cap),
            capacity: cap,
        }
    }

    pub fn get_ref(&self) -> &W {
        &self.inner
    }

    pub fn get_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    pub fn into_inner_without_flush(self) -> W {
        self.inner
    }

    pub fn buffered(&self) -> usize {
        self.buf.len()
    }
}