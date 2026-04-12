//! XML types — generated via protobuf.
#![cfg_attr(not(test), no_std)]
pub mod edgerun { pub mod v0 { pub mod r#xml { include!("gen/edgerun.v0.xml.rs"); } } }
pub use edgerun::v0::r#xml::*;
