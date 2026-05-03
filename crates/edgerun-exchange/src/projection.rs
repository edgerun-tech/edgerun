//! Event-sourced projections for exchange order state.
//!
//! These projections are pure derived views over `ExchangeEvent` sequences.
//! They do not call providers, mutate storage, or decide policy. Their job is to
//! make the current in-memory exchange state rebuildable from events, matching
//! the protocol rule that mutable views are derived from committed facts.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::events::ExchangeEvent;

/// Public-ish derived order view.
///
/// Provider identifiers remain present here because this is an internal
/// projection. API handlers must continue to avoid exposing provider internals.
#[derive(Debug, Clone, Default)]
pub struct ExchangeOrderProjection {
    pub order_id: String,
    pub quote_id: String,
    pub provider_order_id: String,
    pub provider_code: String,
    pub deposit_address: String,
    pub recipient_address: String,
    pub refund_address: Option<String>,
    pub canonical_status: i32,
    pub latest_provider_status: Option<i32>,
    pub latest_provider_status_string: Option<String>,
    pub manual_review_required: bool,
    pub terminal: bool,
    pub event_count: usize,
    pub last_event_type: Option<&'static str>,
    pub updated_at_ms: Option<u64>,
}

impl ExchangeOrderProjection {
    pub fn apply(&mut self, event: &ExchangeEvent) {
        self.event_count += 1;
        self.last_event_type = Some(event.event_type());

        match event {
            ExchangeEvent::QuoteCreated { .. } => {}
            ExchangeEvent::OrderCreated {
                order_id,
                quote_id,
                provider_order_id,
                deposit_address,
                recipient_address,
                refund_address,
                provider_code,
                created_at_ms,
            } => {
                self.order_id = order_id.clone();
                self.quote_id = quote_id.clone();
                self.provider_order_id = provider_order_id.clone();
                self.deposit_address = deposit_address.clone();
                self.recipient_address = recipient_address.clone();
                self.refund_address = refund_address.clone();
                self.provider_code = provider_code.clone();
                self.canonical_status = 3; // CREATED
                self.updated_at_ms = Some(*created_at_ms);
            }
            ExchangeEvent::DepositObserved { observed_at_ms, .. } => {
                self.updated_at_ms = Some(*observed_at_ms);
            }
            ExchangeEvent::ProviderStatusObserved {
                provider_status_string,
                mapped_canonical_status,
                observed_at_ms,
                ..
            } => {
                self.latest_provider_status = Some(*mapped_canonical_status);
                self.latest_provider_status_string = Some(provider_status_string.clone());
                self.updated_at_ms = Some(*observed_at_ms);
            }
            ExchangeEvent::OrderStatusChanged {
                to_status,
                changed_at_ms,
                ..
            } => {
                self.canonical_status = *to_status;
                self.updated_at_ms = Some(*changed_at_ms);
            }
            ExchangeEvent::OrderCompleted {
                completed_at_ms, ..
            } => {
                self.canonical_status = 9; // COMPLETED
                self.terminal = true;
                self.updated_at_ms = Some(*completed_at_ms);
            }
            ExchangeEvent::OrderFailed { failed_at_ms, .. } => {
                self.canonical_status = 15; // FAILED
                self.terminal = true;
                self.updated_at_ms = Some(*failed_at_ms);
            }
            ExchangeEvent::ManualReviewRequired {
                canonical_status,
                flagged_at_ms,
                ..
            } => {
                self.canonical_status = *canonical_status;
                self.manual_review_required = true;
                self.terminal = true;
                self.updated_at_ms = Some(*flagged_at_ms);
            }
        }
    }
}

/// Project a single order from a full event slice.
pub fn project_order(events: &[ExchangeEvent], order_id: &str) -> Option<ExchangeOrderProjection> {
    let mut projection = ExchangeOrderProjection::default();
    let mut seen_order = false;

    for event in events {
        if event.order_id() != Some(order_id) {
            continue;
        }
        if matches!(event, ExchangeEvent::OrderCreated { .. }) {
            seen_order = true;
        }
        projection.apply(event);
    }

    seen_order.then_some(projection)
}

/// Project all events associated with a single order.
pub fn project_order_events(events: Vec<&ExchangeEvent>, order_id: &str) -> Option<ExchangeOrderProjection> {
    let mut projection = ExchangeOrderProjection::default();
    let mut seen_order = false;

    for event in events {
        if event.order_id() != Some(order_id) {
            continue;
        }
        if matches!(event, ExchangeEvent::OrderCreated { .. }) {
            seen_order = true;
        }
        projection.apply(event);
    }

    seen_order.then_some(projection)
}

pub fn is_terminal_status(status: i32) -> bool {
    matches!(status, 9 | 13 | 15 | 16 | 17 | 18) // completed/refunded/failed/rejected/on_hold/canceled
}

pub fn terminal_event_for_status(order_id: String, status: i32, reason: String, at_ms: u64) -> Option<ExchangeEvent> {
    match status {
        9 => Some(ExchangeEvent::OrderCompleted {
            order_id,
            payout_tx_id: None,
            payout_network: None,
            completed_at_ms: at_ms,
        }),
        15 | 16 | 18 => Some(ExchangeEvent::OrderFailed {
            order_id,
            reason,
            failed_at_ms: at_ms,
        }),
        _ => None,
    }
}
