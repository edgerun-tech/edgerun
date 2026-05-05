//! Rkyv-only protocol boundary.
//!
//! Transitional protocol boundary structs have been removed so there is only one
//! internal wire format. Move callers to archive concrete protocol records at
//! the rkyv boundary.
