//! HTTP version handling (RFC 9112 §2.6).

pub use edgerun_protocols::http::http1::version::{
    ConnectionDefault, Http1Version as HttpVersion, HttpVersionError,
};
