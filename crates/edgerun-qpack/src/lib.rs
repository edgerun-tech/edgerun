// Vendored from qpack 0.1.0 (crates.io) — all modules made public.
// QPACK header compression for HTTP/3 (RFC 9204).

#![no_std]

extern crate alloc;

pub use self::{
    decoder::{decode_stateless, Decoded, DecoderError},
    encoder::{encode_stateless, EncoderError},
    field::HeaderField,
};

pub mod block;
pub mod dynamic;
pub mod field;
pub mod parse_error;
pub mod static_;
pub mod stream;
pub mod vas;

pub mod decoder;
pub mod encoder;

pub mod prefix_int;
pub mod prefix_string;

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub enum Error {
    Encoder(EncoderError),
    Decoder(DecoderError),
}

impl alloc::fmt::Display for Error {
    fn fmt(&self, f: &mut alloc::fmt::Formatter<'_>) -> alloc::fmt::Result {
        match self {
            Error::Encoder(e) => write!(f, "Encoder {:?}", e),
            Error::Decoder(e) => write!(f, "Decoder {:?}", e),
        }
    }
}
