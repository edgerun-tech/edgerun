//! Exchange API handlers — wired to real provider routing.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use edgerun_http::{Request, Response, StatusCode};
use edgerun_json::{from_json_slice, from_slice, to_string, JsonValue, Map};
use edgerun_proto::edgerun::v0::wallet::v0::{Quote, QuoteRequest};

use crate::store::ExchangeQuoteId;
use crate::types::*;
use crate::{
    append_exchange_events_to_stream, project_exchange_order_from_stream, with_ctx, with_policy,
    with_providers, with_store,
};

fn now_epoch_millis() -> u64 {
    #[cfg(feature = "std")]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        return SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
    }

    #[cfg(not(feature = "std"))]
    {
        0
    }
}

/// POST /v1/quote — routes to best provider via route_quote.
pub fn handle_quote(req: &Request) -> Response {
    let body = match req.body() {
        Some(b) => b,
        None => return json_error(400, "missing request body"),
    };

    if body.is_empty() {
        return json_error(400, "missing request body");
    }

    let json: JsonValue = match from_slice(body) {
        Ok(j) => j,
        Err(e) => return json_error(400, &alloc::format!("invalid JSON: {}", e)),
    };

    let input = match QuoteInput::from_json_value(&json) {
        Ok(i) => i,
        Err(e) => return json_error(400, &e),
    };

    let proto_req = QuoteRequest {
        settlement_asset_id: input.settlement_asset_id.clone(),
        pay_asset_id: input.pay_asset_id.clone(),
        settlement_amount: input.amount.clone().unwrap_or_default(),
        pay_amount: "".into(),
        quote_mode: input.mode.unwrap_or(1),
        amount_side: input.amount_side.unwrap_or(1),
        refund_address: input.refund_address.clone(),
        recipient_address: input.recipient_address.clone(),
    };

    let routing_result = with_providers(|providers| {
        with_ctx(|ctx| with_policy(|policy| edgerun_exchange::route_quote(&proto_req, providers, ctx, policy)))
            .flatten()
    })
    .flatten();

    let routing_result = match routing_result {
        Some(r) => r,
        None => return json_error(500, "exchange not initialized"),
    };

    match routing_result {
        edgerun_exchange::router::QuoteRoutingResult::Quote(provider_quote) => {
            let result = with_store(|store| {
                let before = store.event_count();
                let quote_id = store.store_quote(&provider_quote);
                let events = store.events_from(before).to_vec();
                (quote_id, events)
            });

            let Some((quote_id, events)) = result else {
                return json_error(500, "store not available");
            };

            if let Err(e) = append_exchange_events_to_stream(&events) {
                return json_error(500, &e);
            }

            let mut response = Map::new();
            response.insert("id".into(), JsonValue::String(quote_id.as_str().into()));
            response.insert(
                "settlement_asset".into(),
                JsonValue::String(alloc::format!(
                    "{}:{}",
                    provider_quote.settlement_asset.symbol,
                    provider_quote.settlement_asset.network
                )),
            );
            response.insert(
                "pay_asset".into(),
                JsonValue::String(alloc::format!(
                    "{}:{}",
                    provider_quote.pay_asset.symbol,
                    provider_quote.pay_asset.network
                )),
            );
            response.insert(
                "settlement_amount".into(),
                JsonValue::String(provider_quote.settlement_amount.to_string()),
            );
            response.insert(
                "pay_amount".into(),
                JsonValue::String(provider_quote.pay_amount.to_string()),
            );
            response.insert("rate".into(), JsonValue::String(provider_quote.rate.to_string()));
            response.insert("expires_at_ms".into(), JsonValue::Number(provider_quote.expires_at_ms.into()));
            if let Some(est) = provider_quote.estimated_seconds {
                response.insert("estimated_seconds".into(), JsonValue::Number(est.into()));
            }

            json_response(200, JsonValue::Object(response))
        }
        edgerun_exchange::router::QuoteRoutingResult::NoProviderAvailable => {
            json_error(404, "no provider available for this asset pair")
        }
        edgerun_exchange::router::QuoteRoutingResult::AllProvidersFailed { errors } => {
            let mut response = Map::new();
            response.insert("error".into(), JsonValue::String("all providers failed".into()));
            response.insert(
                "errors".into(),
                JsonValue::Array(errors.into_iter().map(JsonValue::String).collect()),
            );
            json_response(502, JsonValue::Object(response))
        }
    }
}

