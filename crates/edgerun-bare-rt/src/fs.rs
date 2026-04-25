//! Bare-metal filesystem - RAM disk or stub implementation.

#![no_std]

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};

static FS_DATA: AtomicUsize = AtomicUsize::new(0);

pub struct File {
    data: Vec<u8>,
    pos: usize,
}

impl File {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            pos: 0,
        }
    }

    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        let remaining = &self.data[self.pos..];
        let len = remaining.len().min(buf.len());
        buf[..len].copy_from_slice(&remaining[..len]);
        self.pos += len;
        len
    }

    pub fn write(&mut self, data: &[u8]) -> usize {
        self.data.extend_from_slice(data);
        data.len()
    }

    pub fn seek(&mut self, pos: usize) {
        self.pos = pos.min(self.data.len());
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
}

pub struct DirEntry {
    name: String,
    is_dir: bool,
    size: usize,
}

impl DirEntry {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn is_dir(&self) -> bool {
        self.is_dir
    }

    pub fn len(&self) -> usize {
        self.size
    }
}

pub struct Dir {
    entries: Vec<DirEntry>,
}

impl Dir {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn read_dir(&self, _path: &str) -> Result<Vec<DirEntry>, Error> {
        Ok(self.entries.clone())
    }
}

pub struct Error;

impl Error {
    pub fn not_found() -> Self {
        Self
    }
}