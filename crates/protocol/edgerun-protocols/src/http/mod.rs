//! HTTP protocol semantics and transport-free codecs.
//!
//! This module owns shared HTTP methods, status codes, headers, URI parsing,
//! and transfer-coding helpers. It does not own sockets, TLS, request routing,
//! pools, servers, or clients.

pub mod auth;
pub mod chunked;
pub mod header;
pub mod http1;
pub mod http2;
#[cfg(feature = "http3")]
pub mod http3;
pub mod message;
pub mod method;
pub mod status;
pub mod uri;

pub use auth::parse_bearer_auth;
pub use chunked::{
    ChunkedError, has_chunked_transfer_coding, parse_body, parse_body_with_trailers,
};
pub use header::{HeaderMap, HeaderName, HeaderValue, is_tchar};
pub use message::{HttpMessageError, HttpRequest, HttpResponse};
pub use method::Method;
pub use status::StatusCode;
pub use uri::{Scheme, Uri};
