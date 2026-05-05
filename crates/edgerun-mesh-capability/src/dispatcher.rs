//! Removed mesh capability dispatcher byte codec.
//!
//! Mesh capability dispatch must consume rkyv envelopes only. The previous
//! payload decoder was removed so callers fail until migrated.
