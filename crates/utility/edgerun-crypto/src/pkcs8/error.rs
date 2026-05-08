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

    /// Errors relating to PKCS#5-encrypted keys.
    #[cfg(feature = "pkcs5")]
    EncryptedPrivateKey(pkcs5::Error),

    /// Malformed cryptographic key contained in a PKCS#8 document.
    ///
    /// This is intended for relaying errors related to the raw data contained
    /// within [`PrivateKeyInfo::private_key`][`crate::pkcs8::PrivateKeyInfo::private_key`]
    /// or [`SubjectPublicKeyInfo::subject_public_key`][`crate::pkcs8::SubjectPublicKeyInfo::subject_public_key`].
    KeyMalformed,

    /// [`AlgorithmIdentifier::parameters`][`crate::pkcs8::AlgorithmIdentifierRef::parameters`]
    /// is malformed or otherwise encoded in an unexpected manner.
    ParametersMalformed,

    /// Public key errors propagated from the [`crate::spki::Error`] type.
    PublicKey(crate::spki::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Asn1(err) => write!(f, "PKCS#8 ASN.1 error: {}", err),
            #[cfg(feature = "pkcs5")]
            Error::EncryptedPrivateKey(err) => write!(f, "{}", err),
            Error::KeyMalformed => f.write_str("PKCS#8 cryptographic key data malformed"),
            Error::ParametersMalformed => f.write_str("PKCS#8 algorithm parameters malformed"),
            Error::PublicKey(err) => write!(f, "public key error: {}", err),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

impl From<crate::der::Error> for Error {
    fn from(err: crate::der::Error) -> Error {
        Error::Asn1(err)
    }
}

impl From<crate::der::ErrorKind> for Error {
    fn from(err: crate::der::ErrorKind) -> Error {
        Error::Asn1(err.into())
    }
}

#[cfg(feature = "rsa_pem")]
impl From<pem::Error> for Error {
    fn from(err: pem::Error) -> Error {
        crate::der::Error::from(err).into()
    }
}

#[cfg(feature = "pkcs5")]
impl From<pkcs5::Error> for Error {
    fn from(err: pkcs5::Error) -> Error {
        Error::EncryptedPrivateKey(err)
    }
}

impl From<crate::spki::Error> for Error {
    fn from(err: crate::spki::Error) -> Error {
        Error::PublicKey(err)
    }
}

impl From<Error> for crate::spki::Error {
    fn from(err: Error) -> crate::spki::Error {
        match err {
            Error::Asn1(e) => crate::spki::Error::Asn1(e),
            Error::PublicKey(e) => e,
            _ => crate::spki::Error::KeyMalformed,
        }
    }
}
