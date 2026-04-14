//! QUIC variable-length integer encoding — delegates to edgerun-encoding.

pub use edgerun_encoding::quic_varint::{VarintError, decode_varint, encode_varint, encode_varint_vec};
