//! PKCS#8 encoding/decoding support.

use super::SecretKey;
use crate::elliptic_curve::{
    sec1::{ModulusSize, ValidatePublicKey},
    Curve, FieldBytesSize, ALGORITHM_OID,
};
use crate::pkcs8::spki::{AlgorithmIdentifier, AssociatedAlgorithmIdentifier, ObjectIdentifier};
use crate::pkcs8::{self, der::Decode, AssociatedOid};
use crate::sec1::EcPrivateKey;

// Imports for the `EncodePrivateKey` impl
#[cfg(all(
    feature = "elliptic_curve_alloc",
    feature = "elliptic_curve_arithmetic"
))]
use {
    crate::elliptic_curve::{
        sec1::{FromEncodedPoint, ToEncodedPoint},
        AffinePoint, CurveArithmetic,
    },
    crate::pkcs8::{der, EncodePrivateKey},
};

// Imports for actual PEM support
#[cfg(feature = "elliptic_curve_pem")]
use {
    crate::elliptic_curve::{error::Error, Result},
    crate::pkcs8::DecodePrivateKey,
    core::str::FromStr,
};

impl<C> AssociatedAlgorithmIdentifier for SecretKey<C>
where
    C: AssociatedOid + Curve,
{
    type Params = ObjectIdentifier;

    const ALGORITHM_IDENTIFIER: AlgorithmIdentifier<ObjectIdentifier> = AlgorithmIdentifier {
        oid: ALGORITHM_OID,
        parameters: Some(C::OID),
    };
}

impl<C> TryFrom<crate::pkcs8::PrivateKeyInfo<'_>> for SecretKey<C>
where
    C: AssociatedOid + Curve + ValidatePublicKey,
    FieldBytesSize<C>: ModulusSize,
{
    type Error = crate::pkcs8::Error;

    fn try_from(private_key_info: crate::pkcs8::PrivateKeyInfo<'_>) -> crate::pkcs8::Result<Self> {
        private_key_info
            .algorithm
            .assert_oids(ALGORITHM_OID, C::OID)?;

        let ec_private_key = EcPrivateKey::from_der(private_key_info.private_key)?;
        Ok(Self::try_from(ec_private_key)?)
    }
}

#[cfg(all(
    feature = "elliptic_curve_alloc",
    feature = "elliptic_curve_arithmetic"
))]
impl<C> EncodePrivateKey for SecretKey<C>
where
    C: AssociatedOid + CurveArithmetic,
    AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C>,
    FieldBytesSize<C>: ModulusSize,
{
    fn to_pkcs8_der(&self) -> crate::pkcs8::Result<crate::der::SecretDocument> {
        // TODO(tarcieri): make `PrivateKeyInfo` generic around `Params`
        let algorithm_identifier = crate::pkcs8::AlgorithmIdentifierRef {
            oid: ALGORITHM_OID,
            parameters: Some((&C::OID).into()),
        };

        let ec_private_key = self.to_sec1_der()?;
        let pkcs8_key = crate::pkcs8::PrivateKeyInfo::new(algorithm_identifier, &ec_private_key);
        Ok(crate::der::SecretDocument::encode_msg(&pkcs8_key)?)
    }
}

#[cfg(feature = "elliptic_curve_pem")]
impl<C> FromStr for SecretKey<C>
where
    C: Curve + AssociatedOid + ValidatePublicKey,
    FieldBytesSize<C>: ModulusSize,
{
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Self::from_pkcs8_pem(s).map_err(|_| Error)
    }
}