/// POST /v1/order — creates an order from a stored quote using the same provider that created it.
pub fn handle_order(req: &Request) -> Response {
    let body = match req.body() {
        Some(b) => b,
        None => return json_error(400, "missing request body"),
    };

    if body.is_empty() {
        return json_error(400, "missing request body");
    }

    let input = match from_json_slice::<OrderInput>(body) {
        Ok(i) => i,
        Err(e) => return json_error(400, &alloc::format!("invalid JSON: {}", e)),
    };

    let exchange_quote_id = match ExchangeQuoteId::from_existing(&input.quote_id) {
        Some(id) => id,
        None => return json_error(400, "invalid quote_id"),
    };

    let stored_quote = with_store(|store| store.get_quote(exchange_quote_id.as_str()).cloned());

    let stored_quote = match stored_quote {
        Some(Some(q)) => q,
        Some(None) => return json_error(404, "quote not found"),
        None => return json_error(500, "store not available"),
    };

    let now_ms = now_epoch_millis();
    if now_ms > 0 && stored_quote.expires_at_ms <= now_ms {
        return json_error(409, "quote expired");
    }

    let quote_proto = Quote {
        quote_id: stored_quote.quote_id_internal.clone(),
        settlement_asset_id: stored_quote.settlement_asset_id.clone(),
        pay_asset_id: stored_quote.pay_asset_id.clone(),
        settlement_amount: stored_quote.settlement_amount.clone(),
        pay_amount: stored_quote.pay_amount.clone(),
        rate: stored_quote.rate.clone(),
        quote_mode: stored_quote.quote_mode,
        expires_at_ms: stored_quote.expires_at_ms,
        fees: None,
        estimated_seconds: None,
        provider_code_internal: Some(stored_quote.provider_code.clone()),
    };

    let provider_order_req = edgerun_exchange::provider::ProviderOrderRequest {
        quote_id_internal: stored_quote.quote_id_internal.clone(),
        recipient_address: input.destination.clone(),
        refund_address: input.refund_address.clone(),
    };

    let order_result = with_providers(|providers| {
        with_ctx(|ctx| {
            let provider = providers
                .iter()
                .find(|provider| provider.code().as_str() == stored_quote.provider_code.as_str())?;
            provider
                .create_order(&quote_proto, &provider_order_req, ctx)
                .ok()
                .map(|provider_order| (provider_order, provider.code().as_str().to_string()))
        })
        .flatten()
    })
    .flatten();

    let Some((provider_order, provider_code)) = order_result else {
        return json_error(502, "selected quote provider failed to create order");
    };

    let deposit_addr = provider_order.deposit_address.clone();
    let provider_status = provider_order.status;
    let result = with_store(|store| {
        let before = store.event_count();
        let order_id = store.store_order(
            &exchange_quote_id,
            provider_order.provider_order_id,
            provider_order.deposit_address,
            input.destination,
            input.refund_address,
            provider_code,
            provider_order.status,
            provider_order.created_at_ms,
        );
        let events = store.events_from(before).to_vec();
        (order_id, events)
    });

    let Some((Some(order_id), events)) = result else {
        return json_error(500, "failed to store order");
    };

    if let Err(e) = append_exchange_events_to_stream(&events) {
        return json_error(500, &e);
    }

    let status_str = canonical_status_to_str(provider_status);
    let mut response = Map::new();
    response.insert("id".into(), JsonValue::String(order_id.as_str().into()));
    response.insert("status".into(), JsonValue::String(status_str.into()));
    response.insert("deposit_address".into(), JsonValue::String(deposit_addr));
    response.insert(
        "settlement_asset".into(),
        JsonValue::String(stored_quote.settlement_asset_id.clone()),
    );
    response.insert(
        "settlement_amount".into(),
        JsonValue::String(stored_quote.settlement_amount.clone()),
    );
    response.insert(
        "pay_amount".into(),
        JsonValue::String(stored_quote.pay_amount.clone()),
    );
    json_response(201, JsonValue::Object(response))
}

