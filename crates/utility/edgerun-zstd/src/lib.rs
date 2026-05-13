//! EdgeRun Zstandard boundary.
//!
//! This crate intentionally does not wrap the external `zstd` crate. Until an
//! owned codec lands here, callers must treat Zstd compression as unavailable.

use std::io;
use std::io::Read;

pub mod stream {
    use super::*;

    pub fn encode_all<R: Read>(_reader: R, _level: i32) -> io::Result<Vec<u8>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "edgerun-zstd has no owned encoder yet",
        ))
    }

    pub fn decode_all<R: Read>(_reader: R) -> io::Result<Vec<u8>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "edgerun-zstd has no owned decoder yet",
        ))
    }
}
