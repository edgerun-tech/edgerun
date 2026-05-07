//! RFC 5322 / MIME email message builder re-export.
//!
//! The transport-free formatter lives in `edgerun-protocols`.

pub use edgerun_protocols::smtp::message_builder::{
    encode_base64, encode_quoted_printable, EmailBuilder, MimePart,
};