/// GET /v1/order/:id — returns event-derived order status.
pub fn handle_order_status(req: &Request) -> Response {
    let path = req.uri().path();
    let order_id = &path[11..];

    let result = with_store(|store| {
        let order = store.get_order(order_id).cloned();
        let stream_projection = match project_exchange_order_from_stream(order_id) {
            Ok(projection) => projection,
            Err(e) => {
                edgerun_log::warn!("exchange stream projection failed: {}", e);
                None
            }
        };
        let projection = stream_projection.or_else(|| store.project_order(order_id));
        (order, projection)
    });

    match result {
        Some((Some(order), Some(projection))) => {
            let status_str = canonical_status_to_str(projection.canonical_status);

            let mut response = Map::new();
            response.insert("id".into(), JsonValue::String(order_id.into()));
            response.insert("status".into(), JsonValue::String(status_str.into()));
            response.insert("canonical_status".into(), JsonValue::Number((projection.canonical_status as i64).into()));
            response.insert("deposit_address".into(), JsonValue::String(order.deposit_address));
            response.insert("settlement_amount".into(), JsonValue::String(order.settlement_amount));
            response.insert("pay_amount".into(), JsonValue::String(order.pay_amount));
            response.insert("event_count".into(), JsonValue::Number((projection.event_count as u64).into()));
            response.insert("terminal".into(), JsonValue::Bool(projection.terminal));
            response.insert("manual_review_required".into(), JsonValue::Bool(projection.manual_review_required));
            if let Some(status) = projection.latest_provider_status {
                response.insert("latest_provider_status".into(), JsonValue::Number((status as i64).into()));
            }
            if let Some(detail) = projection.latest_provider_status_string {
                response.insert("latest_provider_status_detail".into(), JsonValue::String(detail));
            }
            if let Some(event_type) = projection.last_event_type {
                response.insert("last_event_type".into(), JsonValue::String(event_type.into()));
            }
            if let Some(updated_at_ms) = projection.updated_at_ms {
                response.insert("updated_at_ms".into(), JsonValue::Number(updated_at_ms.into()));
            }
            json_response(200, JsonValue::Object(response))
        }
        Some((Some(_), None)) => json_error(500, "order projection unavailable"),
        Some((None, _)) => json_error(404, "order not found"),
        None => json_error(500, "store not available"),
    }
}

/// GET /v1/assets — static supported-assets catalog.
pub fn handle_assets(req: &Request) -> Response {
    let _ = req;
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
        "This is a static catalog of supported asset types. Actual pair availability depends on provider support and is determined at quote time.".into(),
    ));

    json_response(200, JsonValue::Object(response))
}

/// GET /health — reports degraded status because provider health checks are not yet implemented.
pub fn handle_health(req: &Request) -> Response {
    let _ = req;
    let mut providers = Map::new();
    providers.insert("SIDESHIFT".into(), JsonValue::String("not_implemented".into()));
    providers.insert("CHANGENOW".into(), JsonValue::String("not_implemented".into()));
    providers.insert("FFIO".into(), JsonValue::String("not_implemented".into()));

    let mut status = Map::new();
    status.insert("status".into(), JsonValue::String("degraded".into()));
    status.insert(
        "reason".into(),
        JsonValue::String("Provider health checks not yet implemented".into()),
    );
    status.insert("providers".into(), JsonValue::Object(providers));

    json_response(200, JsonValue::Object(status))
}

