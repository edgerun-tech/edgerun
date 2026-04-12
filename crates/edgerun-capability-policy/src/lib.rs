//! Policy engine for capability authorization.
//!
//! Evaluates capability requests against constraints, manages grant lifecycle,
//! and handles revocation.

mod types;
mod trait_def;
mod engine;
mod policy_engine;
mod helpers;

pub use types::{PolicyContext, PolicyDecision, RevocationReason, GrantRecord};
pub use trait_def::PolicyEngine;
pub use engine::SimplePolicyEngine;

#[cfg(test)]
mod tests;
