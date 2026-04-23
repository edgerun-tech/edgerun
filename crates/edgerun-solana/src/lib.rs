//! Edgerun Solana Client
//! 
//! Client library for interacting with Edgerun's on-chain programs:
//! - ProviderRegistry: provider registration, collateral, reputation
//! - DeploymentContract: deployment lifecycle, per-second billing
//!
//! Uses pure Solana SDK (no Anchor).

pub mod provider_registry;
pub mod deployment;
pub mod error;
pub mod types;
pub mod solana_types;
pub mod signers;

pub use error::SolanaError;
pub use provider_registry::ProviderClient;
pub use deployment::DeploymentClient;
pub use types::{Provider, Deployment, ProviderStatus, DeploymentStatus};

use crate::solana_types::Pubkey;

const PROVIDER_REGISTRY_PROGRAM_ID: &str = "EgRPRoGiVa7pBq7f9VTvJfJqLQKxVTuKpPQqM8dFLPer";
const DEPLOYMENT_PROGRAM_ID: &str = "DePLoYMtGaqDqLxU1vA3KqLQKxVTuKpPQqM8dFLPer";

pub fn provider_registry_program_id() -> Pubkey {
    PROVIDER_REGISTRY_PROGRAM_ID.parse().unwrap()
}

pub fn deployment_program_id() -> Pubkey {
    DEPLOYMENT_PROGRAM_ID.parse().unwrap()
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_program_ids() {
        let _ = super::provider_registry_program_id();
        let _ = super::deployment_program_id();
    }
}