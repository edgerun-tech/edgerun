//! EdgeRun Wallet — core types and logic.
//!
//! Uses existing edgerun-storage::stream and edgerun-storage for durable event records.
//! Implements DecimalAmount (no floats, no external decimal crates),
//! asset model, status machine, and exchange types.

#![no_std]

extern crate alloc;

pub mod amount;
pub mod asset;
pub mod errors;
pub mod status;

// Re-exports
pub use amount::DecimalAmount;
pub use asset::{canonical_asset_id, Asset, AssetRef};
pub use errors::WalletError;
pub use status::{can_transition, is_terminal, CanonicalOrderStatus};
