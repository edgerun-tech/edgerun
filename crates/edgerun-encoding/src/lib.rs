//! # edgerun-encoding
//!
//! Consolidated encoding utilities for the edgerun workspace.
//! Provides a single boundary for all encoding/decoding operations:
//! - Hex encoding/decoding
//! - Base64 (standard + URL-safe, with/without padding)
//! - Percent/URL encoding
//! - Protobuf canonical encoding + signing helpers
//! - TLV (Tag-Length-Value) encoding/decoding
//! - C-string utilities
//! - Binary string field helpers
//! - RFC3339 timestamp parsing/formatting
//! - Varint (LEB128) encoding/decoding
//! - Quoted-Printable encoding (RFC 2045)
//! - Base32hex encoding (RFC 4648 extended hex)
//! - RFC2822 date formatting
//!
//! All implementations are self-contained with zero external dependencies.

#![no_std]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub mod base32hex;
pub mod base64;
pub mod cstring;
pub mod hex;
pub mod percent;
pub mod protobuf;
pub mod quoted_printable;
pub mod rfc2822;
pub mod rfc3339;
pub mod string_field;
pub mod tlv;
pub mod varint;
