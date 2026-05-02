//! EdgeRun Wallet — core types and logic.
//!
//! Uses existing edgerun-stream and edgerun-storage for durable event records.
//! Implements DecimalAmount (no floats, no external decimal crates),
//! asset model, status machine, and exchange types.

extern crate alloc;

pub mod amount;
pub mod asset;
pub mod status;
pub mod errors;

// Re-exports
pub use amount::DecimalAmount;
pub use asset::{AssetRef, Asset, canonical_asset_id};
pub use status::{CanonicalOrderStatus, is_terminal, can_transition};
pub use errors::WalletError;
