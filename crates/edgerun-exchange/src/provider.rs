//! Exchange provider abstraction.
//!
//! Defines the `ExchangeProvider` trait and canonical types.
//! Uses edgerun-node http for HTTP calls, edgerun-json for JSON parsing.
//! No external dependencies.

extern crate alloc;

use alloc::vec::Vec;
use edgerun_protocols::core_protocol::protocol::edgerun_wallet_v0::{
    AssetRef, FeeBreakdown, Quote, QuoteRequest, TxRef,
};
use edgerun_wallet::{DecimalAmount, WalletError};

/// Internal provider code (never in public API).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderCode {
    SideShift,
    ChangeNOW,
    FFio,
}

impl ProviderCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderCode::SideShift => "SIDESHIFT",
            ProviderCode::ChangeNOW => "CHANGENOW",
            ProviderCode::FFio => "FFIO",
        }
    }
}

/// Provider capability/feature flags.
#[derive(Debug, Clone)]
pub struct ProviderFeatures {
    pub supports_fixed_rate: bool,
    pub supports_float_rate: bool,
    pub supports_refund_address: bool,
    pub requires_destination_tag: bool,
    pub min_confirmations: u32,
}

/// Canonical quote request sent to providers.
#[derive(Debug, Clone)]
pub struct ProviderQuoteRequest {
    pub settlement_asset: AssetRef,
    pub pay_asset: AssetRef,
    pub settlement_amount: Option<DecimalAmount>,
    pub pay_amount: Option<DecimalAmount>,
    pub quote_mode: i32, // QuoteMode as i32
    pub refund_address: Option<alloc::string::String>,
}

/// Canonical quote response from providers.
#[derive(Debug, Clone)]
pub struct ProviderQuote {
    pub provider: ProviderCode,
    pub quote_id_internal: alloc::string::String,
    pub settlement_asset: AssetRef,
    pub pay_asset: AssetRef,
    pub settlement_amount: DecimalAmount,
    pub pay_amount: DecimalAmount,
    pub rate: DecimalAmount, // settlement per pay unit
    pub quote_mode: i32,     // QuoteMode as i32
    pub expires_at_ms: u64,
    pub estimated_seconds: Option<u64>,
    pub fees: Option<FeeBreakdown>,
}

/// Canonical order creation request.
#[derive(Debug, Clone)]
pub struct ProviderOrderRequest {
    pub quote_id_internal: alloc::string::String,
    pub recipient_address: alloc::string::String,
    pub refund_address: Option<alloc::string::String>,
}

/// Canonical order response from providers.
#[derive(Debug, Clone)]
pub struct ProviderOrder {
    pub provider: ProviderCode,
    pub provider_order_id: alloc::string::String, // internal only
    pub order_id: alloc::string::String,
    pub deposit_address: alloc::string::String,
    pub status: i32, // CanonicalOrderStatus as i32
    pub created_at_ms: u64,
}

/// Canonical status check response.
#[derive(Debug, Clone)]
pub struct ProviderStatus {
    pub provider: ProviderCode,
    pub provider_order_id: alloc::string::String,
    pub status: i32, // CanonicalOrderStatus as i32
    pub deposit_tx: Option<TxRef>,
    pub payout_tx: Option<TxRef>,
    pub status_detail: Option<alloc::string::String>,
    pub updated_at_ms: u64,
}

/// Exchange provider trait (no async_trait dependency).
pub trait ExchangeProvider {
    /// Returns the provider code (internal only).
    fn code(&self) -> ProviderCode;

    /// Returns provider features.
    fn features(&self) -> ProviderFeatures;

    /// Checks if provider supports this asset pair and quote mode.
    fn supports_quote(&self, req: &QuoteRequest) -> bool;

    /// Gets a quote from the provider.
    fn quote(
        &self,
        req: &QuoteRequest,
        ctx: &ProviderContext,
    ) -> core::result::Result<ProviderQuote, WalletError>;

    /// Creates an order with the provider.
    fn create_order(
        &self,
        quote: &Quote,
        req: &ProviderOrderRequest,
        ctx: &ProviderContext,
    ) -> core::result::Result<ProviderOrder, WalletError>;

    /// Gets the current status of an order.
    fn get_order_status(
        &self,
        provider_order_id: &str,
        ctx: &ProviderContext,
    ) -> core::result::Result<ProviderStatus, WalletError>;
}

/// Context passed to provider calls (HTTP client, timeout, etc.).
#[derive(Clone)]
pub struct ProviderContext {
    #[cfg(feature = "provider-http")]
    pub http_client: alloc::rc::Rc<core::cell::RefCell<edgerun_node::http::Client>>,
    pub timeout_ms: u64,
}

impl core::fmt::Debug for ProviderContext {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "ProviderContext {{ timeout_ms: {} }}", self.timeout_ms)
    }
}
