//! Async HTTP/1.1 implementation (RFC 9112)
//!
//! This module provides HTTP/1.1 server built on top of
//! [`edgerun_rt`] async primitives. The client is in the unified `HttpClient`.

pub mod body;
pub mod chunked;
pub mod compression;
pub mod connection;
pub mod handler;
pub mod multipart;
pub mod pool;
pub mod range;
pub mod upgrade;
pub mod version;

pub use crate::{Request, Response};

pub use pool::ConnectionPool;
pub use range::{
    build_partial_response, get_range, has_range_header, is_range_satisfiable, parse_range_header,
    range_not_satisfiable_response, resolve_byte_range, ByteRange, ContentRange, RangeSpecifier,
};
pub use version::HttpVersion;
