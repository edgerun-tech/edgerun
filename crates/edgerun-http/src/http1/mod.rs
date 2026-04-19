//! Async HTTP/1.1 implementation (RFC 9112)
//!
//! This module provides HTTP/1.1 server built on top of
//! [`edgerun_rt`] async primitives. The client is in the unified `HttpClient`.

pub mod handler;
pub mod body;
pub mod request;
pub mod response;
pub mod version;
pub mod connection;
pub mod upgrade;
pub mod compression;
pub mod multipart;
pub mod range;
pub mod chunked;
pub mod pool;

pub use handler::{Handler, into_handler, into_handler_async};
pub use body::{Body, BodyReader, BodySender, AsyncBodyReader};
pub use edgerun_rt::BufReader;
pub use request::{Request, RequestBuilder};
pub use response::Response;
pub use version::HttpVersion;
pub use connection::{ConnectionState, determine_connection};
pub use upgrade::{
    UpgradeProtocol, UpgradeHandler, parse_upgrade_request, is_websocket_upgrade,
    build_upgrade_response, build_websocket_accept_headers,
};
pub use compression::{ContentEncoding, decompress_body, accept_encoding_value};
pub use multipart::{MultipartField, parse_multipart, extract_boundary, is_multipart};
pub use range::{
    RangeSpecifier, ByteRange, ContentRange, parse_range_header, resolve_byte_range,
    is_range_satisfiable, has_range_header, get_range, build_partial_response,
    range_not_satisfiable_response,
};
pub use pool::ConnectionPool;
