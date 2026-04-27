//! Errors for the OCI registry client.

use crate::prelude::*;
use alloc::string::String;
use core::fmt;

#[derive(Debug)]
pub enum RegistryError {
    HttpStatus(u16),
    HttpError(String),
    AuthError(String),
    ManifestNotFound(String),
    NoManifests,
    DigestMismatch {
        expected: String,
        computed: String,
    },
    #[cfg(all(feature = "std", not(target_os = "none")))]
    IoError(std::io::Error),
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
            #[cfg(all(feature = "std", not(target_os = "none")))]
            RegistryError::IoError(e) => write!(f, "I/O error: {}", e),
            RegistryError::ParseError(e) => write!(f, "Parse error: {}", e),
        }
    }
}

impl core::error::Error for RegistryError {}

#[cfg(all(feature = "std", not(target_os = "none")))]
impl From<std::io::Error> for RegistryError {
    fn from(e: std::io::Error) -> Self {
        RegistryError::IoError(e)
    }
}

#[cfg(any(
    feature = "registry-client",
    all(feature = "std", not(target_os = "none"))
))]
impl From<edgerun_http::Error> for RegistryError {
    fn from(e: edgerun_http::Error) -> Self {
        RegistryError::HttpError(e.to_string())
    }
}
