//! Policy engine for capability authorization.
//!
//! Evaluates capability requests against constraints, manages grant lifecycle,
//! and handles revocation.

#[path = "std.rs"]
mod std_compat;
pub use std_compat::*;

mod engine;
mod helpers;
mod policy_engine;
mod trait_def;
mod types;

pub use engine::SimplePolicyEngine;
pub use trait_def::PolicyEngine;
pub use types::{GrantRecord, PolicyContext, PolicyDecision, RevocationReason};

#[cfg(test)]
mod tests;
