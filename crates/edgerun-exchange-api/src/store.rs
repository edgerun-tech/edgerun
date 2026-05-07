//! In-memory exchange store.
//!
//! Holds quotes, orders, and events in memory.
//! Generates EdgeRun quote/order IDs and keeps provider identifiers internal.
//! State is tracked per event and mirrored into the protocol event stream when
//! the exchange stream runtime is configured.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use edgerun_exchange::projection::{is_terminal_status, terminal_event_for_status};
use edgerun_exchange::provider::{ProviderQuote, ProviderStatus};
use edgerun_exchange::{project_order_events, ExchangeEvent, ExchangeOrderProjection};
use edgerun_protocols::core_protocol::protocol::edgerun_wallet_v0::PaymentRequest;
use edgerun_wallet::{can_transition, canonical_asset_id, CanonicalOrderStatus};

fn is_valid_generated_id(value: &str, prefix: &str) -> bool {
    value.starts_with(prefix)
        && value.len() == prefix.len() + 32
        && value[prefix.len()..].chars().all(|c| c.is_ascii_hexdigit())
}

/// EdgeRun-generated order ID.
#[derive(Debug, Clone)]
pub struct ExchangeOrderId(String);

impl ExchangeOrderId {
    pub fn new() -> Self {
        use edgerun_rt::rng::Rng;
        let mut rng = Rng::new_from_entropy();
        let hi: u64 = rng.next_u64();
        let lo: u64 = rng.next_u64();
        Self(format!("ex-{hi:016x}{lo:016x}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// EdgeRun-generated quote ID.
#[derive(Debug, Clone)]
pub struct ExchangeQuoteId(String);

impl ExchangeQuoteId {
    pub fn new() -> Self {
        use edgerun_rt::rng::Rng;
        let mut rng = Rng::new_from_entropy();
        let hi: u64 = rng.next_u64();
        let lo: u64 = rng.next_u64();
        Self(format!("eq-{hi:016x}{lo:016x}"))
    }

    /// Reconstruct an EdgeRun quote id received through the public API.
    /// This does not create a new quote; it only validates the generated id shape.
    pub fn from_existing(value: &str) -> Option<Self> {
        if is_valid_generated_id(value, "eq-") {
            Some(Self(value.to_string()))
        } else {
            None
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// EdgeRun-generated payment request ID.
#[derive(Debug, Clone)]
pub struct PaymentRequestId(String);

impl PaymentRequestId {
    pub fn new() -> Self {
        use edgerun_rt::rng::Rng;
        let mut rng = Rng::new_from_entropy();
        let hi: u64 = rng.next_u64();
        let lo: u64 = rng.next_u64();
        Self(format!("pr-{hi:016x}{lo:016x}"))
    }

    pub fn from_existing(value: &str) -> Option<Self> {
        if is_valid_generated_id(value, "pr-") {
            Some(Self(value.to_string()))
        } else {
            None
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Stored quote record.
#[derive(Debug, Clone)]
pub struct StoredQuote {
    pub exchange_quote_id: ExchangeQuoteId,
    /// Internal provider quote identifier. Never expose through public API.
    pub quote_id_internal: String,
    /// Internal provider code. Never expose through public API.
    pub provider_code: String,
    pub settlement_asset_id: String,
    pub pay_asset_id: String,
    pub settlement_amount: String,
    pub pay_amount: String,
    pub rate: String,
    pub quote_mode: i32,
    pub expires_at_ms: u64,
}

/// Stored order record.
#[derive(Debug, Clone)]
pub struct StoredOrder {
    pub exchange_order_id: ExchangeOrderId,
    pub exchange_quote_id: ExchangeQuoteId,
    pub settlement_asset_id: String,
    pub pay_asset_id: String,
    pub settlement_amount: String,
    pub pay_amount: String,
    pub deposit_address: String,
    pub recipient_address: String,
    pub refund_address: Option<String>,
    pub status: i32,
    pub created_at_ms: u64,
    /// Internal only — never in public API response.
    pub provider_order_id: String,
    /// Internal only.
    pub provider_code: String,
}

/// In-memory exchange store.
#[derive(Debug, Default)]
pub struct ExchangeStore {
    payment_requests: BTreeMap<String, PaymentRequest>,
    quotes: BTreeMap<String, StoredQuote>,
    orders: BTreeMap<String, StoredOrder>,
    events: Vec<ExchangeEvent>,
}

impl ExchangeStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Store a marketplace payment request as a concrete wallet protocol type.
    pub fn store_payment_request(
        &mut self,
        settlement_asset_id: String,
        settlement_amount: String,
        recipient_address: Option<String>,
        description: Option<String>,
        created_at_ms: u64,
        expires_at_ms: u64,
        pay_asset_id: Option<String>,
    ) -> PaymentRequestId {
        let id = PaymentRequestId::new();
        let request = PaymentRequest {
            request_id: id.as_str().into(),
            settlement_asset_id,
            settlement_amount,
            recipient_address,
            description,
            created_at_ms,
            expires_at_ms,
            pay_asset_id,
        };
        self.payment_requests.insert(id.as_str().into(), request);
        id
    }

    pub fn get_payment_request(&self, id: &str) -> Option<&PaymentRequest> {
        self.payment_requests.get(id)
    }

    /// Store a provider quote and generate an EdgeRun quote ID.
    /// Emits a QuoteCreated event.
    pub fn store_quote(&mut self, provider_quote: &ProviderQuote) -> ExchangeQuoteId {
        let id = ExchangeQuoteId::new();
        let provider_code = provider_quote.provider.as_str().to_string();
        let settle_id = canonical_asset_id(
            &provider_quote.settlement_asset.symbol,
            &provider_quote.settlement_asset.network,
            provider_quote.settlement_asset.contract.as_deref(),
        );
        let pay_id = canonical_asset_id(
            &provider_quote.pay_asset.symbol,
            &provider_quote.pay_asset.network,
            provider_quote.pay_asset.contract.as_deref(),
        );

        let stored = StoredQuote {
            exchange_quote_id: id.clone(),
            quote_id_internal: provider_quote.quote_id_internal.clone(),
            provider_code: provider_code.clone(),
            settlement_asset_id: settle_id.clone(),
            pay_asset_id: pay_id.clone(),
            settlement_amount: provider_quote.settlement_amount.to_string(),
            pay_amount: provider_quote.pay_amount.to_string(),
            rate: provider_quote.rate.to_string(),
            quote_mode: provider_quote.quote_mode,
            expires_at_ms: provider_quote.expires_at_ms,
        };
        self.quotes.insert(id.as_str().into(), stored);

        self.append_event(ExchangeEvent::QuoteCreated {
            quote_id: id.as_str().into(),
            settlement_asset_id: settle_id.clone(),
            pay_asset_id: pay_id.clone(),
            settlement_amount: provider_quote.settlement_amount.to_string(),
            pay_amount: provider_quote.pay_amount.to_string(),
            rate: provider_quote.rate.to_string(),
            expires_at_ms: provider_quote.expires_at_ms,
            provider_code,
        });

        id
    }

    /// Create an order from a stored quote.
    /// Generates an EdgeRun order ID; provider_order_id/provider_code are internal only.
    /// Emits an OrderCreated event.
    pub fn store_order(
        &mut self,
        quote_id: &ExchangeQuoteId,
        provider_order_id: String,
        deposit_address: String,
        recipient_address: String,
        refund_address: Option<String>,
        provider_code: String,
        status: i32,
        created_at_ms: u64,
    ) -> Option<ExchangeOrderId> {
        let quote = self.quotes.get(quote_id.as_str())?;
        if quote.provider_code != provider_code {
            return None;
        }

        let order_id = ExchangeOrderId::new();

        let stored = StoredOrder {
            exchange_order_id: order_id.clone(),
            exchange_quote_id: quote_id.clone(),
            settlement_asset_id: quote.settlement_asset_id.clone(),
            pay_asset_id: quote.pay_asset_id.clone(),
            settlement_amount: quote.settlement_amount.clone(),
            pay_amount: quote.pay_amount.clone(),
            deposit_address: deposit_address.clone(),
            recipient_address: recipient_address.clone(),
            refund_address: refund_address.clone(),
            status,
            created_at_ms,
            provider_order_id: provider_order_id.clone(),
            provider_code: provider_code.clone(),
        };
        self.orders.insert(order_id.as_str().into(), stored);

        self.append_event(ExchangeEvent::OrderCreated {
            order_id: order_id.as_str().into(),
            quote_id: quote_id.as_str().into(),
            provider_order_id,
            deposit_address,
            recipient_address,
            refund_address,
            provider_code,
            created_at_ms,
        });

        if status != 3 {
            self.append_event(ExchangeEvent::OrderStatusChanged {
                order_id: order_id.as_str().into(),
                from_status: 3,
                to_status: status,
                reason: "provider_initial_status".into(),
                changed_at_ms: created_at_ms,
            });
        }

        Some(order_id)
    }

    pub fn get_order(&self, id: &str) -> Option<&StoredOrder> {
        self.orders.get(id)
    }

    pub fn get_quote(&self, id: &str) -> Option<&StoredQuote> {
        self.quotes.get(id)
    }

    pub fn append_event(&mut self, event: ExchangeEvent) {
        self.events.push(event);
    }

    /// Record a provider status observation and derive canonical order events.
    ///
    /// Provider delivery is not authority by itself; this records the provider
    /// observation first, then advances canonical status only if the wallet
    /// status machine accepts the transition.
    pub fn record_provider_status(
        &mut self,
        order_id: &str,
        status: &ProviderStatus,
    ) -> Option<ExchangeOrderProjection> {
        let provider_order_id = status.provider_order_id.clone();
        let provider_code = status.provider.as_str().to_string();
        let provider_status_string = status
            .status_detail
            .clone()
            .unwrap_or_else(|| status.status.to_string());

        let current_status = self.project_order(order_id)?.canonical_status;
        let expected_provider = self.orders.get(order_id)?.provider_code.clone();
        if expected_provider != provider_code {
            self.append_event(ExchangeEvent::ManualReviewRequired {
                order_id: order_id.into(),
                reason: "provider_status_mismatch".into(),
                canonical_status: current_status,
                provider_status: status.status,
                flagged_at_ms: status.updated_at_ms,
            });
            return self.project_order(order_id);
        }

        self.append_event(ExchangeEvent::ProviderStatusObserved {
            order_id: order_id.into(),
            provider_code,
            provider_order_id,
            provider_status_string: provider_status_string.clone(),
            mapped_canonical_status: status.status,
            observed_at_ms: status.updated_at_ms,
        });

        if current_status == status.status {
            return self.project_order(order_id);
        }

        let Some(from) = canonical_order_status_from_i32(current_status) else {
            self.append_event(ExchangeEvent::ManualReviewRequired {
                order_id: order_id.into(),
                reason: "unknown_current_canonical_status".into(),
                canonical_status: current_status,
                provider_status: status.status,
                flagged_at_ms: status.updated_at_ms,
            });
            return self.project_order(order_id);
        };
        let Some(to) = canonical_order_status_from_i32(status.status) else {
            self.append_event(ExchangeEvent::ManualReviewRequired {
                order_id: order_id.into(),
                reason: "unknown_provider_canonical_status".into(),
                canonical_status: current_status,
                provider_status: status.status,
                flagged_at_ms: status.updated_at_ms,
            });
            return self.project_order(order_id);
        };

        if !can_transition(from, to) {
            self.append_event(ExchangeEvent::ManualReviewRequired {
                order_id: order_id.into(),
                reason: "invalid_provider_status_transition".into(),
                canonical_status: current_status,
                provider_status: status.status,
                flagged_at_ms: status.updated_at_ms,
            });
            return self.project_order(order_id);
        }

        self.append_event(ExchangeEvent::OrderStatusChanged {
            order_id: order_id.into(),
            from_status: current_status,
            to_status: status.status,
            reason: "provider_status_observed".into(),
            changed_at_ms: status.updated_at_ms,
        });

        if is_terminal_status(status.status) {
            if let Some(event) = terminal_event_for_status(
                order_id.into(),
                status.status,
                provider_status_string,
                status.updated_at_ms,
            ) {
                self.append_event(event);
            }
        }

        if let Some(order) = self.orders.get_mut(order_id) {
            order.status = status.status;
        }

        self.project_order(order_id)
    }

    pub fn events_from(&self, start_index: usize) -> &[ExchangeEvent] {
        if start_index >= self.events.len() {
            &[]
        } else {
            &self.events[start_index..]
        }
    }

    pub fn events_for_order(&self, order_id: &str) -> Vec<&ExchangeEvent> {
        self.events
            .iter()
            .filter(|e| e.order_id() == Some(order_id))
            .collect()
    }

    pub fn project_order(&self, order_id: &str) -> Option<ExchangeOrderProjection> {
        project_order_events(self.events_for_order(order_id), order_id)
    }

    pub fn order_count(&self) -> usize {
        self.orders.len()
    }

    pub fn quote_count(&self) -> usize {
        self.quotes.len()
    }

    pub fn payment_request_count(&self) -> usize {
        self.payment_requests.len()
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}

fn canonical_order_status_from_i32(value: i32) -> Option<CanonicalOrderStatus> {
    match value {
        1 => Some(CanonicalOrderStatus::Quoted),
        2 => Some(CanonicalOrderStatus::QuoteExpired),
        3 => Some(CanonicalOrderStatus::Created),
        4 => Some(CanonicalOrderStatus::AwaitingDeposit),
        5 => Some(CanonicalOrderStatus::DepositSeen),
        6 => Some(CanonicalOrderStatus::DepositConfirmed),
        7 => Some(CanonicalOrderStatus::Exchanging),
        8 => Some(CanonicalOrderStatus::Sending),
        9 => Some(CanonicalOrderStatus::Completed),
        10 => Some(CanonicalOrderStatus::ActionRequired),
        11 => Some(CanonicalOrderStatus::RefundRequired),
        12 => Some(CanonicalOrderStatus::Refunding),
        13 => Some(CanonicalOrderStatus::Refunded),
        14 => Some(CanonicalOrderStatus::Expired),
        15 => Some(CanonicalOrderStatus::Failed),
        16 => Some(CanonicalOrderStatus::Rejected),
        17 => Some(CanonicalOrderStatus::OnHold),
        18 => Some(CanonicalOrderStatus::Canceled),
        19 => Some(CanonicalOrderStatus::PartialDeposits),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_exchange::provider::{ProviderCode, ProviderQuote, ProviderStatus};
    use edgerun_protocols::core_protocol::protocol::edgerun_wallet_v0::AssetRef;
    use edgerun_wallet::DecimalAmount;

    fn provider_quote() -> ProviderQuote {
        ProviderQuote {
            provider: ProviderCode::SideShift,
            quote_id_internal: "provider-quote".into(),
            settlement_asset: AssetRef {
                symbol: "USDT".into(),
                network: "tron".into(),
                contract: None,
                decimals: Some(6),
            },
            pay_asset: AssetRef {
                symbol: "BTC".into(),
                network: "bitcoin".into(),
                contract: None,
                decimals: Some(8),
            },
            settlement_amount: DecimalAmount::parse("100").unwrap(),
            pay_amount: DecimalAmount::parse("0.001").unwrap(),
            rate: DecimalAmount::parse("100000").unwrap(),
            quote_mode: 1,
            expires_at_ms: 9999999999999,
            estimated_seconds: Some(600),
            fees: None,
        }
    }

    fn store_with_created_order() -> (ExchangeStore, String) {
        let mut store = ExchangeStore::new();
        let quote_id = store.store_quote(&provider_quote());
        let order_id = store
            .store_order(
                &quote_id,
                "provider-order".into(),
                "deposit".into(),
                "recipient".into(),
                Some("refund".into()),
                ProviderCode::SideShift.as_str().into(),
                3,
                1000,
            )
            .expect("store order");
        (store, order_id.as_str().into())
    }

    #[test]
    fn exchange_quote_id_from_existing_validates_shape() {
        assert!(ExchangeQuoteId::from_existing("eq-0123456789abcdef0123456789abcdef").is_some());
        assert!(ExchangeQuoteId::from_existing("ex-0123456789abcdef0123456789abcdef").is_none());
        assert!(ExchangeQuoteId::from_existing("eq-short").is_none());
    }

    #[test]
    fn payment_request_id_from_existing_validates_shape() {
        assert!(PaymentRequestId::from_existing("pr-0123456789abcdef0123456789abcdef").is_some());
        assert!(PaymentRequestId::from_existing("eq-0123456789abcdef0123456789abcdef").is_none());
        assert!(PaymentRequestId::from_existing("pr-short").is_none());
    }

    #[test]
    fn store_payment_request_keeps_protocol_type() {
        let mut store = ExchangeStore::new();
        let id = store.store_payment_request(
            "USDT:tron".into(),
            "100".into(),
            Some("seller-wallet".into()),
            Some("order-123".into()),
            1000,
            2000,
            None,
        );
        let request = store
            .get_payment_request(id.as_str())
            .expect("payment request stored");
        assert_eq!(request.request_id, id.as_str());
        assert_eq!(request.settlement_asset_id, "USDT:tron");
        assert_eq!(request.recipient_address.as_deref(), Some("seller-wallet"));
    }

    #[test]
    fn provider_status_completion_records_canonical_and_terminal_events() {
        let (mut store, order_id) = store_with_created_order();
        let projection = store
            .record_provider_status(
                &order_id,
                &ProviderStatus {
                    provider: ProviderCode::SideShift,
                    provider_order_id: "provider-order".into(),
                    status: 4,
                    deposit_tx: None,
                    payout_tx: None,
                    status_detail: Some("awaiting_deposit".into()),
                    updated_at_ms: 1100,
                },
            )
            .expect("project awaiting deposit");
        assert_eq!(projection.canonical_status, 4);

        let projection = store
            .record_provider_status(
                &order_id,
                &ProviderStatus {
                    provider: ProviderCode::SideShift,
                    provider_order_id: "provider-order".into(),
                    status: 7,
                    deposit_tx: None,
                    payout_tx: None,
                    status_detail: Some("processing".into()),
                    updated_at_ms: 1200,
                },
            )
            .expect("project exchanging");
        assert_eq!(projection.canonical_status, 7);

        let projection = store
            .record_provider_status(
                &order_id,
                &ProviderStatus {
                    provider: ProviderCode::SideShift,
                    provider_order_id: "provider-order".into(),
                    status: 9,
                    deposit_tx: None,
                    payout_tx: None,
                    status_detail: Some("completed".into()),
                    updated_at_ms: 1300,
                },
            )
            .expect("project completed");

        assert_eq!(projection.canonical_status, 9);
        assert!(projection.terminal);
        assert_eq!(projection.last_event_type, Some("ORDER_COMPLETED"));
    }

    #[test]
    fn provider_status_mismatch_goes_to_manual_review() {
        let (mut store, order_id) = store_with_created_order();
        let projection = store
            .record_provider_status(
                &order_id,
                &ProviderStatus {
                    provider: ProviderCode::ChangeNOW,
                    provider_order_id: "provider-order".into(),
                    status: 4,
                    deposit_tx: None,
                    payout_tx: None,
                    status_detail: Some("waiting".into()),
                    updated_at_ms: 1100,
                },
            )
            .expect("project manual review");

        assert!(projection.manual_review_required);
        assert_eq!(projection.last_event_type, Some("MANUAL_REVIEW_REQUIRED"));
    }
}
