//! SideShift adapter — implements ExchangeProvider trait.
//!
//! Uses edgerun-node http for HTTP calls.
//! Normalizes SideShift API into canonical types.

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

const SIDESHIFT_BASE_URL: &str = "https://sideshift.ai/api/v2";

pub struct SideShiftAdapter {
    pub api_key: String,
    pub affiliate_id: String,
    pub default_commission_bps: u32,
    client: edgerun_node::http::Client,
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
        Self {
            api_key,
            affiliate_id,
            default_commission_bps: 30,
            client: build_client(),
        }
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
        body.insert(
            "depositMethod".into(),
            JsonValue::String(deposit_method.into()),
        );
        body.insert(
            "settleMethod".into(),
            JsonValue::String(settle_method.into()),
        );

        if !req.settlement_amount.is_empty() {
            body.insert(
                "settleAmount".into(),
                JsonValue::String(req.settlement_amount.clone()),
            );
        } else if !req.pay_amount.is_empty() {
            body.insert(
                "depositAmount".into(),
                JsonValue::String(req.pay_amount.clone()),
            );
        }

        if !self.affiliate_id.is_empty() {
            body.insert(
                "affiliateId".into(),
                JsonValue::String(self.affiliate_id.clone()),
            );
        }

        let response = call_json_api(
            &self.client,
            SIDESHIFT_BASE_URL,
            "SideShift",
            Method::POST,
            "/quotes",
            Some(JsonValue::Object(body)),
        )?;

        let obj = object(&response, "Invalid quote response from SideShift")?;
        let quote_id = str_field(obj, "id", "Missing quote ID in SideShift response")?;
        let deposit_amount = str_field(
            obj,
            "depositAmount",
            "Missing deposit amount in SideShift response",
        )?;
        let settle_amount = str_field(
            obj,
            "settleAmount",
            "Missing settle amount in SideShift response",
        )?;
        let rate = str_field(obj, "rate", "Missing rate in SideShift response")?;
        let expires_at = str_field(obj, "expiresAt", "Missing expiresAt in SideShift response")?;

        let expires_ms: u64 = expires_at.parse().map_err(|_| {
            WalletError::ProviderError("Invalid expiresAt format from SideShift".into())
        })?;

        let settlement_amount = decimal(settle_amount)?;
        let pay_amount = decimal(deposit_amount)?;
        let rate_dec = decimal(rate)?;

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
        body.insert(
            "quoteId".into(),
            JsonValue::String(req.quote_id_internal.clone()),
        );
        body.insert(
            "settleAddress".into(),
            JsonValue::String(req.recipient_address.clone()),
        );

        if let Some(ref refund) = req.refund_address {
            body.insert("refundAddress".into(), JsonValue::String(refund.clone()));
        }

        let response = call_json_api(
            &self.client,
            SIDESHIFT_BASE_URL,
            "SideShift",
            Method::POST,
            "/orders",
            Some(JsonValue::Object(body)),
        )?;

        provider_order(
            ProviderCode::SideShift,
            object(&response, "Invalid order response from SideShift")?,
            "id",
            "depositAddress",
            "SideShift",
        )
    }

    fn get_order_status(
        &self,
        provider_order_id: &str,
        _ctx: &ProviderContext,
    ) -> Result<ProviderStatus, WalletError> {
        let path = format!("/orders/{}", provider_order_id);

        let response = call_json_api(
            &self.client,
            SIDESHIFT_BASE_URL,
            "SideShift",
            Method::GET,
            &path,
            None,
        )?;

        let obj = object(&response, "Invalid order status response from SideShift")?;
        let status_str = str_field(obj, "status", "Missing status in SideShift response")?;
        Ok(mapped_status(
            ProviderCode::SideShift,
            provider_order_id,
            status_str,
        ))
    }
}
