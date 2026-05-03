//! In-memory exchange store.
//!
//! Holds quotes, orders, and events in memory.
//! Generates EdgeRun quote/order IDs and keeps provider identifiers internal.
//! State is tracked per event to support eventual event-log persistence.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use edgerun_exchange::provider::ProviderQuote;
use edgerun_exchange::ExchangeEvent;
use edgerun_wallet::canonical_asset_id;

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
    quotes: BTreeMap<String, StoredQuote>,
    orders: BTreeMap<String, StoredOrder>,
    events: Vec<ExchangeEvent>,
}

impl ExchangeStore {
    pub fn new() -> Self {
        Self::default()
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

    pub fn events_for_order(&self, order_id: &str) -> Vec<&ExchangeEvent> {
        self.events
            .iter()
            .filter(|e| e.order_id() == Some(order_id))
            .collect()
    }

    pub fn order_count(&self) -> usize {
        self.orders.len()
    }

    pub fn quote_count(&self) -> usize {
        self.quotes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exchange_quote_id_from_existing_validates_shape() {
        assert!(ExchangeQuoteId::from_existing("eq-0123456789abcdef0123456789abcdef").is_some());
        assert!(ExchangeQuoteId::from_existing("ex-0123456789abcdef0123456789abcdef").is_none());
        assert!(ExchangeQuoteId::from_existing("eq-short").is_none());
    }
}
