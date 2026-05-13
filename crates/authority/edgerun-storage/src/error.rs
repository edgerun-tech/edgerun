//! THE EVENT LOG IS THE STATE.
//!
//! Storage errors.

use crate::prelude::v1::*;
use std::fmt;

#[derive(Debug)]
pub enum StorageError {
    Io(std::io::Error),
    Encode(String),
    Decode(String),
    Encryption(String),
    Decryption(String),
    InvalidBlob(String),
    InvalidArgument(String),
    Stream(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Encode(e) => write!(f, "encode error: {e}"),
            Self::Decode(e) => write!(f, "decode error: {e}"),
            Self::Encryption(e) => write!(f, "encryption error: {e}"),
            Self::Decryption(e) => write!(f, "decryption error: {e}"),
            Self::InvalidBlob(e) => write!(f, "invalid blob: {e}"),
            Self::InvalidArgument(e) => write!(f, "invalid argument: {e}"),
            Self::Stream(e) => write!(f, "stream error: {e}"),
        }
    }
}

impl core::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for StorageError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<crate::stream::StreamError> for StorageError {
    fn from(e: crate::stream::StreamError) -> Self {
        Self::Stream(e.to_string())
    }
}
