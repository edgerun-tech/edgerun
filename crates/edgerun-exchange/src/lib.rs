//! EdgeRun Exchange — provider adapters and routing.
//!
//! State is derived from events, not from mutable provider status.
//! Providers are adapters that normalize external APIs into canonical types.
//! All operations produce immutable events (see `events` module).

extern crate alloc;

pub mod audit;
pub mod events;
pub mod policy;
pub mod projection;
pub mod provider;
pub mod provider_mapping;
pub mod router;
pub mod settlement_intent;
pub mod status_machine;
pub mod stream_codec;

#[cfg(feature = "provider-http")]
pub mod changenow;
#[cfg(feature = "provider-http")]
pub mod sideshift;

// FF.io adapter behind feature flag
#[cfg(feature = "ffio")]
pub mod ffio;

// Re-exports
pub use audit::AuditLogger;
pub use events::ExchangeEvent;
pub use projection::{project_order, project_order_events, ExchangeOrderProjection};
pub use provider::ExchangeProvider;
pub use router::route_quote;
pub use settlement_intent::{
    archive_order_intent, archive_payment_request_intent, archive_quote_intent,
    archive_quote_request_intent, archive_receipt_intent, build_app_intent,
    build_identity_routed_settlement_command, encode_app_intent, SettlementCommandDraft,
};
pub use status_machine::StatusMachine;
pub use stream_codec::{
    build_exchange_event_envelope, decode_exchange_event, encode_exchange_event,
    exchange_event_type, exchange_payload_object_kind, ExchangeStreamPayload,
};
