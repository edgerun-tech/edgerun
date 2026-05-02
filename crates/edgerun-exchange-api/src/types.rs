//! Public API types for exchange service.
//!
//! Uses canonical proto types from edgerun-proto.

extern crate alloc;

use edgerun_json::{Map, JsonValue, ToJson};

pub use edgerun_proto::edgerun::v0::wallet::v0::AssetRef;
pub use edgerun_proto::edgerun::v0::wallet::v0::TxRef;

/// Quote request from client.
#[derive(Debug)]
pub struct ApiQuoteRequest {
    pub settlement: AssetRef,
    pub pay: AssetRef,
    pub amount: Option<alloc::string::String>,
    pub mode: QuoteMode,
}

#[derive(Debug, Clone, Copy)]
pub enum QuoteMode {
    PayIn,
    SettlementOut,
}

impl ToJson for ApiQuoteRequest {
    fn to_json(&self) -> JsonValue {
        let mut map = Map::new();
        map.insert("settlement".into(), self.settlement.to_json());
        map.insert("pay".into(), self.pay.to_json());
        if let Some(ref amt) = self.amount {
            map.insert("amount".into(), amt.to_json());
        }
        map.insert("mode".into(), match self.mode {
            QuoteMode::PayIn => "pay_in",
            QuoteMode::SettlementOut => "settlement_out",
        }.to_json());
        JsonValue::Object(map)
    }
}

/// Quote response to client.
#[derive(Debug)]
pub struct ApiQuoteResponse {
    pub id: alloc::string::String,
    pub settlement: AssetRef,
    pub pay: AssetRef,
    pub settlement_amount: alloc::string::String,
    pub pay_amount: alloc::string::String,
    pub rate: alloc::string::String,
    pub expires_at: alloc::string::String,
}

impl ToJson for ApiQuoteResponse {
    fn to_json(&self) -> JsonValue {
        let mut map = Map::new();
        map.insert("id".into(), self.id.to_json());
        map.insert("settlement".into(), self.settlement.to_json());
        map.insert("pay".into(), self.pay.to_json());
        map.insert("settlement_amount".into(), self.settlement_amount.to_json());
        map.insert("pay_amount".into(), self.pay_amount.to_json());
        map.insert("rate".into(), self.rate.to_json());
        map.insert("expires_at".into(), self.expires_at.to_json());
        JsonValue::Object(map)
    }
}

/// Order request from client.
#[derive(Debug)]
pub struct ApiOrderRequest {
    pub quote_id: alloc::string::String,
    pub destination: alloc::string::String,
    pub refund_address: Option<alloc::string::String>,
}

impl ToJson for ApiOrderRequest {
    fn to_json(&self) -> JsonValue {
        let mut map = Map::new();
        map.insert("quote_id".into(), self.quote_id.to_json());
        map.insert("destination".into(), self.destination.to_json());
        if let Some(ref r) = self.refund_address {
            map.insert("refund_address".into(), r.to_json());
        }
        JsonValue::Object(map)
    }
}

/// Order response to client.
#[derive(Debug)]
pub struct ApiOrderResponse {
    pub id: alloc::string::String,
    pub status: OrderStatus,
    pub settlement_address: alloc::string::String,
    pub settlement_amount: alloc::string::String,
    pub pay_amount: Option<alloc::string::String>,
    pub tx_ref: Option<TxRef>,
}

impl ToJson for ApiOrderResponse {
    fn to_json(&self) -> JsonValue {
        let mut map = Map::new();
        map.insert("id".into(), self.id.to_json());
        map.insert("status".into(), self.status.to_json());
        map.insert("settlement_address".into(), self.settlement_address.to_json());
        map.insert("settlement_amount".into(), self.settlement_amount.to_json());
        if let Some(ref p) = self.pay_amount {
            map.insert("pay_amount".into(), p.to_json());
        }
        if let Some(ref t) = self.tx_ref {
            map.insert("tx_ref".into(), t.to_json());
        }
        JsonValue::Object(map)
    }
}

/// Order status enum (public-facing, not internal provider codes).
#[derive(Debug, Clone, Copy)]
pub enum OrderStatus {
    Pending,
    WaitingForDeposit,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

impl ToJson for OrderStatus {
    fn to_json(&self) -> JsonValue {
        let s = match self {
            OrderStatus::Pending => "pending",
            OrderStatus::WaitingForDeposit => "waiting_for_deposit",
            OrderStatus::Processing => "processing",
            OrderStatus::Completed => "completed",
            OrderStatus::Failed => "failed",
            OrderStatus::Cancelled => "cancelled",
        };
        JsonValue::String(s.into())
    }
}

/// Supported assets response.
#[derive(Debug)]
pub struct ApiAssetsResponse {
    pub assets: alloc::vec::Vec<AssetInfo>,
    pub count: usize,
}

#[derive(Debug)]
pub struct AssetInfo {
    pub symbol: alloc::string::String,
    pub network: alloc::string::String,
    pub contract: Option<alloc::string::String>,
    pub decimals: u32,
}

impl ToJson for ApiAssetsResponse {
    fn to_json(&self) -> JsonValue {
        let mut map = Map::new();
        map.insert("assets".into(), self.assets.to_json());
        map.insert("count".into(), self.count.to_json());
        JsonValue::Object(map)
    }
}

impl ToJson for AssetInfo {
    fn to_json(&self) -> JsonValue {
        let mut map = Map::new();
        map.insert("symbol".into(), self.symbol.to_json());
        map.insert("network".into(), self.network.to_json());
        if let Some(ref c) = self.contract {
            map.insert("contract".into(), c.to_json());
        }
        map.insert("decimals".into(), self.decimals.to_json());
        JsonValue::Object(map)
    }
}

/// Health check response.
#[derive(Debug)]
pub struct ApiHealthResponse {
    pub status: alloc::string::String,
    pub providers: alloc::collections::BTreeMap<alloc::string::String, ProviderHealth>,
}

#[derive(Debug)]
pub struct ProviderHealth {
    pub available: bool,
    pub latency_ms: Option<u32>,
}

impl ToJson for ApiHealthResponse {
    fn to_json(&self) -> JsonValue {
        let mut map = Map::new();
        map.insert("status".into(), self.status.to_json());
        map.insert("providers".into(), self.providers.to_json());
        JsonValue::Object(map)
    }
}

impl ToJson for ProviderHealth {
    fn to_json(&self) -> JsonValue {
        let mut map = Map::new();
        map.insert("available".into(), self.available.to_json());
        if let Some(l) = self.latency_ms {
            map.insert("latency_ms".into(), l.to_json());
        }
        JsonValue::Object(map)
    }
}