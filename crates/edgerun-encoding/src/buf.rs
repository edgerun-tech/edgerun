//! Buffer traits for efficient byte reading/writing.
//!
//! Provides `Buf` and `BufMut` traits compatible with the `bytes` crate API,
//! allowing zero-cost abstraction over different buffer types.

use alloc::vec::Vec;

#[derive(Debug)]
pub struct BufferError;

impl core::fmt::Display for BufferError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "buffer underflow")
    }
}

impl core::error::Error for BufferError {}

/// Trait for reading bytes from a buffer.
///
/// Compatible with `bytes::Buf`.
pub trait Buf {
    /// Returns the number of bytes between the current position and the end of the buffer.
    fn remaining(&self) -> usize;

    /// Returns a slice containing the bytes yet to be read.
    fn chunk(&self) -> &[u8];

    /// Advances the buffer position by `n` bytes.
    fn advance(&mut self, n: usize);

    /// Returns `true` if there are any bytes remaining in the buffer.
    fn has_remaining(&self) -> bool {
        self.remaining() > 0
    }

    /// Read a single byte.
    fn get_u8(&mut self) -> u8 {
        let b = self.chunk()[0];
        self.advance(1);
        b
    }

    /// Try to read a single byte, returning None if the buffer is empty.
    fn try_get_u8(&mut self) -> Option<u8> {
        if self.remaining() >= 1 {
            Some(self.get_u8())
        } else {
            None
        }
    }

    /// Read a single byte, returning an error if the buffer is empty.
    fn get_u8_result(&mut self) -> Result<u8, BufferError> {
        self.try_get_u8().ok_or(BufferError)
    }

    /// Copy bytes from the buffer without advancing the read position.
    fn copy_to_bytes(&mut self, len: usize) -> alloc::vec::Vec<u8> {
        let bytes = self.chunk()[..len].to_vec();
        self.advance(len);
        bytes
    }

    /// Read 2 bytes as big-endian u16.
    fn get_u16(&mut self) -> u16 {
        let bytes = self.chunk();
        let val = u16::from_be_bytes([bytes[0], bytes[1]]);
        self.advance(2);
        val
    }

    /// Read 2 bytes as little-endian u16.
    fn get_u16_le(&mut self) -> u16 {
        let bytes = self.chunk();
        let val = u16::from_le_bytes([bytes[0], bytes[1]]);
        self.advance(2);
        val
    }

    /// Read 4 bytes as big-endian u32.
    fn get_u32(&mut self) -> u32 {
        let bytes = self.chunk();
        let val = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        self.advance(4);
        val
    }

    /// Read 4 bytes as little-endian u32.
    fn get_u32_le(&mut self) -> u32 {
        let bytes = self.chunk();
        let val = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        self.advance(4);
        val
    }

    /// Read 8 bytes as big-endian u64.
    fn get_u64(&mut self) -> u64 {
        let bytes = self.chunk();
        let val = u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]);
        self.advance(8);
        val
    }

    /// Read 8 bytes as little-endian u64.
    fn get_u64_le(&mut self) -> u64 {
        let bytes = self.chunk();
        let val = u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]);
        self.advance(8);
        val
    }

    /// Read bytes into the given slice.
    fn get_bytes(&mut self, dst: &mut [u8]) {
        let src = &self.chunk()[..dst.len()];
        dst.copy_from_slice(src);
        self.advance(dst.len());
    }
}

/// Trait for writing bytes to a buffer.
///
/// Compatible with `bytes::BufMut`.
pub trait BufMut {
    /// Returns the number of bytes that can be written to the buffer.
    fn remaining(&self) -> usize;

    /// Returns a mutable slice containing the bytes yet to be written.
    fn chunk_mut(&mut self) -> &mut [u8];

    /// Advances the buffer position by `n` bytes.
    fn advance(&mut self, n: usize);

    /// Returns `true` if there are any bytes that can be written.
    fn has_remaining(&self) -> bool {
        self.remaining() > 0
    }

    /// Write a single byte.
    fn put_u8(&mut self, val: u8) {
        if self.remaining() >= 1 {
            let chunk = self.chunk_mut();
            if !chunk.is_empty() {
                chunk[0] = val;
                self.advance(1);
            }
        }
    }

    /// Write 2 bytes as big-endian u16.
    fn put_u16(&mut self, val: u16) {
        let chunk = self.chunk_mut();
        if chunk.len() >= 2 {
            chunk[..2].copy_from_slice(&val.to_be_bytes());
            self.advance(2);
        }
    }

