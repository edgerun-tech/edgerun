//! Error type.

use core::fmt::{self, Display};

#[cfg(feature = "elliptic_curve_pkcs8")]
use crate::elliptic_curve::pkcs8;

/// Result type with the `elliptic-curve` crate's [`Error`] type.
pub type Result<T> = core::result::Result<T, Error>;

/// Elliptic curve errors.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Error;

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("crypto error")
    }
}

impl From<crate::base16ct::Error> for Error {
    fn from(_: crate::base16ct::Error) -> Error {
        Error
    }
}

#[cfg(feature = "elliptic_curve_pkcs8")]
impl From<crate::pkcs8::Error> for Error {
    fn from(_: crate::pkcs8::Error) -> Error {
        Error
    }
}

#[cfg(feature = "elliptic_curve_sec1")]
impl From<crate::sec1::Error> for Error {
    fn from(_: crate::sec1::Error) -> Error {
        Error
    }
}

#[cfg(feature = "elliptic_curve_std")]
impl std::error::Error for Error {}
