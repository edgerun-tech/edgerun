//! Async HTTP/1.1 implementation (RFC 9112)
//!
//! This module provides HTTP/1.1 server built on top of
//! [`crate::rt`] async primitives. The client is in the unified `HttpClient`.

pub mod body;
pub mod chunked;
pub mod compression {
    pub use edgerun_protocols::http::http1::compression::*;
}
pub mod connection {
    pub use edgerun_protocols::http::http1::connection::*;
}
pub mod handler;
pub mod multipart {
    pub use edgerun_protocols::http::http1::multipart::*;
}
#[cfg(feature = "client")]
pub mod pool;
pub mod range {
    pub use edgerun_protocols::http::http1::range::*;
}
pub mod upgrade;
pub mod version {
    pub use edgerun_protocols::http::http1::version::{
        ConnectionDefault, Http1Version as HttpVersion, HttpVersionError,
    };
}

pub use crate::http::{Request, Response};

pub use crate::http::runtime::BufReader;
pub use body::{AsyncBodyReader, Body, BodyReader, BodySender};
pub use compression::{
    accept_encoding_value, compress_body, decompress_body, preferred_response_encoding,
    ContentEncoding,
};
pub use connection::{determine_connection, ConnectionState};
pub use handler::{into_handler, into_handler_async, Handler};
pub use multipart::{extract_boundary, is_multipart, parse_multipart, MultipartField};
#[cfg(feature = "client")]
pub use pool::ConnectionPool;
pub use range::{
    build_partial_response, get_range, has_range_header, is_range_satisfiable, parse_range_header,
    range_not_satisfiable_response, resolve_byte_range, ByteRange, ContentRange, RangeSpecifier,
};
pub use upgrade::{
    build_upgrade_response, build_websocket_accept_headers, is_websocket_upgrade, UpgradeHandler,
    UpgradeProtocol,
};
pub use version::HttpVersion;