    /// Write 2 bytes as little-endian u16.
    fn put_u16_le(&mut self, val: u16) {
        let chunk = self.chunk_mut();
        if chunk.len() >= 2 {
            chunk[..2].copy_from_slice(&val.to_le_bytes());
            self.advance(2);
        }
    }

    /// Write 4 bytes as big-endian u32.
    fn put_u32(&mut self, val: u32) {
        let chunk = self.chunk_mut();
        if chunk.len() >= 4 {
            chunk[..4].copy_from_slice(&val.to_be_bytes());
            self.advance(4);
        }
    }

    /// Write 4 bytes as little-endian u32.
    fn put_u32_le(&mut self, val: u32) {
        let chunk = self.chunk_mut();
        if chunk.len() >= 4 {
            chunk[..4].copy_from_slice(&val.to_le_bytes());
            self.advance(4);
        }
    }

    /// Write 8 bytes as big-endian u64.
    fn put_u64(&mut self, val: u64) {
        let chunk = self.chunk_mut();
        if chunk.len() >= 8 {
            chunk[..8].copy_from_slice(&val.to_be_bytes());
            self.advance(8);
        }
    }

    /// Write 8 bytes as little-endian u64.
    fn put_u64_le(&mut self, val: u64) {
        let chunk = self.chunk_mut();
        if chunk.len() >= 8 {
            chunk[..8].copy_from_slice(&val.to_le_bytes());
            self.advance(8);
        }
    }

    /// Write bytes from a slice.
    fn put(&mut self, src: &[u8]) {
        let chunk = self.chunk_mut();
        if chunk.len() >= src.len() {
            chunk[..src.len()].copy_from_slice(src);
            self.advance(src.len());
        }
    }

    /// Reserve additional capacity.
    fn reserve(&mut self, _additional: usize) {}
}

// ===========================================================================
// Slice implementations
// ===========================================================================

impl Buf for &[u8] {
    #[inline]
    fn remaining(&self) -> usize {
        self.len()
    }

    #[inline]
    fn chunk(&self) -> &[u8] {
        self
    }

    #[inline]
    fn advance(&mut self, n: usize) {
        *self = &self[n..];
    }
}

impl Buf for Vec<u8> {
    #[inline]
    fn remaining(&self) -> usize {
        self.len()
    }

    #[inline]
    fn chunk(&self) -> &[u8] {
        self
    }

    #[inline]
    fn advance(&mut self, n: usize) {
        let new_len = n.min(self.len());
        self.drain(..new_len);
    }
}

impl BufMut for &mut [u8] {
    #[inline]
    fn remaining(&self) -> usize {
        self.len()
    }

    #[inline]
    fn chunk_mut(&mut self) -> &mut [u8] {
        self
    }

    #[inline]
    fn advance(&mut self, n: usize) {
        let new_len = n.min(self.len());
        // SAFETY: we're advancing by valid bytes
        unsafe {
            let ptr = self.as_mut_ptr();
            *self = std::slice::from_raw_parts_mut(ptr.add(n), self.len() - n);
        }
    }
}

impl BufMut for Vec<u8> {
    #[inline]
    fn remaining(&self) -> usize {
        // Allow writing even to empty Vec - let it grow as needed
        // Use a large value that won't overflow
        1024 * 1024
    }

    #[inline]
    fn chunk_mut(&mut self) -> &mut [u8] {
        let len = self.len();
        let cap = self.capacity();
        if cap > len {
            unsafe { core::slice::from_raw_parts_mut(self.as_mut_ptr().add(len), cap - len) }
        } else {
            // Need to reserve more space
            self.reserve(64);
            let cap = self.capacity();
            unsafe { core::slice::from_raw_parts_mut(self.as_mut_ptr().add(len), cap - len) }
        }
    }

    #[inline]
    fn advance(&mut self, n: usize) {
        let new_len = self.len() + n;
        if new_len <= self.capacity() {
            unsafe {
                self.set_len(new_len);
            }
        }
    }
}

// ===========================================================================
// Cursor implementations
// ===========================================================================

use core::fmt;

/// A cursor for reading from a byte slice.
#[derive(Debug)]
pub struct Cursor<T> {
    buf: T,
    pos: usize,
}

