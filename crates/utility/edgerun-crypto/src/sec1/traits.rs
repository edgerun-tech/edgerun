//! Traits for parsing objects from SEC1 encoded documents

use crate::sec1::Result;

#[cfg(feature = "elliptic_curve_alloc")]
use crate::der::SecretDocument;

#[cfg(feature = "elliptic_curve_pkcs8")]
use {
    crate::der::Decode,
    crate::sec1::{ALGORITHM_OID, EcPrivateKey},
};

#[cfg(feature = "elliptic_curve_std")]
use std::path::Path;

/// Parse an [`EcPrivateKey`] from a SEC1-encoded document.
pub trait DecodeEcPrivateKey: Sized {
    /// Deserialize SEC1 private key from ASN.1 DER-encoded data
    /// (binary format).
    fn from_sec1_der(bytes: &[u8]) -> Result<Self>;

    /// Load SEC1 private key from an ASN.1 DER-encoded file on the local
    /// filesystem (binary format).
    #[cfg(feature = "elliptic_curve_std")]
    fn read_sec1_der_file(path: impl AsRef<Path>) -> Result<Self> {
        Self::from_sec1_der(SecretDocument::read_der_file(path)?.as_bytes())
    }

}

/// Serialize a [`EcPrivateKey`] to a SEC1 encoded document.
#[cfg(feature = "elliptic_curve_alloc")]
pub trait EncodeEcPrivateKey {
    /// Serialize a [`SecretDocument`] containing a SEC1-encoded private key.
    fn to_sec1_der(&self) -> Result<SecretDocument>;

    /// Write ASN.1 DER-encoded SEC1 private key to the given path.
    #[cfg(feature = "elliptic_curve_std")]
    fn write_sec1_der_file(&self, path: impl AsRef<Path>) -> Result<()> {
        Ok(self.to_sec1_der()?.write_der_file(path)?)
    }

}

#[cfg(feature = "elliptic_curve_pkcs8")]
impl<T> DecodeEcPrivateKey for T
where
    T: for<'a> TryFrom<crate::pkcs8::PrivateKeyInfo<'a>, Error = crate::pkcs8::Error>,
{
    fn from_sec1_der(private_key: &[u8]) -> Result<Self> {
        let params_oid = EcPrivateKey::from_der(private_key)?
            .parameters
            .and_then(|params| params.named_curve());

        let algorithm = crate::pkcs8::AlgorithmIdentifierRef {
            oid: ALGORITHM_OID,
            parameters: params_oid.as_ref().map(Into::into),
        };

        Ok(Self::try_from(crate::pkcs8::PrivateKeyInfo {
            algorithm,
            private_key,
            public_key: None,
        })?)
    }
}

#[cfg(all(feature = "elliptic_curve_alloc", feature = "elliptic_curve_pkcs8"))]
impl<T: crate::pkcs8::EncodePrivateKey> EncodeEcPrivateKey for T {
    fn to_sec1_der(&self) -> Result<SecretDocument> {
        let doc = self.to_pkcs8_der()?;
        let pkcs8_key = crate::pkcs8::PrivateKeyInfo::from_der(doc.as_bytes())?;
        pkcs8_key.algorithm.assert_algorithm_oid(ALGORITHM_OID)?;

        let mut pkcs1_key = EcPrivateKey::from_der(pkcs8_key.private_key)?;
        pkcs1_key.parameters = Some(pkcs8_key.algorithm.parameters_oid()?.into());
        pkcs1_key.try_into()
    }
}
