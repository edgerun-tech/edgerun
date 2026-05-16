//! Request routing.

extern crate alloc;

use edgerun_node::http::{Method, Request, Response, StatusCode};

use super::handlers;

pub fn route(req: &Request) -> Response {
    let path = req.uri().path();
    let method = req.method();

    // POST /v1/quote
    if *method == Method::POST && path == "/v1/quote" {
        return handlers::handle_quote(req);
    }
    // POST /v1/payment-request
    if *method == Method::POST && path == "/v1/payment-request" {
        return handlers::handle_create_payment_request(req);
    }
    // POST /v1/payment-request/:id/quote
    if *method == Method::POST
        && path.starts_with("/v1/payment-request/")
        && path.ends_with("/quote")
        && path.len() > 27
    {
        return handlers::handle_payment_request_quote(req);
    }
    // GET /v1/payment-request/:id
    if *method == Method::GET && path.starts_with("/v1/payment-request/") && path.len() > 20 {
        return handlers::handle_get_payment_request(req);
    }
    // POST /v1/order
    if *method == Method::POST && path == "/v1/order" {
        return handlers::handle_order(req);
    }
    // POST /v1/order/:id/refresh
    if *method == Method::POST
        && path.starts_with("/v1/order/")
        && path.ends_with("/refresh")
        && path.len() > 19
    {
        return handlers::handle_order_refresh(req);
    }
    // GET /v1/assets
    if *method == Method::GET && path == "/v1/assets" {
        return handlers::handle_assets(req);
    }
    // GET /health
    if *method == Method::GET && path == "/health" {
        return handlers::handle_health(req);
    }
    // GET /v1/order/:id
    if *method == Method::GET && path.starts_with("/v1/order/") && path.len() > 11 {
        return handlers::handle_order_status(req);
    }

    not_found()
}

fn not_found() -> Response {
    let mut map = edgerun_json::Map::new();
    map.insert(
        "error".into(),
        edgerun_json::JsonValue::String("not found".into()),
    );
    let body = edgerun_json::to_string(&edgerun_json::JsonValue::Object(map)).unwrap_or_default();
    let mut resp = Response::json(StatusCode::new(404).unwrap(), &body);
    resp.headers_mut()
        .insert("Content-Type", "application/json".try_into().unwrap());
    resp
}
