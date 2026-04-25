//! OAuth 2.0 / OIDC error types.

use std::fmt;

pub type OAuthResult<T> = Result<T, OAuthError>;

/// OAuth 2.0 error.
#[derive(Debug, Clone)]
pub enum OAuthError {
    /// OAuth server returned an error response.
    ServerError {
        error: String,
        error_description: String,
    },
    /// HTTP request failed.
    HttpError(String),
    /// JSON parse/serialize error.
    JsonError(String),
    /// Token expired and no refresh token available.
    TokenExpired,
    /// PKCE verification failed.
    PkceError(String),
    /// JWT / ID token error.
    JwtError(String),
    /// Device flow specific error.
    DeviceError(DeviceError),
    /// Authorization code flow error.
    AuthCodeError(String),
    /// IO error (file operations, network).
    IoError(String),
    /// Configuration error.
    ConfigError(String),
}

impl fmt::Display for OAuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OAuthError::ServerError {
                error,
                error_description,
            } => {
                write!(f, "OAuth server error: {error} — {error_description}")
            }
            OAuthError::HttpError(msg) => write!(f, "HTTP error: {msg}"),
            OAuthError::JsonError(msg) => write!(f, "JSON error: {msg}"),
            OAuthError::TokenExpired => write!(f, "Token expired and no refresh token available"),
            OAuthError::PkceError(msg) => write!(f, "PKCE error: {msg}"),
            OAuthError::JwtError(msg) => write!(f, "JWT error: {msg}"),
            OAuthError::DeviceError(e) => write!(f, "Device flow error: {e}"),
            OAuthError::AuthCodeError(msg) => write!(f, "Authorization code error: {msg}"),
            OAuthError::IoError(msg) => write!(f, "IO error: {msg}"),
            OAuthError::ConfigError(msg) => write!(f, "Config error: {msg}"),
        }
    }
}

impl std::error::Error for OAuthError {}

impl From<std::io::Error> for OAuthError {
    fn from(e: std::io::Error) -> Self {
        OAuthError::IoError(e.to_string())
    }
}

impl From<String> for OAuthError {
    fn from(msg: String) -> Self {
        OAuthError::JsonError(msg)
    }
}

/// Device flow specific errors.
#[derive(Debug, Clone)]
pub enum DeviceError {
    /// User has not yet authorized the device code.
    AuthorizationPending,
    /// Server asked us to slow down polling.
    SlowDown,
    /// Device code expired.
    Expired,
    /// Device code was denied.
    Denied,
    /// Polling timed out.
    Timeout,
    /// Invalid server response.
    InvalidResponse(String),
}

impl fmt::Display for DeviceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceError::AuthorizationPending => write!(f, "authorization pending"),
            DeviceError::SlowDown => write!(f, "slow down"),
            DeviceError::Expired => write!(f, "device code expired"),
            DeviceError::Denied => write!(f, "device code denied"),
            DeviceError::Timeout => write!(f, "device flow timed out"),
            DeviceError::InvalidResponse(msg) => write!(f, "invalid response: {msg}"),
        }
    }
}

impl std::error::Error for DeviceError {}
