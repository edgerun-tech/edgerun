//! Removed mesh capability server byte path.
//!
//! Mesh capability servers must use rkyv archived envelopes. The previous server
//! wrapper depended on removed transport and dispatcher shims.
