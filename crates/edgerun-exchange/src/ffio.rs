//! FF.io adapter — implements ExchangeProvider trait.
//!
//! Uses edgerun-node http for HTTP calls.
//! Normalizes FF.io API into canonical types.
//! Behind `ffio` feature flag.

extern crate alloc;

use crate::provider::*;
use alloc::string::String;
use core::result::Result;
use edgerun_json::{from_str, to_string, JsonValue, Map, ToJson};
use edgerun_node::http::client_middleware::Chain;
use edgerun_node::http::{HttpClient, Method};
use edgerun_node::rt::block_on;
use edgerun_proto::edgerun::v0::wallet::v0::{AssetRef, Quote, QuoteRequest};
use edgerun_wallet::{DecimalAmount, WalletError};

const FFIO_BASE_URL: &str = "https://api.ff.io/api/v1";

pub struct FFioAdapter {
    pub api_key: String,
    pub api_secret: String,
    pub refcode: String,
    pub afftax_bps: u32,
    client: edgerun_node::http::Client,
}

impl core::fmt::Debug for FFioAdapter {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FFioAdapter")
            .field("api_key", &"[REDACTED]")
            .field("refcode", &self.refcode)
            .finish()
    }
}

impl FFioAdapter {
    pub fn new(api_key: String, api_secret: String, refcode: String) -> Self {
        let http_client = HttpClient::new();
        let client = Chain::new(http_client).build();
        Self {
            api_key,
            api_secret,
            refcode,
            afftax_bps: 0,
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
        let url = format!("{}{}", FFIO_BASE_URL, path);

        let body_bytes = if let Some(b) = body {
            let body_str = to_string(&b).map_err(|e| WalletError::Serialization(e.to_string()))?;
            Some(body_str.into_bytes())
        } else {
            None
        };

        let response = match method {
            Method::GET => block_on(async { self.client.get(&url).await }),
            Method::POST => block_on(async {
                self.client
                    .post(
                        &url,
                        body_bytes.as_ref().map(|b| b.as_slice()).unwrap_or(b""),
                    )
                    .await
            }),
            _ => return Err(WalletError::HttpError("unsupported method".into())),
        }
        .map_err(|e| WalletError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(WalletError::ProviderError(format!(
                "FF.io API error: HTTP {}",
                response.status().as_u16()
            )));
        }

        let body = response.body();
        if body.is_empty() {
            return Err(WalletError::ProviderError(
                "FF.io API returned empty response".into(),
            ));
        }

        let body_str =
            core::str::from_utf8(body).map_err(|e| WalletError::Serialization(e.to_string()))?;
        from_str(body_str).map_err(|e| WalletError::Serialization(e.to_string()))
    }
}

impl ExchangeProvider for FFioAdapter {
    fn code(&self) -> ProviderCode {
        ProviderCode::FFio
    }

    fn features(&self) -> ProviderFeatures {
        ProviderFeatures {
            supports_fixed_rate: true,
            supports_float_rate: true,
            supports_refund_address: true,
            requires_destination_tag: false,
            min_confirmations: 1,
        }
    }

    fn supports_quote(&self, req: &QuoteRequest) -> bool {
        let (settle_sym, _) = Self::parse_asset_id(&req.settlement_asset_id);
        let (pay_sym, _) = Self::parse_asset_id(&req.pay_asset_id);

        matches!(
            (
                settle_sym.to_lowercase().as_str(),
                pay_sym.to_lowercase().as_str()
            ),
            ("usdt", "doge")
                | ("usdt", "btc")
                | ("usdt", "eth")
                | ("btc", "usdt")
                | ("eth", "usdt")
                | ("doge", "usdt")
        )
    }

