//! EdgeRun Exchange API — HTTP service.
//!
//! Uses edgerun-node http for the HTTP server (no axum/tokio).
//! Uses edgerun-json for JSON parsing (no edgerun_json).
//! Public API never exposes provider names.
//!
//! Real wiring:
//! - POST /v1/quote → route_quote across configured providers
//! - POST /v1/order → create_order via selected provider
//! - GET /v1/order/:id → event-derived status from store
//!
//! The API does not own a stream runtime. The node owns the single event stream.
//! Exchange API records `ExchangeEvent`s in its store; node-owned code may drain
//! those events and append them to the node stream using `edgerun-exchange`'s
//! stream codec.

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::ptr::addr_of;
use std::sync::Mutex;

use edgerun_exchange::policy::RoutingPolicy;
use edgerun_exchange::provider::{ExchangeProvider, ProviderContext};
use edgerun_node::http::{Handler, Request, Response};
use edgerun_sdk::runtime_api::{
    self, HttpRouteSpec, ROUTE_SCHEME_HTTPS, RuntimeAppInstall, RuntimeHttpRoute,
};

pub mod config;
mod handlers;
mod routes;
pub mod store;
pub mod types;

pub use config::Config;
pub use store::ExchangeStore;
pub use types::*;

pub const APP_SLUG: &[u8] = b"edgerun-exchange-api";
pub const APP_VERSION: &[u8] = b"0.1.0";
pub const STORAGE_NAMESPACE: &[u8] = b"edgerun-exchange-api/state";
pub const ROUTE_SPECS: &[HttpRouteSpec] = &[
    HttpRouteSpec {
        scheme: ROUTE_SCHEME_HTTPS,
        host: b"",
        path_prefix: b"/v1/quote",
    },
    HttpRouteSpec {
        scheme: ROUTE_SCHEME_HTTPS,
        host: b"",
        path_prefix: b"/v1/payment-request",
    },
    HttpRouteSpec {
        scheme: ROUTE_SCHEME_HTTPS,
        host: b"",
        path_prefix: b"/v1/order",
    },
    HttpRouteSpec {
        scheme: ROUTE_SCHEME_HTTPS,
        host: b"",
        path_prefix: b"/v1/assets",
    },
    HttpRouteSpec {
        scheme: ROUTE_SCHEME_HTTPS,
        host: b"",
        path_prefix: b"/health",
    },
];

pub fn declared_routes(developer_id: [u8; 32], host: impl AsRef<[u8]>) -> Vec<RuntimeHttpRoute> {
    let app_id = runtime_api::app_id(APP_SLUG, &developer_id);
    let manifest_sha256 = runtime_api::manifest_sha256(APP_SLUG, APP_VERSION, ROUTE_SPECS);
    let release_id = runtime_api::release_id(&app_id, APP_VERSION, &manifest_sha256);
    let host = host.as_ref().to_vec();
    ROUTE_SPECS
        .iter()
        .map(|route| {
            runtime_api::http_route(
                app_id,
                release_id,
                route.scheme,
                host.clone(),
                route.path_prefix,
            )
        })
        .collect()
}

pub fn runtime_app_install(developer_id: [u8; 32], host: impl AsRef<[u8]>) -> RuntimeAppInstall {
    let app_id = runtime_api::app_id(APP_SLUG, &developer_id);
    let manifest_sha256 = runtime_api::manifest_sha256(APP_SLUG, APP_VERSION, ROUTE_SPECS);
    let release_id = runtime_api::release_id(&app_id, APP_VERSION, &manifest_sha256);
    runtime_api::app_install(
        app_id,
        release_id,
        edgerun_sdk::sha256(APP_SLUG),
        developer_id,
        manifest_sha256,
        declared_routes(developer_id, host),
        vec![STORAGE_NAMESPACE.to_vec()],
    )
}

/// Global shared state for the exchange API.
/// Initialized via `init_global_state` before serving requests.
static mut GLOBAL_STORE: Option<Mutex<ExchangeStore>> = None;
static mut GLOBAL_PROVIDERS: Option<alloc::vec::Vec<Box<dyn ExchangeProvider>>> = None;
static mut GLOBAL_CTX: Option<ProviderContext> = None;
static mut GLOBAL_POLICY: Option<RoutingPolicy> = None;

/// Initialize the global exchange state.
/// Must be called before handling any requests.
pub fn init_global_state(
    store: ExchangeStore,
    providers: alloc::vec::Vec<Box<dyn ExchangeProvider>>,
    ctx: ProviderContext,
    policy: RoutingPolicy,
) {
    unsafe {
        GLOBAL_STORE = Some(Mutex::new(store));
        GLOBAL_PROVIDERS = Some(providers);
        GLOBAL_CTX = Some(ctx);
        GLOBAL_POLICY = Some(policy);
    }
}

#[inline(always)]
fn global_store_ptr() -> *const Option<Mutex<ExchangeStore>> {
    addr_of!(GLOBAL_STORE)
}

#[inline(always)]
fn global_providers_ptr() -> *const Option<alloc::vec::Vec<Box<dyn ExchangeProvider>>> {
    addr_of!(GLOBAL_PROVIDERS)
}

#[inline(always)]
fn global_ctx_ptr() -> *const Option<ProviderContext> {
    addr_of!(GLOBAL_CTX)
}

#[inline(always)]
fn global_policy_ptr() -> *const Option<RoutingPolicy> {
    addr_of!(GLOBAL_POLICY)
}

pub fn with_store<R>(f: impl FnOnce(&mut ExchangeStore) -> R) -> Option<R> {
    unsafe {
        let s = (*global_store_ptr()).as_ref()?;
        let mut guard = s.lock().ok()?;
        Some(f(&mut guard))
    }
}

pub fn with_providers<R>(f: impl FnOnce(&[Box<dyn ExchangeProvider>]) -> R) -> Option<R> {
    unsafe { (*global_providers_ptr()).as_ref().map(|p| f(p.as_slice())) }
}

pub fn with_ctx<R>(f: impl FnOnce(&ProviderContext) -> R) -> Option<R> {
    unsafe { (*global_ctx_ptr()).as_ref().map(|c| f(c)) }
}

pub fn with_policy<R>(f: impl FnOnce(&RoutingPolicy) -> R) -> Option<R> {
    unsafe { (*global_policy_ptr()).as_ref().map(|p| f(p)) }
}

/// Handler that dispatches to real provider-backed routes.
#[derive(Clone)]
pub struct ExchangeApiHandler;

impl Handler for ExchangeApiHandler {
    fn handle(&self, request: Request) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
        Box::pin(async move { routes::route(&request) })
    }
}

/// Build a configured handler for the exchange API.
/// Does NOT start a server — the caller is responsible for binding
/// this handler to an actual HTTP transport.
///
/// Call `init_global_state()` first to wire providers and store.
pub fn build_handler() -> ExchangeApiHandler {
    ExchangeApiHandler
}
