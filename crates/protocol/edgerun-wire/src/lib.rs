#![cfg_attr(not(feature = "std"), no_std)]

//! Edgerun wire protocol boundary.
//!
//! The only supported internal wire protocol is rkyv. Legacy structural wire
//! APIs have intentionally been removed so old call sites fail at compile time
//! instead of continuing on a second wire format.

pub const WIRE_PROTOCOL: &str = "rkyv";

pub use rkyv::rancor::Error as WireError;
pub use rkyv::*;
