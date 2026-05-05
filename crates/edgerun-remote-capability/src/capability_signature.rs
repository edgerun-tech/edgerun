//! Removed capability signature byte codec.
//!
//! Capability signatures must archive the concrete rkyv payload at the signing
//! boundary. The previous message-byte signing implementation was removed so
//! callers fail until migrated to the single rkyv protocol.
