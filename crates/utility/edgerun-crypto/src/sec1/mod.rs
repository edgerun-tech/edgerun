//! ## `edgerun_json_compat` support
//!
//! When the `edgerun_json_compat` feature of this crate is enabled, the [`EncodedPoint`]
//! type receives impls of [`edgerun_json_compat::Serialize`] and [`edgerun_json_compat::Deserialize`].
//!
//! Additionally, when both the `alloc` and `edgerun_json_compat` features are enabled, the
//! serializers/deserializers will autodetect if a "human friendly" textual
//! encoding is being used, and if so encode the points as hexadecimal.

pub mod point;

mod error;
mod parameters;
mod private_key;
mod traits;

pub use crate::der;

pub use crate::sec1::error::{Error, Result};

pub use crate::sec1::point::EncodedPoint;

pub use generic_array::typenum::consts;

pub use crate::sec1::{
    parameters::EcParameters, private_key::EcPrivateKey, traits::DecodeEcPrivateKey,
};

#[cfg(feature = "elliptic_curve_alloc")]
pub use crate::sec1::traits::EncodeEcPrivateKey;

#[cfg(feature = "elliptic_curve_pkcs8")]
pub use crate::pkcs8;

#[cfg(feature = "elliptic_curve_pkcs8")]
use crate::pkcs8::ObjectIdentifier;

/// Algorithm [`ObjectIdentifier`] for elliptic curve public key cryptography
/// (`id-ecPublicKey`).
///
/// <http://oid-info.com/get/1.2.840.10045.2.1>
#[cfg(feature = "elliptic_curve_pkcs8")]
pub const ALGORITHM_OID: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.10045.2.1");
