//! Removed mesh capability client byte path.
//!
//! Mesh capability clients must use rkyv archived envelopes. The previous client
//! wrapper depended on removed transport and dispatcher shims.
