//! Provider routing — select best provider for a quote.
//!
//! Hard filters first, then score by rate/fees/expiry/provider health.
//! Never exposes provider_code publicly; that is internal only.

extern crate alloc;

use alloc::vec::Vec;
use edgerun_proto::edgerun::v0::wallet::v0::QuoteRequest;
use edgerun_wallet::WalletError;
use crate::provider::{ExchangeProvider, ProviderContext, ProviderQuote};
use crate::policy::RoutingPolicy;

/// Result of routing a quote request.
#[derive(Debug)]
pub enum QuoteRoutingResult {
    /// Best provider found.
    Quote(ProviderQuote),
    /// No provider supports this asset pair / mode.
    NoProviderAvailable,
    /// All supporting providers failed during quote.
    AllProvidersFailed { errors: Vec<alloc::string::String> },
}

/// Route a quote request to the best available provider.
///
/// Pipeline:
/// 1. Hard-filter by supports_quote (asset pair, mode)
/// 2. Call provider.quote() for each candidate
/// 3. Score by rate, fees, expiry, health
/// 4. Return best quote or explicit error
pub fn route_quote(
    req: &QuoteRequest,
    providers: &[alloc::boxed::Box<dyn ExchangeProvider>],
    ctx: &ProviderContext,
    policy: &RoutingPolicy,
) -> QuoteRoutingResult {
    // Step 1: Hard filter — which providers support this asset pair?
    let mut errors: Vec<alloc::string::String> = Vec::new();

    let candidates: Vec<ProviderQuote> = providers
        .iter()
        .filter(|p| p.supports_quote(req))
        .filter_map(|provider| {
            match provider.quote(req, ctx) {
                Ok(q) => Some(q),
                Err(e) => {
                    errors.push(alloc::format!(
                        "{}: {}",
                        provider.code().as_str(),
                        e
                    ));
                    None
                }
            }
        })
        .collect();

    if candidates.is_empty() {
        if errors.is_empty() {
            return QuoteRoutingResult::NoProviderAvailable;
        }
        return QuoteRoutingResult::AllProvidersFailed { errors };
    }

    // Step 2: Score and select best
    let best = score_and_select(candidates, policy);
    QuoteRoutingResult::Quote(best)
}

/// Score candidates and return the best one.
/// Scoring: higher rate is better, lower fees is better,
/// longer expiry window is better.
fn score_and_select(mut candidates: Vec<ProviderQuote>, _policy: &RoutingPolicy) -> ProviderQuote {
    // For now: sort by rate descending (higher settlement per pay unit = better)
    // TODO: incorporate fee weighting, expiry confidence, provider health
    candidates.sort_by(|a, b| {
        // Compare rates — higher is better for the user
        b.rate.cmp(&a.rate)
    });

    candidates.into_iter().next().expect("non-empty candidates")
}
