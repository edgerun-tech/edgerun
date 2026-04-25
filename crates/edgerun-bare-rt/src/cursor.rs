//! Async `Cursor` — in-memory reader/writer over `&[u8]` and `Vec<u8>` (no_std).


extern crate alloc;

use alloc::vec::Vec;
use alloc::boxed::Box;
use core::pin::Pin;
use core::task::{Context, Poll};

use crate::io_traits::{AsyncRead, AsyncWrite, Error};

pub struct Cursor<T> {
    inner: T,
    pos: u64,
}

impl<T: AsRef<[u8]>> Cursor<T> {
    pub fn new(inner: T) -> Self {
        Self { inner, pos: 0 }
    }

    pub fn position(&self) -> u64 {
        self.pos
    }

    pub fn set_position(&mut self, pos: u64) {
        self.pos = pos;
    }

    pub fn get_ref(&self) -> &T {
        &self.inner
    }

    pub fn get_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    pub fn into_inner(self) -> T {
        self.inner
    }

    fn len(&self) -> u64 {
        self.inner.as_ref().len() as u64
    }

    pub fn seek(&mut self, offset: i64, from: SeekFrom) -> Result<u64, Error> {
        let len = self.len() as i64;
        let new_pos = match from {
            SeekFrom::Start(n) => n as i64,
            SeekFrom::End(n) => len + n,
            SeekFrom::Current(n) => self.pos as i64 + n,
        };
        if new_pos < 0 {
            Err(Error::new(crate::io_traits::ErrorKind::InvalidData))
        } else {
            self.pos = new_pos as u64;
            Ok(self.pos)
        }
    }
}

impl<T: AsRef<[u8]> + Unpin> AsyncRead for Cursor<T> {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize, Error>> {
        let this = unsafe { self.get_unchecked_mut() };
        let slice = this.inner.as_ref();
        let start = this.pos as usize;
        if start >= slice.len() {
            return Poll::Ready(Ok(0));
        }
        let available = &slice[start..];
        let n = core::cmp::min(buf.len(), available.len());
        buf[..n].copy_from_slice(&available[..n]);
        this.pos += n as u64;
        Poll::Ready(Ok(n))
    }
}

impl AsyncWrite for Cursor<Vec<u8>> {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        let this = unsafe { self.get_unchecked_mut() };
        let pos = this.pos as usize;

        if pos >= this.inner.len() {
            this.inner.resize(pos, 0);
        }

        let end = pos + buf.len();
        if end > this.inner.len() {
            this.inner.resize(end, 0);
        }

        this.inner[pos..pos + buf.len()].copy_from_slice(buf);
        this.pos += buf.len() as u64;
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }
}

#[derive(Clone, Copy)]
pub enum SeekFrom {
    Start(u64),
    End(i64),
    Current(i64),
}