fn canonical_status_to_str(status: i32) -> &'static str {
    match status {
        1 => "quoted",
        2 => "quote_expired",
        3 => "created",
        4 => "waiting_for_deposit",
        5 => "deposit_seen",
        6 => "deposit_confirmed",
        7 => "exchanging",
        8 => "sending",
        9 => "completed",
        10 => "action_required",
        11 => "refund_required",
        12 => "refunding",
        13 => "refunded",
        14 => "expired",
        15 => "failed",
        16 => "rejected",
        17 => "on_hold",
        18 => "canceled",
        19 => "partial_deposits",
        _ => "unknown",
    }
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

// ── Input types ────────────────────────────────────────────────────────────

/// Input for POST /v1/quote
#[derive(Debug)]
struct QuoteInput {
    settlement_asset_id: String,
    pay_asset_id: String,
    amount: Option<String>,
    mode: Option<i32>,
    amount_side: Option<i32>,
    refund_address: Option<String>,
    recipient_address: Option<String>,
}

impl QuoteInput {
    fn from_json_value(json: &JsonValue) -> Result<Self, String> {
        let obj = json.as_object().ok_or("expected JSON object")?;
        let settlement = obj.get("settlement").ok_or("missing 'settlement'")?;
        let settlement_obj = settlement.as_object().ok_or("'settlement' must be object")?;
        let settlement_symbol = settlement_obj
            .get("symbol")
            .and_then(|v| v.as_str())
            .ok_or("missing settlement.symbol")?;
        let settlement_network = settlement_obj
            .get("network")
            .and_then(|v| v.as_str())
            .ok_or("missing settlement.network")?;

        let pay = obj.get("pay").ok_or("missing 'pay'")?;
        let pay_obj = pay.as_object().ok_or("'pay' must be object")?;
        let pay_symbol = pay_obj
            .get("symbol")
            .and_then(|v| v.as_str())
            .ok_or("missing pay.symbol")?;
        let pay_network = pay_obj
            .get("network")
            .and_then(|v| v.as_str())
            .ok_or("missing pay.network")?;

        let amount = obj.get("amount").and_then(|v| v.as_str()).map(String::from);
        let mode = obj.get("mode").and_then(|v| v.as_str()).map(|s| match s {
            "instant" => 1,
            "floating" => 2,
            _ => 1,
        });
        let amount_side = obj.get("amount_side").and_then(|v| v.as_str()).map(|s| match s {
            "settlement" => 1,
            "pay" => 2,
            _ => 1,
        });
        let refund_address = obj.get("refund_address").and_then(|v| v.as_str()).map(String::from);
        let recipient_address = obj.get("recipient_address").and_then(|v| v.as_str()).map(String::from);

        Ok(QuoteInput {
            settlement_asset_id: alloc::format!(
                "{}:{}",
                settlement_symbol.to_uppercase(),
                settlement_network.to_lowercase()
            ),
            pay_asset_id: alloc::format!(
                "{}:{}",
                pay_symbol.to_uppercase(),
                pay_network.to_lowercase()
            ),
            amount,
            mode,
            amount_side,
            refund_address,
            recipient_address,
        })
    }
}

/// Input for POST /v1/order
#[derive(Debug)]
#[allow(dead_code)]
struct OrderInput {
    quote_id: String,
    destination: String,
    refund_address: Option<String>,
}

impl edgerun_json::FromJson for OrderInput {
    fn from_json(value: JsonValue) -> Result<Self, edgerun_json::JsonValueError> {
        let obj = match value {
            JsonValue::Object(o) => o,
            _ => return Err(edgerun_json::JsonValueError::WrongType("expected object".into())),
        };
        let quote_id = obj
            .get("quote_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| edgerun_json::JsonValueError::WrongType("missing quote_id".into()))?
            .to_string();
        let destination = obj
            .get("destination")
            .and_then(|v| v.as_str())
            .ok_or_else(|| edgerun_json::JsonValueError::WrongType("missing destination".into()))?
            .to_string();
        let refund_address = obj.get("refund_address").and_then(|v| v.as_str()).map(String::from);
        Ok(OrderInput { quote_id, destination, refund_address })
    }
}
