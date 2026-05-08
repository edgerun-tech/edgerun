mod error;
mod params;
mod private_key;
mod public_key;
mod traits;
mod version;

pub use crate::der::{
    self,
    asn1::{ObjectIdentifier, UintRef},
};

pub use crate::pkcs1::{
    error::{Error, Result},
    params::{RsaOaepParams, RsaPssParams, TrailerField},
    private_key::RsaPrivateKey,
    public_key::RsaPublicKey,
    traits::{DecodeRsaPrivateKey, DecodeRsaPublicKey},
    version::Version,
};

#[cfg(feature = "rsa")]
pub use crate::pkcs1::{
    private_key::{other_prime_info::OtherPrimeInfo, OtherPrimeInfos},
    traits::{EncodeRsaPrivateKey, EncodeRsaPublicKey},
};

#[cfg(feature = "rsa_pem")]
pub use crate::der::pem::{self, LineEnding};

/// `rsaEncryption` Object Identifier (OID)
#[cfg(feature = "rsa")]
pub const ALGORITHM_OID: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.1");

/// `AlgorithmIdentifier` for RSA.
#[cfg(feature = "rsa")]
pub const ALGORITHM_ID: crate::pkcs8::AlgorithmIdentifierRef<'static> =
    crate::pkcs8::AlgorithmIdentifierRef {
        oid: ALGORITHM_OID,
        parameters: Some(crate::der::asn1::AnyRef::NULL),
    };
