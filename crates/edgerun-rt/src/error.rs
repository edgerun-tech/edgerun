//! Error types for bare-metal runtime

macro_rules! impl_error {
    ($ty:ty, |$this:ident, $f:ident| $body:block) => {
        impl core::fmt::Display for $ty {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                let $this = self;
                let $f = f;
                $body
            }
        }

        impl core::error::Error for $ty {}
    };
}

pub(crate) use impl_error;

#[derive(Debug)]
pub enum Error {
    Cancelled,
    Closed,
    InvalidInput,
    NotReady,
    Timeout,
    TooMany,
}

impl_error!(Error, |this, f| {
    match this {
        Error::Cancelled => write!(f, "operation cancelled"),
        Error::Closed => write!(f, "channel closed"),
        Error::InvalidInput => write!(f, "invalid input"),
        Error::NotReady => write!(f, "not ready"),
        Error::Timeout => write!(f, "timeout"),
        Error::TooMany => write!(f, "too many"),
    }
});
