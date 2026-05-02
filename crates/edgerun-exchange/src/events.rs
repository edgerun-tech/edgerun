//! Exchange event model — canonical events that drive all exchange state.
//!
//! State is derived from events, not from mutable provider status.
//! Every operation produces an immutable event that is appended to
//! the exchange stream. Events are the only source of truth.
//!
//! Event flow:
//!   QuoteRequested -> QuoteCreated (with provider_code_internal)
//!   OrderCreated (provider creates provider_order_id, we generate order_id)
//!   DepositObserved (on-chain or provider-reported)
//!   ProviderStatusObserved (poller or webhook)
//!   OrderStatusChanged (derived, canonical)
//!   OrderCompleted / OrderFailed / ManualReviewRequired (terminal)

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

/// Exchange event types. Every event is immutable once recorded.
#[derive(Debug, Clone)]
pub enum ExchangeEvent {
    /// User requested a quote — the quote object is created.
    QuoteCreated {
        quote_id: String,
        settlement_asset_id: String,
        pay_asset_id: String,
        settlement_amount: String,
        pay_amount: String,
        rate: String,
        expires_at_ms: u64,
        /// Internal only — never exposed in public API.
        provider_code: String,
    },
    /// Order created from a quote. Provider order ID is internal.
    OrderCreated {
        /// EdgeRun-generated order ID (public).
        order_id: String,
        quote_id: String,
        /// Provider's order ID — internal only.
        provider_order_id: String,
        deposit_address: String,
        recipient_address: String,
        refund_address: Option<String>,
        /// Internal only.
        provider_code: String,
        created_at_ms: u64,
    },
    /// Deposit observed on-chain or via provider.
    DepositObserved {
        order_id: String,
        tx_id: String,
        network: String,
        amount: String,
        confirmations: u64,
        observed_at_ms: u64,
    },
    /// Provider reported a status update. This is an observation,
    /// not the canonical status. Canonical status is derived from
    /// the ordered sequence of events.
    ProviderStatusObserved {
        order_id: String,
        /// Internal only.
        provider_code: String,
        provider_order_id: String,
        provider_status_string: String,
        mapped_canonical_status: i32,
        observed_at_ms: u64,
    },
    /// Canonical order status changed. Derived from events,
    /// not directly from provider status.
    OrderStatusChanged {
        order_id: String,
        from_status: i32,
        to_status: i32,
        reason: String,
        changed_at_ms: u64,
    },
    /// Order completed — final terminal state.
    OrderCompleted {
        order_id: String,
        payout_tx_id: Option<String>,
        payout_network: Option<String>,
        completed_at_ms: u64,
    },
    /// Order failed — final terminal state.
    OrderFailed {
        order_id: String,
        reason: String,
        failed_at_ms: u64,
    },
    /// Manual review required — provider contradiction or policy flag.
    ManualReviewRequired {
        order_id: String,
        reason: String,
        canonical_status: i32,
        provider_status: i32,
        flagged_at_ms: u64,
    },
}

impl ExchangeEvent {
    /// Returns the event type name for logging/indexing.
    pub fn event_type(&self) -> &'static str {
        match self {
            ExchangeEvent::QuoteCreated { .. } => "QUOTE_CREATED",
            ExchangeEvent::OrderCreated { .. } => "ORDER_CREATED",
            ExchangeEvent::DepositObserved { .. } => "DEPOSIT_OBSERVED",
            ExchangeEvent::ProviderStatusObserved { .. } => "PROVIDER_STATUS_OBSERVED",
            ExchangeEvent::OrderStatusChanged { .. } => "ORDER_STATUS_CHANGED",
            ExchangeEvent::OrderCompleted { .. } => "ORDER_COMPLETED",
            ExchangeEvent::OrderFailed { .. } => "ORDER_FAILED",
            ExchangeEvent::ManualReviewRequired { .. } => "MANUAL_REVIEW_REQUIRED",
        }
    }

    /// Returns the order_id if this event is associated with an order.
    pub fn order_id(&self) -> Option<&str> {
        match self {
            ExchangeEvent::QuoteCreated { .. } => None,
            ExchangeEvent::OrderCreated { order_id, .. } => Some(order_id),
            ExchangeEvent::DepositObserved { order_id, .. } => Some(order_id),
            ExchangeEvent::ProviderStatusObserved { order_id, .. } => Some(order_id),
            ExchangeEvent::OrderStatusChanged { order_id, .. } => Some(order_id),
            ExchangeEvent::OrderCompleted { order_id, .. } => Some(order_id),
            ExchangeEvent::OrderFailed { order_id, .. } => Some(order_id),
            ExchangeEvent::ManualReviewRequired { order_id, .. } => Some(order_id),
        }
    }

    /// Returns whether this event is terminal for the order.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ExchangeEvent::OrderCompleted { .. }
                | ExchangeEvent::OrderFailed { .. }
                | ExchangeEvent::ManualReviewRequired { .. }
        )
    }
}
