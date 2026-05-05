//! Removed command result wire codec.
//!
//! Command results must use the rkyv boundary. This transitional codec was
//! removed so callers fail instead of decoding a second wire format.