impl<T: AsRef<[u8]>> Cursor<T> {
    pub fn new(buf: T) -> Self {
        Cursor { buf, pos: 0 }
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn set_position(&mut self, pos: usize) {
        self.pos = pos;
    }

    pub fn get_ref(&self) -> &T {
        &self.buf
    }

    pub fn get_mut(&mut self) -> &mut T {
        &mut self.buf
    }
}

impl<T: AsRef<[u8]>> Buf for Cursor<T> {
    #[inline]
    fn remaining(&self) -> usize {
        let buf = self.buf.as_ref();
        buf.len().saturating_sub(self.pos)
    }

    #[inline]
    fn chunk(&self) -> &[u8] {
        &self.buf.as_ref()[self.pos..]
    }

    #[inline]
    fn advance(&mut self, n: usize) {
        self.pos = (self.pos + n).min(self.buf.as_ref().len());
    }
}

impl Buf for std::io::Cursor<&mut Vec<u8>> {
    #[inline]
    fn remaining(&self) -> usize {
        let pos = self.position() as usize;
        let len = self.get_ref().len();
        len.saturating_sub(pos)
    }

    #[inline]
    fn chunk(&self) -> &[u8] {
        let pos = self.position() as usize;
        &self.get_ref()[pos..]
    }

    #[inline]
    fn advance(&mut self, n: usize) {
        use std::io::Read;
        let mut skip = [0u8; 1];
        for _ in 0..n {
            let _ = self.read(&mut skip);
        }
    }
}

impl Buf for std::io::Cursor<Vec<u8>> {
    #[inline]
    fn remaining(&self) -> usize {
        let pos = self.position() as usize;
        let len = self.get_ref().len();
        len.saturating_sub(pos)
    }

    #[inline]
    fn chunk(&self) -> &[u8] {
        let pos = self.position() as usize;
        &self.get_ref()[pos..]
    }

    #[inline]
    fn advance(&mut self, n: usize) {
        use std::io::Read;
        let mut skip = [0u8; 1];
        for _ in 0..n {
            let _ = self.read(&mut skip);
        }
    }
}

impl Buf for std::io::Cursor<&Vec<u8>> {
    #[inline]
    fn remaining(&self) -> usize {
        let pos = self.position() as usize;
        let len = self.get_ref().len();
        len.saturating_sub(pos)
    }

    #[inline]
    fn chunk(&self) -> &[u8] {
        let pos = self.position() as usize;
        &self.get_ref()[pos..]
    }

    #[inline]
    fn advance(&mut self, n: usize) {
        use std::io::Read;
        let mut skip = [0u8; 1];
        for _ in 0..n {
            let _ = self.read(&mut skip);
        }
    }
}

impl Buf for &mut std::io::Cursor<&mut Vec<u8>> {
    #[inline]
    fn remaining(&self) -> usize {
        let pos = self.position() as usize;
        let len = self.get_ref().len();
        len.saturating_sub(pos)
    }

    #[inline]
    fn chunk(&self) -> &[u8] {
        let pos = self.position() as usize;
        &self.get_ref()[pos..]
    }

    #[inline]
    fn advance(&mut self, n: usize) {
        use std::io::Read;
        let mut skip = [0u8; 1];
        for _ in 0..n {
            let _ = self.read(&mut skip);
        }
    }
}

impl Buf for &mut Vec<u8> {
    #[inline]
    fn remaining(&self) -> usize {
        self.len()
    }

    #[inline]
    fn chunk(&self) -> &[u8] {
        self
    }

    #[inline]
    fn advance(&mut self, n: usize) {
        let _ = self.drain(..n);
    }
}

impl<T: AsRef<[u8]> + fmt::Debug> fmt::Display for Cursor<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Cursor {{ pos: {}, buf: {:?} }}", self.pos, self.buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_get_u8() {
        let mut c = Cursor::new([1u8, 2, 3]);
        assert_eq!(c.get_u8(), 1);
        assert_eq!(c.get_u8(), 2);
        assert_eq!(c.remaining(), 1);
    }

    #[test]
    fn test_cursor_get_u16() {
        let mut c = Cursor::new([0x12, 0x34]);
        assert_eq!(c.get_u16(), 0x1234);
    }

    #[test]
    fn test_cursor_get_u32() {
        let mut c = Cursor::new([0x12, 0x34, 0x56, 0x78]);
        assert_eq!(c.get_u32(), 0x12345678);
    }

    #[test]
    fn test_slice_buf() {
        let data = [1u8, 2, 3, 4];
        let mut slice: &[u8] = &data;
        assert_eq!(slice.get_u8(), 1);
        assert_eq!(slice.get_u16(), 0x0203);
        assert_eq!(slice.remaining(), 1);
    }
}
