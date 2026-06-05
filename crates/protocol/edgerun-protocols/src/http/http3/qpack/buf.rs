use alloc::vec::Vec;
use core::fmt;

#[derive(Debug)]
pub struct BufferError;

pub trait Buf {
    fn remaining(&self) -> usize;
    fn chunk(&self) -> &[u8];
    fn advance(&mut self, n: usize);

    fn has_remaining(&self) -> bool {
        self.remaining() > 0
    }

    fn get_u8(&mut self) -> u8 {
        let b = self.chunk()[0];
        self.advance(1);
        b
    }

    fn get_u8_result(&mut self) -> Result<u8, BufferError> {
        if self.remaining() >= 1 {
            Ok(self.get_u8())
        } else {
            Err(BufferError)
        }
    }

    fn copy_to_bytes(&mut self, len: usize) -> Vec<u8> {
        let bytes = self.chunk()[..len].to_vec();
        self.advance(len);
        bytes
    }
}

pub trait BufMut {
    fn remaining(&self) -> usize;
    fn chunk_mut(&mut self) -> &mut [u8];
    fn advance(&mut self, n: usize);

    fn put_u8(&mut self, val: u8) {
        if self.remaining() >= 1 {
            let chunk = self.chunk_mut();
            if !chunk.is_empty() {
                chunk[0] = val;
                self.advance(1);
            }
        }
    }

    fn put(&mut self, src: &[u8]) {
        let chunk = self.chunk_mut();
        if chunk.len() >= src.len() {
            chunk[..src.len()].copy_from_slice(src);
            self.advance(src.len());
        }
    }
}

impl Buf for &[u8] {
    fn remaining(&self) -> usize {
        self.len()
    }

    fn chunk(&self) -> &[u8] {
        self
    }

    fn advance(&mut self, n: usize) {
        *self = &self[n..];
    }
}

impl BufMut for Vec<u8> {
    fn remaining(&self) -> usize {
        usize::MAX / 2
    }

    fn chunk_mut(&mut self) -> &mut [u8] {
        let len = self.len();
        if self.capacity() == len {
            self.reserve(64);
        }
        unsafe {
            core::slice::from_raw_parts_mut(self.as_mut_ptr().add(len), self.capacity() - len)
        }
    }

    fn advance(&mut self, n: usize) {
        let new_len = self.len() + n;
        if new_len <= self.capacity() {
            unsafe {
                self.set_len(new_len);
            }
        }
    }
}

#[derive(Debug)]
pub struct Cursor<T> {
    buf: T,
    pos: usize,
}

impl<T: AsRef<[u8]>> Cursor<T> {
    pub fn new(buf: T) -> Self {
        Self { buf, pos: 0 }
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn get_ref(&self) -> &T {
        &self.buf
    }
}

impl<T: AsRef<[u8]>> Buf for Cursor<T> {
    fn remaining(&self) -> usize {
        self.buf.as_ref().len().saturating_sub(self.pos)
    }

    fn chunk(&self) -> &[u8] {
        &self.buf.as_ref()[self.pos..]
    }

    fn advance(&mut self, n: usize) {
        self.pos = (self.pos + n).min(self.buf.as_ref().len());
    }
}

impl<T: AsRef<[u8]> + fmt::Debug> fmt::Display for Cursor<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Cursor {{ pos: {}, buf: {:?} }}", self.pos, self.buf)
    }
}
