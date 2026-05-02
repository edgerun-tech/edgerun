//! EdgeRun Exchange — provider adapters and routing.
//!
//! State is derived from events, not from mutable provider status.
//! Providers are adapters that normalize external APIs into canonical types.
//! All operations produce immutable events (see `events` module).

extern crate alloc;

pub mod provider;
pub mod router;
pub mod policy;
pub mod audit;
pub mod status_machine;
pub mod provider_mapping;
pub mod sideshift;
pub mod changenow;
pub mod events;

// FF.io adapter behind feature flag
#[cfg(feature = "ffio")]
pub mod ffio;

// Re-exports
pub use provider::ExchangeProvider;
pub use router::route_quote;
pub use status_machine::StatusMachine;
pub use audit::AuditLogger;
pub use events::ExchangeEvent;
