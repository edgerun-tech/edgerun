//! FF.io adapter — implements ExchangeProvider trait.
//!
//! Uses edgerun-node http for HTTP calls.
//! Normalizes FF.io API into canonical types.
//! Behind `ffio` feature flag.

extern crate alloc;

use crate::provider::*;
use crate::provider_http::{
    build_client, call_json_api, decimal, mapped_status, object, parse_asset_id, provider_order,
    str_field,
};
use alloc::string::String;
use core::result::Result;
use edgerun_json::{JsonValue, Map};
use edgerun_node::http::Method;
use edgerun_protocols::core_protocol::protocol::edgerun_wallet_v0::{
    AssetRef, Quote, QuoteRequest,
};
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
        Self {
            api_key,
            api_secret,
            refcode,
            afftax_bps: 0,
            client: build_client(),
        }
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
        let (settle_sym, _) = parse_asset_id(&req.settlement_asset_id);
        let (pay_sym, _) = parse_asset_id(&req.pay_asset_id);

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
        let (settle_sym, settle_net) = parse_asset_id(&req.settlement_asset_id);
        let (pay_sym, _) = parse_asset_id(&req.pay_asset_id);

        let mut body = Map::new();
        body.insert("from".into(), JsonValue::String(pay_sym.to_lowercase()));
        body.insert("to".into(), JsonValue::String(settle_sym.to_lowercase()));

        if !req.settlement_amount.is_empty() {
            body.insert(
                "toAmount".into(),
                JsonValue::String(req.settlement_amount.clone()),
            );
        }

        let response = call_json_api(
            &self.client,
            FFIO_BASE_URL,
            "FF.io",
            Method::POST,
            "/quote",
            Some(JsonValue::Object(body)),
        )?;

        let obj = object(&response, "Invalid quote response from FF.io")?;
        let est_amount = str_field(obj, "toAmount", "Missing toAmount in FF.io response")?;
        let rate = str_field(obj, "rate", "Missing rate in FF.io response")?;
        let quote_id = str_field(obj, "id", "Missing id in FF.io response")?;

        let settlement_amount = decimal(est_amount)?;
        let rate_dec = decimal(rate)?;

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

        let response = call_json_api(
            &self.client,
            FFIO_BASE_URL,
            "FF.io",
            Method::POST,
            "/order",
            Some(JsonValue::Object(body)),
        )?;

        provider_order(
            ProviderCode::FFio,
            object(&response, "Invalid order response from FF.io")?,
            "id",
            "fromAddress",
            "FF.io",
        )
    }

    fn get_order_status(
        &self,
        provider_order_id: &str,
        _ctx: &ProviderContext,
    ) -> Result<ProviderStatus, WalletError> {
        let path = format!("/order/{}", provider_order_id);

        let response = call_json_api(
            &self.client,
            FFIO_BASE_URL,
            "FF.io",
            Method::GET,
            &path,
            None,
        )?;

        let obj = object(&response, "Invalid order status from FF.io")?;
        let status_str = str_field(obj, "status", "Missing status in FF.io response")?;

        Ok(mapped_status(
            ProviderCode::FFio,
            provider_order_id,
            status_str,
        ))
    }
}
