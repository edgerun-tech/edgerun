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
pub mod projection;
pub mod stream_codec;

// FF.io adapter behind feature flag
#[cfg(feature = "ffio")]
pub mod ffio;

// Re-exports
pub use provider::ExchangeProvider;
pub use router::route_quote;
pub use status_machine::StatusMachine;
pub use audit::AuditLogger;
pub use events::ExchangeEvent;
pub use projection::{project_order, project_order_events, ExchangeOrderProjection};
pub use stream_codec::{
    build_exchange_event_envelope, decode_exchange_event, encode_exchange_event,
    exchange_event_type, exchange_payload_object_kind, ExchangeStreamPayload,
};
