//! Error types

use core::fmt;

#[cfg(feature = "rsa_pem")]
use crate::der::pem;

/// Result type
pub type Result<T> = core::result::Result<T, Error>;

/// Error type
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Error {
    /// ASN.1 DER-related errors.
    Asn1(crate::der::Error),

    /// Cryptographic errors.
    ///
    /// These can be used by RSA implementations to signal that a key is
    /// invalid for cryptographic reasons. This means the document parsed
    /// correctly, but one of the values contained within was invalid, e.g.
    /// a number expected to be a prime was not a prime.
    Crypto,

    /// PKCS#8 errors.
    #[cfg(feature = "rsa")]
    Pkcs8(crate::pkcs8::Error),

    /// Version errors
    Version,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Asn1(err) => write!(f, "PKCS#1 ASN.1 error: {}", err),
            Error::Crypto => f.write_str("PKCS#1 cryptographic error"),
            #[cfg(feature = "rsa")]
            Error::Pkcs8(err) => write!(f, "{}", err),
            Error::Version => f.write_str("PKCS#1 version error"),
        }
    }
}

impl From<crate::der::Error> for Error {
    fn from(err: crate::der::Error) -> Error {
        Error::Asn1(err)
    }
}

#[cfg(feature = "rsa_pem")]
impl From<pem::Error> for Error {
    fn from(err: pem::Error) -> Error {
        crate::der::Error::from(err).into()
    }
}

#[cfg(feature = "rsa")]
impl From<Error> for crate::pkcs8::Error {
    fn from(err: Error) -> crate::pkcs8::Error {
        match err {
            Error::Asn1(e) => crate::pkcs8::Error::Asn1(e),
            Error::Crypto | Error::Version => crate::pkcs8::Error::KeyMalformed,
            Error::Pkcs8(e) => e,
        }
    }
}

#[cfg(feature = "rsa")]
impl From<crate::pkcs8::Error> for Error {
    fn from(err: crate::pkcs8::Error) -> Error {
        Error::Pkcs8(err)
    }
}

#[cfg(feature = "rsa")]
impl From<Error> for crate::pkcs8::spki::Error {
    fn from(err: Error) -> crate::pkcs8::spki::Error {
        match err {
            Error::Asn1(e) => crate::pkcs8::spki::Error::Asn1(e),
            _ => crate::pkcs8::spki::Error::KeyMalformed,
        }
    }
}

#[cfg(feature = "rsa")]
impl From<crate::pkcs8::spki::Error> for Error {
    fn from(err: crate::pkcs8::spki::Error) -> Error {
        Error::Pkcs8(crate::pkcs8::Error::PublicKey(err))
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}
