//! HTTP/1 protocol helpers.

pub mod chunked;
pub mod compression;
pub mod connection;
pub mod multipart;
pub mod range;
pub mod version;

pub use chunked::{parse_chunked_body, parse_chunked_body_with_trailers};
pub use compression::{
    ContentEncoding, accept_encoding_value, compress_body, decompress_body,
    preferred_response_encoding,
};
pub use connection::{ConnectionState, determine_connection};
pub use multipart::{
    MultipartError, MultipartField, extract_boundary, is_multipart, parse_multipart,
};
pub use range::{
    ByteRange, ContentRange, RangeSpecifier, build_partial_response, get_range, has_range_header,
    is_range_satisfiable, parse_range_header, range_not_satisfiable_response, resolve_byte_range,
};
pub use version::{ConnectionDefault, Http1Version};
