//! Error types for bare-metal runtime

#[derive(Debug)]
pub enum Error {
    Cancelled,
    Closed,
    InvalidInput,
    NotReady,
    Timeout,
    TooMany,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Cancelled => write!(f, "operation cancelled"),
            Error::Closed => write!(f, "channel closed"),
            Error::InvalidInput => write!(f, "invalid input"),
            Error::NotReady => write!(f, "not ready"),
            Error::Timeout => write!(f, "timeout"),
            Error::TooMany => write!(f, "too many"),
        }
    }
}