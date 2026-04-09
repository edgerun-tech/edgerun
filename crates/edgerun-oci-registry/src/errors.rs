//! Errors for the OCI registry client.

use std::fmt;
use std::io;

#[derive(Debug)]
pub enum RegistryError {
    HttpStatus(u16),
    HttpError(String),
    AuthError(String),
    ManifestNotFound(String),
    NoManifests,
    DigestMismatch { expected: String, computed: String },
    IoError(io::Error),
    ParseError(String),
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegistryError::HttpStatus(code) => write!(f, "HTTP {}", code),
            RegistryError::HttpError(e) => write!(f, "HTTP error: {}", e),
            RegistryError::AuthError(e) => write!(f, "Auth error: {}", e),
            RegistryError::ManifestNotFound(tag) => write!(f, "Manifest not found: {}", tag),
            RegistryError::NoManifests => write!(f, "No manifests in index"),
            RegistryError::DigestMismatch { expected, computed } => {
                write!(
                    f,
                    "Digest mismatch:\n  expected: {}\n  computed: {}",
                    expected, computed
                )
            }
            RegistryError::IoError(e) => write!(f, "I/O error: {}", e),
            RegistryError::ParseError(e) => write!(f, "Parse error: {}", e),
        }
    }
}

impl std::error::Error for RegistryError {}

impl From<io::Error> for RegistryError {
    fn from(e: io::Error) -> Self {
        RegistryError::IoError(e)
    }
}
