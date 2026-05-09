//! QUIC variable-length integer encoding — delegates to edgerun-encoding.

use alloc::string::{String, ToString};

pub use edgerun_encoding::quic_varint::{
    VarintError, decode_varint, encode_varint, encode_varint_vec,
};

pub fn decode_varint_string(data: &[u8]) -> Result<(u64, usize), String> {
    decode_varint(data).map_err(|e| e.to_string())
}
