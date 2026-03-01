// SPDX-License-Identifier: Apache-2.0
use axum::extract::{Json, Query};
use axum::http::StatusCode as AxumStatusCode;
use axum::response::Response;
use serde::Deserialize;
use sonic_rs::{JsonContainerTrait, JsonValueMutTrait, JsonValueTrait};

#[derive(Debug, Deserialize)]
pub(crate) struct LocalCloudflareVerifyRequest {
    #[serde(default)]
    token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LocalCloudflareZonesQuery {
    token: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LocalCloudflareTunnelsQuery {
    token: String,
    #[serde(default, alias = "accountId")]
    account_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LocalCloudflareAccessAppsQuery {
    token: String,
    #[serde(default, alias = "accountId")]
    account_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LocalCloudflareWorkersQuery {
    token: String,
    #[serde(default, alias = "accountId")]
    account_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LocalCloudflarePagesQuery {
    token: String,
    #[serde(default, alias = "accountId")]
    account_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LocalCloudflareDnsRecordsQuery {
    token: String,
    #[serde(default, alias = "zoneId")]
    zone_id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    record_type: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LocalCloudflareDnsUpsertRequest {
    token: String,
    #[serde(alias = "zoneId")]
    zone_id: String,
    name: String,
    content: String,
    #[serde(default, alias = "recordType")]
    record_type: Option<String>,
    #[serde(default)]
    proxied: Option<bool>,
    #[serde(default)]
    ttl: Option<u32>,
}

pub(crate) async fn handle_local_cloudflare_verify(
    Json(body): Json<LocalCloudflareVerifyRequest>,
) -> Response {
    let token = match super::cloudflare::cloudflare_token_from(body.token.as_deref().unwrap_or_default()) {
        Ok(value) => value,
        Err(err) => return crate::local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let payload = match super::cloudflare::cloudflare_api_verify_token(&token).await {
        Ok(value) => value,
        Err(err) => return crate::local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string()),
    };
    let user_payload = super::cloudflare::cloudflare_api_get_user(&token).await.ok();
    let user_email = user_payload
        .as_ref()
        .and_then(|value| value["result"]["email"].as_str())
        .unwrap_or_default();
    let user_id = user_payload
        .as_ref()
        .and_then(|value| value["result"]["id"].as_str())
        .unwrap_or_default();
    crate::local_json_value(
        AxumStatusCode::OK,
        sonic_rs::json!({
            "ok": true,
            "token_id": payload["result"]["id"].as_str().unwrap_or_default(),
            "status": payload["result"]["status"].as_str().unwrap_or_default(),
            "expires_on": payload["result"]["expires_on"].as_str().unwrap_or_default(),
            "not_before": payload["result"]["not_before"].as_str().unwrap_or_default(),
            "user_email": user_email,
            "user_id": user_id,
        }),
    )
}

pub(crate) async fn handle_local_cloudflare_zones(
    Query(query): Query<LocalCloudflareZonesQuery>,
) -> Response {
    let token = match super::cloudflare::cloudflare_token_from(&query.token) {
        Ok(value) => value,
        Err(err) => return crate::local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let payload = match super::cloudflare::cloudflare_api_request(
        &token,
        reqwest::Method::GET,
        "https://api.cloudflare.com/client/v4/zones?per_page=100",
        None,
    )
    .await
    {
        Ok(value) => value,
        Err(err) => return crate::local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string()),
    };
    let zones = payload["result"].clone();
    let count = zones.as_array().map(|items| items.len()).unwrap_or(0);
    crate::local_json_value(
        AxumStatusCode::OK,
        sonic_rs::json!({
            "ok": true,
            "zones": zones,
            "count": count,
        }),
    )
}

pub(crate) async fn handle_local_cloudflare_tunnels(
    Query(query): Query<LocalCloudflareTunnelsQuery>,
) -> Response {
    let token = match super::cloudflare::cloudflare_token_from(&query.token) {
        Ok(value) => value,
        Err(err) => return crate::local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let account_id = match super::cloudflare::cloudflare_resolve_account_id(&token, query.account_id.as_deref()).await {
        Ok(value) => value,
        Err(err) => return crate::local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string()),
    };
    let url = format!(
        "https://api.cloudflare.com/client/v4/accounts/{account_id}/cfd_tunnel?per_page=100"
    );
    let payload = match super::cloudflare::cloudflare_api_request(&token, reqwest::Method::GET, &url, None).await {
        Ok(value) => value,
        Err(err) => return crate::local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string()),
    };
    let tunnels = payload["result"].clone();
    let count = tunnels.as_array().map(|items| items.len()).unwrap_or(0);
    crate::local_json_value(
        AxumStatusCode::OK,
        sonic_rs::json!({
            "ok": true,
            "account_id": account_id,
            "tunnels": tunnels,
            "count": count,
        }),
    )
}

pub(crate) async fn handle_local_cloudflare_access_apps(
    Query(query): Query<LocalCloudflareAccessAppsQuery>,
) -> Response {
    let token = match super::cloudflare::cloudflare_token_from(&query.token) {
        Ok(value) => value,
        Err(err) => return crate::local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let account_id = match super::cloudflare::cloudflare_resolve_account_id(&token, query.account_id.as_deref()).await {
        Ok(value) => value,
        Err(err) => return crate::local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string()),
    };
    let url = format!(
        "https://api.cloudflare.com/client/v4/accounts/{account_id}/access/apps?per_page=100"
    );
    let payload = match super::cloudflare::cloudflare_api_request(&token, reqwest::Method::GET, &url, None).await {
        Ok(value) => value,
        Err(err) => return crate::local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string()),
    };
    let apps = payload["result"].clone();
    let count = apps.as_array().map(|items| items.len()).unwrap_or(0);
    crate::local_json_value(
        AxumStatusCode::OK,
        sonic_rs::json!({
            "ok": true,
            "account_id": account_id,
            "apps": apps,
            "count": count,
        }),
    )
}

pub(crate) async fn handle_local_cloudflare_workers(
    Query(query): Query<LocalCloudflareWorkersQuery>,
) -> Response {
    let token = match super::cloudflare::cloudflare_token_from(&query.token) {
        Ok(value) => value,
        Err(err) => return crate::local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let account_id = match super::cloudflare::cloudflare_resolve_account_id(&token, query.account_id.as_deref()).await {
        Ok(value) => value,
        Err(err) => return crate::local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string()),
    };
    let url = format!(
        "https://api.cloudflare.com/client/v4/accounts/{account_id}/workers/scripts"
    );
    let payload = match super::cloudflare::cloudflare_api_request(&token, reqwest::Method::GET, &url, None).await {
        Ok(value) => value,
        Err(err) => return crate::local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string()),
    };
    let workers = payload["result"].clone();
    let count = workers.as_array().map(|items| items.len()).unwrap_or(0);
    crate::local_json_value(
        AxumStatusCode::OK,
        sonic_rs::json!({
            "ok": true,
            "account_id": account_id,
            "workers": workers,
            "count": count,
        }),
    )
}

pub(crate) async fn handle_local_cloudflare_pages(
    Query(query): Query<LocalCloudflarePagesQuery>,
) -> Response {
    let token = match super::cloudflare::cloudflare_token_from(&query.token) {
        Ok(value) => value,
        Err(err) => return crate::local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let account_id = match super::cloudflare::cloudflare_resolve_account_id(&token, query.account_id.as_deref()).await {
        Ok(value) => value,
        Err(err) => return crate::local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string()),
    };
    let url = format!(
        "https://api.cloudflare.com/client/v4/accounts/{account_id}/pages/projects"
    );
    let payload = match super::cloudflare::cloudflare_api_request(&token, reqwest::Method::GET, &url, None).await {
        Ok(value) => value,
        Err(err) => return crate::local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string()),
    };
    let pages = payload["result"].clone();
    let count = pages.as_array().map(|items| items.len()).unwrap_or(0);
    crate::local_json_value(
        AxumStatusCode::OK,
        sonic_rs::json!({
            "ok": true,
            "account_id": account_id,
            "pages": pages,
            "count": count,
        }),
    )
}

pub(crate) async fn handle_local_cloudflare_dns_records(
    Query(query): Query<LocalCloudflareDnsRecordsQuery>,
) -> Response {
    let token = match super::cloudflare::cloudflare_token_from(&query.token) {
        Ok(value) => value,
        Err(err) => return crate::local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let zone_id = query.zone_id.unwrap_or_default().trim().to_string();
    if zone_id.is_empty() {
        return crate::local_json_error(AxumStatusCode::BAD_REQUEST, "zone_id is required");
    }
    let mut url = format!("https://api.cloudflare.com/client/v4/zones/{}/dns_records", zone_id);
    crate::append_query_pair(
        &mut url,
        "per_page",
        &query.limit.unwrap_or(100).clamp(1, 200).to_string(),
    );
    crate::append_query_pair(&mut url, "name", query.name.as_deref().unwrap_or_default());
    crate::append_query_pair(
        &mut url,
        "type",
        query.record_type.as_deref().unwrap_or_default(),
    );
    let payload = match super::cloudflare::cloudflare_api_request(&token, reqwest::Method::GET, &url, None).await {
        Ok(value) => value,
        Err(err) => return crate::local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string()),
    };
    let records = payload["result"].clone();
    let count = records.as_array().map(|items| items.len()).unwrap_or(0);
    crate::local_json_value(
        AxumStatusCode::OK,
        sonic_rs::json!({
            "ok": true,
            "zone_id": zone_id,
            "records": records,
            "count": count,
        }),
    )
}

pub(crate) async fn handle_local_cloudflare_dns_upsert(
    Json(body): Json<LocalCloudflareDnsUpsertRequest>,
) -> Response {
    let token = match super::cloudflare::cloudflare_token_from(&body.token) {
        Ok(value) => value,
        Err(err) => return crate::local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let zone_id = body.zone_id.trim();
    if zone_id.is_empty() {
        return crate::local_json_error(AxumStatusCode::BAD_REQUEST, "zone_id is required");
    }
    let name = body.name.trim();
    if name.is_empty() {
        return crate::local_json_error(AxumStatusCode::BAD_REQUEST, "name is required");
    }
    let content = body.content.trim();
    if content.is_empty() {
        return crate::local_json_error(AxumStatusCode::BAD_REQUEST, "content is required");
    }
    let record_type = body
        .record_type
        .as_deref()
        .unwrap_or("CNAME")
        .trim()
        .to_ascii_uppercase();
    if record_type.is_empty() {
        return crate::local_json_error(AxumStatusCode::BAD_REQUEST, "record_type is required");
    }
    let supports_proxy = matches!(record_type.as_str(), "A" | "AAAA" | "CNAME");
    let ttl = body.ttl.unwrap_or(1).clamp(1, 86_400);

    let mut existing_url = format!("https://api.cloudflare.com/client/v4/zones/{zone_id}/dns_records");
    crate::append_query_pair(&mut existing_url, "name", name);
    crate::append_query_pair(&mut existing_url, "type", &record_type);
    crate::append_query_pair(&mut existing_url, "per_page", "1");

    let existing_payload = match super::cloudflare::cloudflare_api_request(
        &token,
        reqwest::Method::GET,
        &existing_url,
        None,
    )
    .await
    {
        Ok(value) => value,
        Err(err) => return crate::local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string()),
    };
    let existing_id = existing_payload["result"]
        .as_array()
        .and_then(|entries| entries.first())
        .and_then(|entry| entry["id"].as_str())
        .unwrap_or_default()
        .trim()
        .to_string();

    let mut payload = sonic_rs::json!({
        "type": record_type,
        "name": name,
        "content": content,
        "ttl": ttl,
    });
    if supports_proxy {
        let proxied = body.proxied.unwrap_or(false);
        if let Some(object) = payload.as_object_mut() {
            object.insert("proxied", sonic_rs::json!(proxied));
        }
    }

    let (method, url, action) = if existing_id.is_empty() {
        (
            reqwest::Method::POST,
            format!("https://api.cloudflare.com/client/v4/zones/{zone_id}/dns_records"),
            "created",
        )
    } else {
        (
            reqwest::Method::PUT,
            format!(
                "https://api.cloudflare.com/client/v4/zones/{zone_id}/dns_records/{existing_id}"
            ),
            "updated",
        )
    };
    let result = match super::cloudflare::cloudflare_api_request(&token, method, &url, Some(payload)).await {
        Ok(value) => value,
        Err(err) => return crate::local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string()),
    };
    crate::local_json_value(
        AxumStatusCode::OK,
        sonic_rs::json!({
            "ok": true,
            "zone_id": zone_id,
            "action": action,
            "record": result["result"],
        }),
    )
}
