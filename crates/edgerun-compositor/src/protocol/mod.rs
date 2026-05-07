//! Protocol interface definitions.
//!
//! Each Wayland interface is defined by:
//! - A set of **requests** (client → server) with opcode + argument signature
//! - A set of **events** (server → client) with opcode + argument signature

pub mod dispatch;
mod dispatch_legacy;
