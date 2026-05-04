//! EdgeRun Exchange API — HTTP service.
//!
//! Uses edgerun-http for the HTTP server (no axum/tokio).
//! Uses edgerun-json for JSON parsing (no serde_json).
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
use core::future::Future;
use core::pin::Pin;
use core::ptr::addr_of;
use edgerun_rt::sync::Mutex;

use edgerun_exchange::policy::RoutingPolicy;
use edgerun_exchange::provider::{ExchangeProvider, ProviderContext};
use edgerun_http::{Handler, Request, Response};

pub mod config;
mod handlers;
mod routes;
pub mod store;
pub mod types;

pub use config::Config;
pub use store::ExchangeStore;
pub use types::*;

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
        (*global_store_ptr()).as_ref().map(|s| {
            let mut guard = s.lock();
            f(&mut guard)
        })
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
