//! Lightweight `edgerun-bytes` implementation with no external buffer dependency.

#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use core::fmt;
use core::ops::Deref;

#[derive(Clone, Default, Eq, PartialEq)]
pub struct Bytes(Vec<u8>);

impl Bytes {
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    pub fn from_static(bytes: &'static [u8]) -> Self {
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
