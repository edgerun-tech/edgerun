#![allow(missing_docs)]


pub mod asn1;
pub mod referenced;

pub(crate) mod arrayvec;
mod bytes_ref;
mod datetime;
mod decode;
mod encode;
mod encode_ref;
mod error;
mod header;
mod length;
mod ord;
mod reader;
mod str_ref;
mod tag;
mod writer;

#[cfg(feature = "alloc")]
mod bytes_owned;
#[cfg(feature = "alloc")]
mod document;
#[cfg(feature = "alloc")]
mod str_owned;

pub use crate::der::{
    asn1::{AnyRef, Choice, Sequence},
    datetime::DateTime,
    decode::{Decode, DecodeOwned, DecodeValue},
    encode::{Encode, EncodeValue},
    encode_ref::{EncodeRef, EncodeValueRef},
    error::{Error, ErrorKind, Result},
    header::Header,
    length::{IndefiniteLength, Length},
    ord::{DerOrd, ValueOrd},
    reader::{nested::NestedReader, slice::SliceReader, Reader},
    tag::{Class, FixedTag, Tag, TagMode, TagNumber, Tagged},
    writer::{slice::SliceWriter, Writer},
};

#[cfg(feature = "alloc")]
pub use crate::der::{asn1::Any, document::Document};

#[cfg(feature = "bigint")]
pub use crypto_bigint as bigint;

#[cfg(feature = "derive")]
pub use der_derive::{Choice, Enumerated, Sequence, ValueOrd};

#[cfg(feature = "flagset")]
pub use flagset;

#[cfg(feature = "oid")]
pub use crate::const_oid as oid;

#[cfg(feature = "pem")]
pub use {
    crate::der::{decode::DecodePem, encode::EncodePem, reader::pem::PemReader, writer::pem::PemWriter},
    crate::pem_rfc7468 as pem,
};

#[cfg(feature = "time")]
pub use time;

#[cfg(feature = "zeroize")]
pub use crate::zeroize;

#[cfg(all(feature = "alloc", feature = "zeroize"))]
pub use crate::der::document::SecretDocument;

pub(crate) use crate::der::{arrayvec::ArrayVec, bytes_ref::BytesRef, str_ref::StrRef};
#[cfg(feature = "alloc")]
pub(crate) use crate::der::{bytes_owned::BytesOwned, str_owned::StrOwned};
