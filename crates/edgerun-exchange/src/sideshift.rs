//! SideShift adapter — implements ExchangeProvider trait.
//!
//! Uses edgerun-http for HTTP calls.
//! Normalizes SideShift API into canonical types.

extern crate alloc;

use alloc::string::String;
use core::result::Result;
use edgerun_http::{HttpClient, Method};
use edgerun_http::client_middleware::Chain;
use edgerun_json::{Map, JsonValue, ToJson, from_str, to_string};
use edgerun_proto::edgerun::v0::wallet::v0::{
    AssetRef, Quote, QuoteRequest,
};
use edgerun_rt::block_on;
use edgerun_wallet::{DecimalAmount, WalletError};
use crate::provider::*;

const SIDESHIFT_BASE_URL: &str = "https://sideshift.ai/api/v2";

pub struct SideShiftAdapter {
    pub api_key: String,
    pub affiliate_id: String,
    pub default_commission_bps: u32,
    client: edgerun_http::Client,
}

impl core::fmt::Debug for SideShiftAdapter {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SideShiftAdapter")
            .field("api_key", &"[REDACTED]")
            .field("affiliate_id", &self.affiliate_id)
            .finish()
    }
}

impl SideShiftAdapter {
    pub fn new(api_key: String, affiliate_id: String) -> Self {
        let http_client = HttpClient::new();
        let client = Chain::new(http_client).build();
        Self {
            api_key,
            affiliate_id,
            default_commission_bps: 30,
            client,
        }
    }

    fn parse_asset_id(id: &str) -> (String, String) {
        let parts: Vec<&str> = id.split(':').collect();
        if parts.len() >= 2 {
            (parts[0].to_string(), parts[1].to_string())
        } else {
            (id.to_string(), "".to_string())
        }
    }

    fn call_api(
        &self,
        method: Method,
        path: &str,
        body: Option<JsonValue>,
    ) -> Result<JsonValue, WalletError> {
        let url = format!("{}{}", SIDESHIFT_BASE_URL, path);
        
        let body_bytes = if let Some(b) = body {
            let body_str = to_string(&b).map_err(|e| WalletError::Serialization(e.to_string()))?;
            Some(body_str.into_bytes())
        } else {
            None
        };

        let response = match method {
            Method::GET => {
                edgerun_rt::block_on(async {
                    self.client.get(&url).await
                })
            }
            Method::POST => {
                edgerun_rt::block_on(async {
                    self.client.post(&url, body_bytes.as_ref().map(|b| b.as_slice()).unwrap_or(b"")).await
                })
            }
            _ => {
                return Err(WalletError::HttpError("unsupported method".into()));
            }
        }.map_err(|e| WalletError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(WalletError::ProviderError(format!(
                "SideShift API error: HTTP {}",
                response.status().as_u16()
            )));
        }

        let body = response.body();
        if body.is_empty() {
            return Err(WalletError::ProviderError("SideShift API returned empty response".into()));
        }

        let body_str = core::str::from_utf8(body)
            .map_err(|e| WalletError::Serialization(e.to_string()))?;
        
        from_str(body_str).map_err(|e| WalletError::Serialization(e.to_string()))
    }
}

impl ExchangeProvider for SideShiftAdapter {
    fn code(&self) -> ProviderCode {
        ProviderCode::SideShift
    }

    fn features(&self) -> ProviderFeatures {
        ProviderFeatures {
            supports_fixed_rate: true,
            supports_float_rate: true,
            supports_refund_address: true,
            requires_destination_tag: false,
            min_confirmations: 3,
        }
    }

    fn supports_quote(&self, req: &QuoteRequest) -> bool {
        let (settle_sym, _) = Self::parse_asset_id(&req.settlement_asset_id);
        let (pay_sym, _) = Self::parse_asset_id(&req.pay_asset_id);
        
        matches!(
            (settle_sym.to_lowercase().as_str(), pay_sym.to_lowercase().as_str()),
            ("usdt", "doge") | ("usdt", "btc") | ("usdt", "eth") |
            ("btc", "usdt") | ("eth", "usdt") | ("doge", "usdt")
        )
    }

