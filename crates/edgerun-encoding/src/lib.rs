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
//!
//! All implementations are self-contained with zero external dependencies.

#![no_std]

extern crate alloc;

pub mod base64;
pub mod cstring;
pub mod hex;
pub mod percent;
pub mod protobuf;
pub mod string_field;
pub mod tlv;
