//! Storage errors.

use std::fmt;

#[derive(Debug)]
pub enum StorageError {
    Io(std::io::Error),
    Encode(String),
    Decode(String),
    Encryption(String),
    Decryption(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Encode(e) => write!(f, "encode error: {e}"),
            Self::Decode(e) => write!(f, "decode error: {e}"),
            Self::Encryption(e) => write!(f, "encryption error: {e}"),
            Self::Decryption(e) => write!(f, "decryption error: {e}"),
        }
    }
}

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for StorageError {
    fn from(e: std::io::Error) -> Self { Self::Io(e) }
}
