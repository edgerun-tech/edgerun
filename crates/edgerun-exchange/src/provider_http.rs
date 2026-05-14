//! Shared HTTP/JSON helpers for exchange provider adapters.

extern crate alloc;

use crate::provider::{ProviderCode, ProviderOrder, ProviderStatus};
use crate::provider_mapping::map_provider_status;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use edgerun_json::{JsonValue, Map, ToJson, to_string};
use edgerun_node::http::client_middleware::Chain;
use edgerun_node::http::{HttpClient, Method};
use edgerun_node::rt::block_on;
use edgerun_wallet::{DecimalAmount, WalletError};

pub(crate) fn build_client() -> edgerun_node::http::Client {
    Chain::new(HttpClient::new()).build()
}

pub(crate) fn parse_asset_id(id: &str) -> (String, String) {
    id.split_once(':')
        .map(|(symbol, network)| (symbol.to_string(), network.to_string()))
        .unwrap_or_else(|| (id.to_string(), String::new()))
}

pub(crate) fn call_json_api(
    client: &edgerun_node::http::Client,
    base_url: &str,
    provider_name: &str,
    method: Method,
    path: &str,
    body: Option<JsonValue>,
) -> Result<JsonValue, WalletError> {
    let url = format!("{base_url}{path}");
    let body_bytes = body
        .as_ref()
        .map(to_json_bytes)
        .transpose()?
        .unwrap_or_default();

    let response = match method {
        Method::GET => block_on(async { client.get(&url).await }),
        Method::POST => block_on(async { client.post(&url, &body_bytes).await }),
        _ => return Err(WalletError::HttpError("unsupported method".into())),
    }
    .map_err(|e| WalletError::HttpError(e.to_string()))?;

    if !response.status().is_success() {
        return Err(WalletError::ProviderError(format!(
            "{provider_name} API error: HTTP {}",
            response.status().as_u16()
        )));
    }

    let body = response.body();
    if body.is_empty() {
        return Err(WalletError::ProviderError(format!(
            "{provider_name} API returned empty response"
        )));
    }

    let body_str =
        core::str::from_utf8(body).map_err(|e| WalletError::Serialization(e.to_string()))?;
    let tape = edgerun_json::parse_json_tape(body_str)
        .map_err(|e| WalletError::Serialization(e.to_string()))?;
    tape.root(body_str)
        .and_then(|value| value.to_json_value())
        .ok_or_else(|| WalletError::Serialization("missing JSON root value".into()))
}

pub(crate) fn object<'a>(
    value: &'a JsonValue,
    invalid_message: &'static str,
) -> Result<&'a Map, WalletError> {
    value
        .as_object()
        .ok_or_else(|| WalletError::ProviderError(invalid_message.into()))
}

pub(crate) fn str_field<'a>(
    object: &'a Map,
    field: &'static str,
    missing_message: &'static str,
) -> Result<&'a str, WalletError> {
    object
        .get(field)
        .and_then(JsonValue::as_str)
        .ok_or_else(|| WalletError::ProviderError(missing_message.into()))
}

pub(crate) fn decimal(value: &str) -> Result<DecimalAmount, WalletError> {
    DecimalAmount::parse(value).ok_or_else(|| WalletError::InvalidDecimal(value.into()))
}

pub(crate) fn provider_order(
    provider: ProviderCode,
    object: &Map,
    order_id_field: &'static str,
    deposit_address_field: &'static str,
    provider_name: &'static str,
) -> Result<ProviderOrder, WalletError> {
    let order_id = str_field(
        object,
        order_id_field,
        "missing provider order id in exchange response",
    )?;
    let deposit_address = str_field(
        object,
        deposit_address_field,
        "missing deposit address in exchange response",
    )?;

    if order_id.is_empty() || deposit_address.is_empty() {
        return Err(WalletError::ProviderError(format!(
            "{provider_name} returned an empty order id or deposit address"
        )));
    }

    Ok(ProviderOrder {
        provider,
        provider_order_id: order_id.into(),
        order_id: order_id.into(),
        deposit_address: deposit_address.into(),
        status: 3,
        created_at_ms: 0,
    })
}

pub(crate) fn mapped_status(
    provider: ProviderCode,
    provider_order_id: &str,
    status_str: &str,
) -> ProviderStatus {
    ProviderStatus {
        provider,
        provider_order_id: provider_order_id.into(),
        status: map_provider_status(provider.as_str(), status_str),
        deposit_tx: None,
        payout_tx: None,
        status_detail: Some(status_str.into()),
        updated_at_ms: 0,
    }
}

fn to_json_bytes(value: &JsonValue) -> Result<Vec<u8>, WalletError> {
    to_string(value)
        .map(|json| json.into_bytes())
        .map_err(|e| WalletError::Serialization(e.to_string()))
}
