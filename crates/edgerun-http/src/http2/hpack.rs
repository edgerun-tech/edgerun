//! HPACK header compression for HTTP/2 (RFC 7541).
//!
//! Re-exports from edgerun-hpack:
//! static/dynamic tables, integer encoding, Huffman coding, and dynamic table management.

pub use edgerun_hpack::{Decoder, Encoder};
