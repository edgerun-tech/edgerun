//! Lightweight `edgerun-bytes` implementation with no external buffer dependency.

#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use core::fmt;
use core::iter::FromIterator;
use core::ops::{Bound, Deref, DerefMut, RangeBounds};
use core::mem;

#[derive(Clone, Default, Eq, PartialEq)]
pub struct Bytes(Vec<u8>);

impl Bytes {
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    pub fn from_static(bytes: &'static [u8]) -> Self {
        Self(bytes.to_vec())
    }

    pub fn copy_from_slice(bytes: &[u8]) -> Self {
        Self(bytes.to_vec())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.0.clone()
    }

    pub fn slice<R: RangeBounds<usize>>(&self, range: R) -> Self {
        let len = self.0.len();
        let start = match range.start_bound() {
            Bound::Included(&value) => value,
            Bound::Excluded(&value) => value.saturating_add(1),
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(&value) => value.saturating_add(1),
            Bound::Excluded(&value) => value,
            Bound::Unbounded => len,
        };
        Self(self.0[start..end].to_vec())
    }
}

impl AsRef<[u8]> for Bytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Deref for Bytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.0.as_slice()
    }
}

impl From<Vec<u8>> for Bytes {
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

impl From<&[u8]> for Bytes {
    fn from(bytes: &[u8]) -> Self {
        Self(bytes.to_vec())
    }
}

impl From<&str> for Bytes {
    fn from(value: &str) -> Self {
        Self(value.as_bytes().to_vec())
    }
}

impl From<String> for Bytes {
    fn from(value: String) -> Self {
        Self(value.into_bytes())
    }
}

impl From<alloc::boxed::Box<[u8]>> for Bytes {
    fn from(bytes: alloc::boxed::Box<[u8]>) -> Self {
        Self(bytes.into_vec())
    }
}

impl From<Bytes> for Vec<u8> {
    fn from(bytes: Bytes) -> Self {
        bytes.0
    }
}

impl FromIterator<u8> for Bytes {
    fn from_iter<I: IntoIterator<Item = u8>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl fmt::Debug for Bytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bytes({:?})", &self.0)
    }
}

impl PartialEq<[u8]> for Bytes {
    fn eq(&self, other: &[u8]) -> bool {
        self.0.as_slice() == other
    }
}

impl PartialEq<&[u8]> for Bytes {
    fn eq(&self, other: &&[u8]) -> bool {
        self.0.as_slice() == *other
    }
}

#[derive(Clone, Default, Eq, PartialEq, Debug)]
pub struct BytesMut(Vec<u8>);

impl BytesMut {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    pub fn freeze(self) -> Bytes {
        self.into()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional)
    }

    pub fn resize(&mut self, new_len: usize, value: u8) {
        self.0.resize(new_len, value)
    }

    pub fn truncate(&mut self, len: usize) {
        self.0.truncate(len)
    }

    pub fn split_to(&mut self, at: usize) -> Bytes {
        let tail = self.0.split_off(at);
        Bytes(mem::replace(&mut self.0, tail))
    }

    pub fn extend_from_slice(&mut self, bytes: &[u8]) {
        self.0.extend_from_slice(bytes)
    }

    pub fn zeroed(len: usize) -> Self {
        Self(vec![0u8; len])
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.0.clone()
    }

    pub fn advance(&mut self, cnt: usize) {
        let start = mem::take(&mut self.0);
        let remain = &start[cnt..];
        self.0 = remain.to_vec();
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl AsRef<[u8]> for BytesMut {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsMut<[u8]> for BytesMut {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl Deref for BytesMut {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.0.as_slice()
    }
}

impl DerefMut for BytesMut {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Extend<u8> for BytesMut {
    fn extend<T: IntoIterator<Item = u8>>(&mut self, iter: T) {
        self.0.extend(iter)
    }
}

impl From<Vec<u8>> for BytesMut {
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

impl From<&[u8]> for BytesMut {
    fn from(bytes: &[u8]) -> Self {
        Self(bytes.to_vec())
    }
}

impl From<&str> for BytesMut {
    fn from(value: &str) -> Self {
        Self::from(value.as_bytes())
    }
}

impl From<String> for BytesMut {
    fn from(value: String) -> Self {
        Self(value.into_bytes())
    }
}

impl From<Bytes> for BytesMut {
    fn from(value: Bytes) -> Self {
        Self::from(value.0)
    }
}

impl From<BytesMut> for Bytes {
    fn from(value: BytesMut) -> Self {
        Self(value.0)
    }
}

impl From<BytesMut> for Vec<u8> {
    fn from(value: BytesMut) -> Self {
        value.0
    }
}

impl FromIterator<u8> for BytesMut {
    fn from_iter<I: IntoIterator<Item = u8>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}
