//! Exchange API handlers.
//!
//! Honesty contract:
//! - /v1/quote: 501 until provider-backed routing is wired.
//! - /v1/order: 501 until order creation is wired.
//! - /v1/order/:id: 501 until event-derived status is wired.
//! - /v1/assets: Static supported-assets catalog (NOT provider truth).
//! - /health: degraded until provider health checks are real.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use edgerun_http::{Request, Response, StatusCode};
use edgerun_json::{Map, JsonValue, to_string, from_slice};

use crate::types::*;

/// POST /v1/quote — returns 501 until provider-backed routing is implemented.
/// The request is parsed and validated, but no provider quote is called.
pub fn handle_quote(req: &Request) -> Response {
    let body = match req.body() {
        Some(b) => b,
        None => return json_error(400, "missing request body"),
    };

    if body.is_empty() {
        return json_error(400, "missing request body");
    }

    match from_slice(body) {
        Ok(json) => {
            match parse_quote_request(&json) {
                Ok(_req) => {
                    // Quote routing not yet wired to real providers.
                    // Do not return fake data.
                    json_error(501, "quote routing not yet implemented")
                }
                Err(e) => json_error(400, &e),
            }
        }
        Err(e) => json_error(400, &e.to_string()),
    }
}

/// POST /v1/order — returns 501 until order creation is implemented.
pub fn handle_order(req: &Request) -> Response {
    let _body = req.body();
    // Order creation not yet wired to providers or event log.
    json_error(501, "order creation not yet implemented")
}

/// GET /v1/order/:id — returns 501 until event-derived status is implemented.
/// Status must come from the exchange event stream, not from mutable provider state.
pub fn handle_order_status(req: &Request) -> Response {
    let _path = req.uri().path();
    json_error(501, "order status not yet implemented")
}

/// GET /v1/assets — static supported-assets catalog.
/// This is NOT provider truth. Providers may or may not support these pairs at any given time.
/// Actual availability is determined at quote time via provider.quote().
pub fn handle_assets(req: &Request) -> Response {
    let mut assets = Map::new();
    assets.insert("USDT".into(), asset_info("Tron", "TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6t"));
    assets.insert("BTC".into(), asset_info("Bitcoin", ""));
    assets.insert("ETH".into(), asset_info("Ethereum", ""));
    assets.insert("DOGE".into(), asset_info("Dogecoin", ""));
    assets.insert("SOL".into(), asset_info("Solana", ""));

    let mut response = Map::new();
    response.insert("assets".into(), JsonValue::Object(assets));
    response.insert("count".into(), JsonValue::Number(5u64.into()));
    response.insert("source".into(), JsonValue::String("static_catalog".into()));
    response.insert("note".into(), JsonValue::String(
        "This is a static catalog of supported asset types. \
         Actual pair availability depends on provider support and is determined at quote time."
            .into()));

    json_response(200, JsonValue::Object(response))
}

/// GET /health — reports degraded status because provider health checks are not yet implemented.
/// Must not return "healthy" unless providers are actually checked.
pub fn handle_health(req: &Request) -> Response {
    let mut providers = Map::new();
    providers.insert(
        "SIDESHIFT".into(),
        JsonValue::String("not_implemented".into()),
    );
    providers.insert(
        "CHANGENOW".into(),
        JsonValue::String("not_implemented".into()),
    );
    providers.insert(
        "FFIO".into(),
        JsonValue::String("not_implemented".into()),
    );

    let mut status = Map::new();
    status.insert("status".into(), JsonValue::String("degraded".into()));
    status.insert("reason".into(), JsonValue::String(
        "Provider health checks not yet implemented".into()));
    status.insert("providers".into(), JsonValue::Object(providers));

    // 200 but status=degraded so orchestrators can detect
    json_response(200, JsonValue::Object(status))
}

fn json_response(status: u16, body: JsonValue) -> Response {
    let body_str = to_string(&body).unwrap_or_else(|_| "{}".into());
    let mut resp = Response::json(StatusCode::new(status).unwrap(), &body_str);
    resp.headers_mut().insert("Content-Type", "application/json".try_into().unwrap());
    resp
}

fn json_error(status: u16, msg: &str) -> Response {
    let mut map = Map::new();
    map.insert("error".into(), JsonValue::String(msg.into()));
    json_response(status, JsonValue::Object(map))
}

fn asset_info(network: &str, contract: &str) -> JsonValue {
    let mut map = Map::new();
    map.insert("network".into(), JsonValue::String(network.into()));
    if !contract.is_empty() {
        map.insert("contract".into(), JsonValue::String(contract.into()));
    }
    JsonValue::Object(map)
}

fn parse_quote_request(json: &JsonValue) -> Result<QuoteRequestInput, String> {
    let obj = json.as_object().ok_or("expected JSON object".to_string())?;
    
    let settlement = obj.get("settlement").ok_or("missing 'settlement'".to_string())?;
    let settlement_obj = settlement.as_object().ok_or("'settlement' must be object".to_string())?;
    let settlement_symbol = settlement_obj.get("symbol").and_then(|v| v.as_str()).ok_or("missing settlement.symbol".to_string())?;
    let settlement_network = settlement_obj.get("network").and_then(|v| v.as_str()).ok_or("missing settlement.network".to_string())?;

    let pay = obj.get("pay").ok_or("missing 'pay'".to_string())?;
    let pay_obj = pay.as_object().ok_or("'pay' must be object".to_string())?;
    let pay_symbol = pay_obj.get("symbol").and_then(|v| v.as_str()).ok_or("missing pay.symbol".to_string())?;
    let pay_network = pay_obj.get("network").and_then(|v| v.as_str()).ok_or("missing pay.network".to_string())?;

    let amount = obj.get("amount").and_then(|v| v.as_str());

    Ok(QuoteRequestInput {
        settlement_symbol: settlement_symbol.into(),
        settlement_network: settlement_network.into(),
        pay_symbol: pay_symbol.into(),
        pay_network: pay_network.into(),
        amount: amount.map(|s| s.into()),
    })
}

struct QuoteRequestInput {
    settlement_symbol: String,
    settlement_network: String,
    pay_symbol: String,
    pay_network: String,
    amount: Option<String>,
}