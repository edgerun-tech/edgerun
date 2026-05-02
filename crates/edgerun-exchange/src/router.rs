//! Provider routing — select best provider for a quote.
//!
//! Hard filters first, then score.

use alloc::vec::Vec;
use edgerun_proto::edgerun::v0::wallet::v0::QuoteRequest;
use crate::provider::{ExchangeProvider, ProviderCode, ProviderQuote};

/// Route a quote request to the best available provider.
pub fn route_quote(
    req: &QuoteRequest,
    providers: &[alloc::boxed::Box<dyn ExchangeProvider>],
) -> Option<ProviderQuote> {
    // Hard filters first
    let mut candidates: Vec<(alloc::boxed::Box<dyn ExchangeProvider>, ProviderQuote)> = Vec::new();

    for provider in providers {
        if !provider.supports_quote(req) {
            continue;
        }
        // TODO: check provider health, enabled status, etc.
        // For now, just collect
    }

    // TODO: Score and select best provider
    // For now, return None (stub)
    None
}
