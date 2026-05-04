//! Wallet-specific error types.

extern crate alloc;

use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalletError {
    /// Invalid decimal string (contains exponent, negative, NaN, etc.)
    InvalidDecimal(alloc::string::String),
    /// Arithmetic overflow
    Overflow,
    /// Negative result from subtraction
    NegativeResult,
    /// Invalid asset ID format
    InvalidAssetId(alloc::string::String),
    /// Invalid status transition
    InvalidStatusTransition {
        from: &'static str,
        to: &'static str,
    },
    /// Terminal state cannot transition
    TerminalState(&'static str),
    /// Provider error (internal, never exposed to public API)
    ProviderError(alloc::string::String),
    /// Serialization/deserialization error
    Serialization(alloc::string::String),
    /// HTTP client error
    HttpError(alloc::string::String),
    /// Event log error
    EventError(alloc::string::String),
    /// Not found
    NotFound(alloc::string::String),
    /// Configuration error
    Config(alloc::string::String),
}

impl fmt::Display for WalletError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WalletError::InvalidDecimal(s) => write!(f, "invalid decimal: {}", s),
            WalletError::Overflow => write!(f, "arithmetic overflow"),
            WalletError::NegativeResult => write!(f, "negative result from subtraction"),
            WalletError::InvalidAssetId(s) => write!(f, "invalid asset id: {}", s),
            WalletError::InvalidStatusTransition { from, to } => {
                write!(f, "invalid status transition: {} -> {}", from, to)
            }
            WalletError::TerminalState(s) => write!(f, "terminal state cannot transition: {}", s),
            WalletError::ProviderError(s) => write!(f, "provider error: {}", s),
            WalletError::Serialization(s) => write!(f, "serialization error: {}", s),
            WalletError::HttpError(s) => write!(f, "HTTP error: {}", s),
            WalletError::EventError(s) => write!(f, "event error: {}", s),
            WalletError::NotFound(s) => write!(f, "not found: {}", s),
            WalletError::Config(s) => write!(f, "config error: {}", s),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for WalletError {}