    fn quote(
        &self,
        req: &QuoteRequest,
        _ctx: &ProviderContext,
    ) -> Result<ProviderQuote, WalletError> {
        let (settle_sym, settle_net) = Self::parse_asset_id(&req.settlement_asset_id);
        let (pay_sym, _) = Self::parse_asset_id(&req.pay_asset_id);

        let mut body = Map::new();
        body.insert("from".into(), JsonValue::String(pay_sym.to_lowercase()));
        body.insert("to".into(), JsonValue::String(settle_sym.to_lowercase()));

        if !req.settlement_amount.is_empty() {
            body.insert(
                "toAmount".into(),
                JsonValue::String(req.settlement_amount.clone()),
            );
        }

        let response = self.call_api(Method::POST, "/quote", Some(JsonValue::Object(body)))?;

        let obj = response.as_object().ok_or(WalletError::ProviderError(
            "Invalid quote response from FF.io".into(),
        ))?;

        let est_amount =
            obj.get("toAmount")
                .and_then(|v| v.as_str())
                .ok_or(WalletError::ProviderError(
                    "Missing toAmount in FF.io response".into(),
                ))?;

        let rate = obj
            .get("rate")
            .and_then(|v| v.as_str())
            .ok_or(WalletError::ProviderError(
                "Missing rate in FF.io response".into(),
            ))?;

        let quote_id = obj
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(WalletError::ProviderError(
                "Missing id in FF.io response".into(),
            ))?;

        let settlement_amount = DecimalAmount::parse(est_amount)
            .ok_or_else(|| WalletError::InvalidDecimal(est_amount.into()))?;

        let rate_dec =
            DecimalAmount::parse(rate).ok_or_else(|| WalletError::InvalidDecimal(rate.into()))?;

        Ok(ProviderQuote {
            provider: ProviderCode::FFio,
            quote_id_internal: quote_id.into(),
            settlement_asset: AssetRef {
                symbol: settle_sym,
                network: settle_net,
                contract: None,
                decimals: Some(6),
            },
            pay_asset: AssetRef {
                symbol: pay_sym,
                network: "".into(),
                contract: None,
                decimals: Some(8),
            },
            settlement_amount,
            pay_amount: settlement_amount.clone(),
            rate: rate_dec,
            quote_mode: req.quote_mode,
            expires_at_ms: 9999999999999,
            estimated_seconds: Some(180),
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
        body.insert(
            "quoteId".into(),
            JsonValue::String(req.quote_id_internal.clone()),
        );
        body.insert(
            "toAddress".into(),
            JsonValue::String(req.recipient_address.clone()),
        );

        let response = self.call_api(Method::POST, "/order", Some(JsonValue::Object(body)))?;

        let obj = response.as_object().ok_or(WalletError::ProviderError(
            "Invalid order response from FF.io".into(),
        ))?;

        let order_id = obj
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(WalletError::ProviderError(
                "Missing order ID in FF.io response".into(),
            ))?;

        let deposit_address =
            obj.get("fromAddress")
                .and_then(|v| v.as_str())
                .ok_or(WalletError::ProviderError(
                    "Missing fromAddress in FF.io response".into(),
                ))?;

        Ok(ProviderOrder {
            provider: ProviderCode::FFio,
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
        let path = format!("/order/{}", provider_order_id);

        let response = self.call_api(Method::GET, &path, None)?;

        let obj = response.as_object().ok_or(WalletError::ProviderError(
            "Invalid order status from FF.io".into(),
        ))?;

        let status_str =
            obj.get("status")
                .and_then(|v| v.as_str())
                .ok_or(WalletError::ProviderError(
                    "Missing status in FF.io response".into(),
                ))?;

        let status = match status_str {
            "pending" => 3,
            "awaiting_deposit" => 4,
            "processing" => 6,
            "completed" => 7,
            "failed" => 8,
            "cancelled" => 9,
            _ => {
                return Err(WalletError::ProviderError(format!(
                    "Unknown FF.io status: {}",
                    status_str
                )))
            }
        };

        Ok(ProviderStatus {
            provider: ProviderCode::FFio,
            provider_order_id: provider_order_id.into(),
            status,
            deposit_tx: None,
            payout_tx: None,
            status_detail: Some(status_str.into()),
            updated_at_ms: 0,
        })
    }
}
