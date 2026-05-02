//! EdgeRun Exchange — provider adapters and routing.
//!
//! Uses edgerun-stream for durable event storage.
//! Providers are adapters that normalize external APIs into canonical types.

extern crate alloc;

pub mod provider;
pub mod router;
pub mod policy;
pub mod audit;
pub mod status_machine;
pub mod provider_mapping;
pub mod sideshift;
pub mod changenow;

// FF.io adapter behind feature flag
#[cfg(feature = "ffio")]
pub mod ffio;

// Re-exports
pub use provider::ExchangeProvider;
pub use router::route_quote;
pub use status_machine::StatusMachine;
pub use audit::AuditLogger;
