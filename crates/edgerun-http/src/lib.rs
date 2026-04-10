//! A dependency-free HTTP/1.1, HTTP/2, and HTTP/3 implementation built exclusively with
//! Rust's standard library.
//!
//! # Protocol Modules
//! - [`http1`] — HTTP/1.1 client and types (RFC 9112, RFC 9110)
//! - [`http2`] — HTTP/2 with HPACK, frames, streams, flow control (RFC 9113, RFC 7541)
//! - [`http3`] — HTTP/3 with QUIC transport and QPACK (RFC 9114, RFC 9000, RFC 9204)
//!
//! # Shared Types
//! The top-level re-exports provide shared HTTP types used across all versions:
//! - [`Method`] — HTTP methods (GET, POST, etc.)
//! - [`StatusCode`] — HTTP status codes with category helpers
//! - [`HeaderName`] / [`HeaderValue`] / [`HeaderMap`] — Header types
//! - [`Uri`] / [`Scheme`] — URI parsing
//! - [`Error`] / [`Result`] — Shared error types
//!
//! # HTTP/1.1 Example
//! ```no_run
//! use edgerun_http::http1::{Client, Request, Response};
//! use edgerun_http::{Method, StatusCode};
//!
//! let request = Request::builder()
//!     .method(Method::GET)
//!     .uri("http://example.com/api/data")
//!     .header("Accept", "application/json")
//!     .build()
//!     .unwrap();
//! ```
//!
//! # HTTP/2 Example
//! ```no_run
//! use edgerun_http::http2::{Connection, Encoder, Decoder};
//! use edgerun_http::http2::frame::HeadersFrame;
//!
//! let mut encoder = Encoder::new();
//! let headers = [
//!     (":method", "GET"),
//!     (":path", "/"),
//!     (":scheme", "https"),
//!     (":authority", "example.com"),
//! ];
//! let header_block = encoder.encode(headers.iter().map(|(k, v)| (k.as_bytes(), v.as_bytes())));
//!
//! let headers_frame = HeadersFrame::new(1, header_block, true);
//! ```
//!
//! # HTTP/3 Example
//! ```no_run
//! use edgerun_http::http3::{Http3Connection, QpackEncoder, QpackDecoder};
//! use edgerun_http::http3::http3::frame::Http3FrameType;
//!
//! let mut qpack_encoder = QpackEncoder::new();
//! let mut qpack_decoder = QpackDecoder::new();
//!
//! let header_block = qpack_encoder.encode(&[
//!     (":method", "GET"),
//!     (":scheme", "https"),
//!     (":path", "/api/data"),
//!     (":authority", "example.com"),
//! ]).unwrap();
//! ```
//!
//! # Official Specifications
//! This crate implements the following RFCs:
//!
//! | Module | RFC | Title |
//! |--------|-----|-------|
//! | http1  | [RFC 9110](https://www.rfc-editor.org/rfc/rfc9110) | HTTP Semantics |
//! | http1  | [RFC 9112](https://www.rfc-editor.org/rfc/rfc9112) | HTTP/1.1 Message Syntax |
//! | http2  | [RFC 9113](https://www.rfc-editor.org/rfc/rfc9113) | HTTP/2 |
//! | http2  | [RFC 7541](https://www.rfc-editor.org/rfc/rfc7541) | HPACK Header Compression |
//! | http3  | [RFC 9114](https://www.rfc-editor.org/rfc/rfc9114) | HTTP/3 |
//! | http3  | [RFC 9000](https://www.rfc-editor.org/rfc/rfc9000) | QUIC Transport |
//! | http3  | [RFC 9001](https://www.rfc-editor.org/rfc/rfc9001) | QUIC TLS Mapping |
//! | http3  | [RFC 9204](https://www.rfc-editor.org/rfc/rfc9204) | QPACK Header Compression |
//!
//! Note: RFC 9110–9113 (2022) obsolete RFC 7230–7235 (the original HTTP/1.1 suite),
//! RFC 7540 (HTTP/2), RFC 7541 (HPACK), and RFC 7231 (Semantics).

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
#![allow(non_camel_case_types)] // HTTP/2 error code names follow RFC 9113

// ---------------------------------------------------------------------------
// Shared HTTP types (re-exported from the types module)
// ---------------------------------------------------------------------------
pub mod header;
pub mod method;
pub mod status;
pub mod uri;
pub mod error;

pub use error::{Error, HttpError, Result};
pub use header::{HeaderMap, HeaderName, HeaderValue};
pub use method::Method;
pub use status::StatusCode;
pub use uri::{Scheme, Uri};

// ---------------------------------------------------------------------------
// Protocol-specific modules
// ---------------------------------------------------------------------------
pub mod http1;
pub mod http2;
pub mod http3;

#[cfg(feature = "tls")]
pub mod tls;

#[cfg(test)]
mod semantics_conformance;
