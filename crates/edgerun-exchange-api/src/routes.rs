//! Request routing.

extern crate alloc;

use edgerun_http::{Method, Request, Response, StatusCode};

use super::handlers;

pub fn route(req: &Request) -> Response {
    let path = req.uri().path();
    let method = req.method();

    // POST /v1/quote
    if *method == Method::POST && path == "/v1/quote" {
        return handlers::handle_quote(req);
    }
    // POST /v1/order
    if *method == Method::POST && path == "/v1/order" {
        return handlers::handle_order(req);
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
