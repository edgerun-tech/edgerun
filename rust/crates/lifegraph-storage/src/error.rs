//! Storage errors.

/// Unified storage error type.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("encode error: {0}")]
    Encode(String),
    #[error("decode error: {0}")]
    Decode(String),
    #[error("encryption error: {0}")]
    Encryption(String),
    #[error("decryption error: {0}")]
    Decryption(String),
}
