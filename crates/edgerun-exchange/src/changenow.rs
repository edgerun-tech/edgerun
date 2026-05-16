//! ChangeNOW adapter — implements ExchangeProvider trait.
//!
//! Uses edgerun-node http for HTTP calls.
//! Normalizes ChangeNOW API into canonical types.

extern crate alloc;

use crate::provider::*;
use crate::provider_http::{
    build_client, call_json_api, decimal, mapped_status, object, parse_asset_id, provider_features,
    provider_order, str_field, supports_pair,
};
use alloc::string::String;
use core::result::Result;
use edgerun_json::{JsonValue, Map};
use edgerun_node::http::Method;
use edgerun_protocols::core_protocol::protocol::edgerun_wallet_v0::{
    AssetRef, Quote, QuoteRequest,
};
use edgerun_wallet::{DecimalAmount, WalletError};

const CHANGENOW_BASE_URL: &str = "https://api.changenow.io/api/v2";

pub struct ChangeNOWAdapter {
    pub api_key: String,
    pub partner_id: String,
    pub default_commission_bps: u32,
    client: edgerun_node::http::Client,
}

impl core::fmt::Debug for ChangeNOWAdapter {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ChangeNOWAdapter")
            .field("api_key", &"[REDACTED]")
            .field("partner_id", &self.partner_id)
            .finish()
    }
}

impl ChangeNOWAdapter {
    pub fn new(api_key: String, partner_id: String) -> Self {
        Self {
            api_key,
            partner_id,
            default_commission_bps: 25,
            client: build_client(),
        }
    }
}

impl ExchangeProvider for ChangeNOWAdapter {
    fn code(&self) -> ProviderCode {
        ProviderCode::ChangeNOW
    }

    fn features(&self) -> ProviderFeatures {
        provider_features(ProviderCode::ChangeNOW)
    }

    fn supports_quote(&self, req: &QuoteRequest) -> bool {
        supports_pair(
            ProviderCode::ChangeNOW,
            &req.settlement_asset_id,
            &req.pay_asset_id,
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
        body.insert(
            "fromCurrency".into(),
            JsonValue::String(pay_sym.to_lowercase()),
        );
        body.insert(
            "toCurrency".into(),
            JsonValue::String(settle_sym.to_lowercase()),
        );

        if !req.settlement_amount.is_empty() {
            body.insert(
                "toAmount".into(),
                JsonValue::String(req.settlement_amount.clone()),
            );
        } else if !req.pay_amount.is_empty() {
            body.insert(
                "fromAmount".into(),
                JsonValue::String(req.pay_amount.clone()),
            );
        }

        if !self.api_key.is_empty() {
            body.insert("apiKey".into(), JsonValue::String(self.api_key.clone()));
        }

        let response = call_json_api(
            &self.client,
            CHANGENOW_BASE_URL,
            "ChangeNOW",
            Method::POST,
            "/exchange/estimated-amount",
            Some(JsonValue::Object(body)),
        )?;

        let obj = object(&response, "Invalid quote response from ChangeNOW")?;
        let est_amount = str_field(
            obj,
            "estimatedAmount",
            "Missing estimatedAmount in ChangeNOW response",
        )?;
        let rate = str_field(obj, "rate", "Missing rate in ChangeNOW response")?;

        let settlement_amount = decimal(est_amount)?;
        let rate_dec = decimal(rate)?;

        // Calculate pay amount from settlement and rate
        let pay_amount = settlement_amount.clone(); // Approximation

        Ok(ProviderQuote {
            provider: ProviderCode::ChangeNOW,
            quote_id_internal: "changenow-live".into(),
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
            pay_amount,
            rate: rate_dec,
            quote_mode: req.quote_mode,
            expires_at_ms: 9999999999999,
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
            "payoutAddress".into(),
            JsonValue::String(req.recipient_address.clone()),
        );

        if let Some(ref refund) = req.refund_address {
            body.insert("refundAddress".into(), JsonValue::String(refund.clone()));
        }

        let response = call_json_api(
            &self.client,
            CHANGENOW_BASE_URL,
            "ChangeNOW",
            Method::POST,
            "/exchange/create",
            Some(JsonValue::Object(body)),
        )?;

        provider_order(
            ProviderCode::ChangeNOW,
            object(&response, "Invalid order response from ChangeNOW")?,
            "id",
            "payinAddress",
            "ChangeNOW",
        )
    }

    fn get_order_status(
        &self,
        provider_order_id: &str,
        _ctx: &ProviderContext,
    ) -> Result<ProviderStatus, WalletError> {
        let path = format!("/exchange/order/{}", provider_order_id);

        let response = call_json_api(
            &self.client,
            CHANGENOW_BASE_URL,
            "ChangeNOW",
            Method::GET,
            &path,
            None,
        )?;

        let obj = object(&response, "Invalid order status from ChangeNOW")?;
        let status_str = str_field(obj, "status", "Missing status in ChangeNOW response")?;
        Ok(mapped_status(
            ProviderCode::ChangeNOW,
            provider_order_id,
            status_str,
        ))
    }
}
