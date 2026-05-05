//! Removed app package wire codec.
//!
//! App package payloads must use the rkyv boundary. This transitional codec was
//! removed so callers fail instead of decoding a second wire format.
