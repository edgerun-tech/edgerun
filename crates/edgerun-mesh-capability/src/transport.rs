//! Removed mesh capability transport byte codec.
//!
//! Mesh capability envelopes must use the rkyv boundary. The previous envelope
//! byte encoder was removed so callers fail until migrated.
