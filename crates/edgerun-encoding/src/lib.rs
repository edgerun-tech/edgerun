//! # edgerun-encoding
//!
//! Consolidated encoding utilities.

#![no_std]

extern crate alloc;
#[cfg(all(feature = "std", not(target_os = "none")))]
extern crate std;

pub mod base32hex;
pub mod base64;
pub mod buf;
pub mod byteorder;
pub mod chunked;
pub mod cstring;
pub mod crc32;
pub mod frame;
pub mod hex;
#[cfg(feature = "hpack")]
pub mod hpack;
pub mod io;
pub mod ip;
pub mod kv;
pub mod net;
pub mod percent;
pub mod protobuf;
pub mod quic_varint;
pub mod quoted_printable;
pub mod rfc2822;
pub mod rfc3339;
pub mod string_field;
pub mod tlv;
pub mod varint;

// HPACK (re-exported from edgerun-hpack)
#[cfg(feature = "hpack")]
pub use edgerun_hpack::{Decoder, DecoderError, Encoder, HuffmanDecoder};

pub use base64::{
    base64url_decode, base64url_encode, base64url_nopad_encode, standard_decode, standard_encode,
    standard_encode_wrapped,
};
pub use crc32::{crc32, Crc32};
