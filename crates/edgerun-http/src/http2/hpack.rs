//! HPACK header compression for HTTP/2 (RFC 7541).
//!
//! Re-exports `hpack_patched` for the full implementation:
//! static/dynamic tables, integer encoding, Huffman coding, and dynamic table management.

pub use hpack_patched::{Decoder, Encoder};
