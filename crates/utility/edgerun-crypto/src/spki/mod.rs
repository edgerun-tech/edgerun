//! # Usage
//! The following example demonstrates how to use an OID as the `parameters`
//! of an [`AlgorithmIdentifier`].
//!
//! Borrow the [`ObjectIdentifier`] first then use [`crate::der::AnyRef::from`] or `.into()`:
//!
//! ```ignore
//! use edgerun_crypto::spki::{AlgorithmIdentifier, ObjectIdentifier};
//!
//! let alg_oid = "1.2.840.10045.2.1".parse::<ObjectIdentifier>().unwrap();
//! let params_oid = "1.2.840.10045.3.1.7".parse::<ObjectIdentifier>().unwrap();
//!
//! let alg_id = AlgorithmIdentifier {
//!     oid: alg_oid,
//!     parameters: Some(params_oid)
//! };
//! ```

mod algorithm;
mod error;
mod spki;
mod traits;

pub use crate::spki::{
    algorithm::{AlgorithmIdentifier, AlgorithmIdentifierRef, AlgorithmIdentifierWithOid},
    error::{Error, Result},
    spki::{SubjectPublicKeyInfo, SubjectPublicKeyInfoRef},
    traits::{AssociatedAlgorithmIdentifier, DecodePublicKey, SignatureAlgorithmIdentifier},
};
pub use crate::der::{self, asn1::ObjectIdentifier};

pub use {
    crate::spki::{
        algorithm::AlgorithmIdentifierOwned,
        spki::SubjectPublicKeyInfoOwned,
        traits::{
            DynAssociatedAlgorithmIdentifier, DynSignatureAlgorithmIdentifier, EncodePublicKey,
            SignatureBitStringEncoding,
        },
    }, der::Document,
};