    fn quote(
        &self,
        req: &QuoteRequest,
        _ctx: &ProviderContext,
    ) -> Result<ProviderQuote, WalletError> {
        let (settle_sym, settle_net) = Self::parse_asset_id(&req.settlement_asset_id);
        let (pay_sym, _) = Self::parse_asset_id(&req.pay_asset_id);

        let deposit_method = match pay_sym.to_lowercase().as_str() {
            "doge" => "dogecoin",
            "btc" => "bitcoin",
            "eth" => "ethereum",
            "usdt" => "tron",
            _ => return Err(WalletError::InvalidAssetId(pay_sym.clone())),
        };

        let settle_method = match settle_sym.to_lowercase().as_str() {
            "doge" => "dogecoin",
            "btc" => "bitcoin",
            "eth" => "ethereum",
            "usdt" => "tron",
            _ => return Err(WalletError::InvalidAssetId(settle_sym.clone())),
        };

        let mut body = Map::new();
        body.insert("depositMethod".into(), JsonValue::String(deposit_method.into()));
        body.insert("settleMethod".into(), JsonValue::String(settle_method.into()));
        
        if !req.settlement_amount.is_empty() {
            body.insert("settleAmount".into(), JsonValue::String(req.settlement_amount.clone()));
        }
        
        if !self.affiliate_id.is_empty() {
            body.insert("affiliateId".into(), JsonValue::String(self.affiliate_id.clone()));
        }

        let response = self.call_api(Method::POST, "/quotes", Some(JsonValue::Object(body)))?;

        let obj = response.as_object().ok_or(WalletError::ProviderError("Invalid quote response from SideShift".into()))?;
        
        let quote_id = obj.get("id").and_then(|v| v.as_str())
            .ok_or(WalletError::ProviderError("Missing quote ID in SideShift response".into()))?;
        
        let deposit_amount = obj.get("depositAmount").and_then(|v| v.as_str())
            .ok_or(WalletError::ProviderError("Missing deposit amount in SideShift response".into()))?;
        
        let settle_amount = obj.get("settleAmount").and_then(|v| v.as_str())
            .ok_or(WalletError::ProviderError("Missing settle amount in SideShift response".into()))?;
        
        let rate = obj.get("rate").and_then(|v| v.as_str())
            .ok_or(WalletError::ProviderError("Missing rate in SideShift response".into()))?;
        
        let expires_at = obj.get("expiresAt").and_then(|v| v.as_str())
            .ok_or(WalletError::ProviderError("Missing expiresAt in SideShift response".into()))?;
        
        let expires_ms: u64 = expires_at.parse()
            .map_err(|_| WalletError::ProviderError("Invalid expiresAt format from SideShift".into()))?;

        let settlement_amount = DecimalAmount::parse(settle_amount)
            .ok_or_else(|| WalletError::InvalidDecimal(settle_amount.into()))?;
        
        let pay_amount = DecimalAmount::parse(deposit_amount)
            .ok_or_else(|| WalletError::InvalidDecimal(deposit_amount.into()))?;
        
        let rate_dec = DecimalAmount::parse(rate)
            .ok_or_else(|| WalletError::InvalidDecimal(rate.into()))?;

        Ok(ProviderQuote {
            provider: ProviderCode::SideShift,
            quote_id_internal: quote_id.into(),
            settlement_asset: AssetRef {
                symbol: settle_sym,
                network: settle_net,
                contract: None,
                decimals: Some(6),
            },
            pay_asset: AssetRef {
                symbol: pay_sym,
                network: deposit_method.into(),
                contract: None,
                decimals: Some(8),
            },
            settlement_amount,
            pay_amount,
            rate: rate_dec,
            quote_mode: req.quote_mode,
            expires_at_ms: expires_ms,
            estimated_seconds: Some(600),
            fees: None,
        })
    }

    fn create_order(
        &self,
        _quote: &Quote,
        req: &ProviderOrderRequest,
        _ctx: &ProviderContext,
    ) -> Result<ProviderOrder, WalletError> {
        let mut body = Map::new();
        body.insert("quoteId".into(), JsonValue::String(req.quote_id_internal.clone()));
        body.insert("settleAddress".into(), JsonValue::String(req.recipient_address.clone()));
        
        if let Some(ref refund) = req.refund_address {
            body.insert("refundAddress".into(), JsonValue::String(refund.clone()));
        }

        let response = self.call_api(Method::POST, "/orders", Some(JsonValue::Object(body)))?;

        let obj = response.as_object().ok_or(WalletError::ProviderError("Invalid order response from SideShift".into()))?;
        
        let order_id = obj.get("id").and_then(|v| v.as_str())
            .ok_or(WalletError::ProviderError("Missing order ID in SideShift response".into()))?;
        
        let deposit_address = obj.get("depositAddress").and_then(|v| v.as_str())
            .ok_or(WalletError::ProviderError("Missing deposit address in SideShift response".into()))?;

        Ok(ProviderOrder {
            provider: ProviderCode::SideShift,
            provider_order_id: order_id.into(),
            order_id: order_id.into(),
            deposit_address: deposit_address.into(),
            status: 3,
            created_at_ms: 0,
        })
    }

    fn get_order_status(
        &self,
        provider_order_id: &str,
        _ctx: &ProviderContext,
    ) -> Result<ProviderStatus, WalletError> {
        let path = format!("/orders/{}", provider_order_id);
        
        let response = self.call_api(Method::GET, &path, None)?;

        let obj = response.as_object().ok_or(WalletError::ProviderError("Invalid order status response from SideShift".into()))?;
        
        let status_str = obj.get("status").and_then(|v| v.as_str())
            .ok_or(WalletError::ProviderError("Missing status in SideShift response".into()))?;
        
        let status = match status_str {
            "created" => 3,
            "awaiting_deposit" => 4,
            "deposit_received" => 5,
            "processing" => 6,
            "completed" => 7,
            "failed" => 8,
            "cancelled" => 9,
            _ => return Err(WalletError::ProviderError(format!("Unknown SideShift status: {}", status_str))),
        };

        Ok(ProviderStatus {
            provider: ProviderCode::SideShift,
            provider_order_id: provider_order_id.into(),
            status,
            deposit_tx: None,
            payout_tx: None,
            status_detail: Some(status_str.into()),
            updated_at_ms: 0,
        })
    }
}