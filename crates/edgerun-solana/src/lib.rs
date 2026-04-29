//! Edgerun Solana Client
//!
//! Client library for interacting with Edgerun's on-chain programs:
//! - ProviderRegistry: provider registration, collateral, reputation
//! - DeploymentContract: deployment lifecycle, per-second billing
//!
//! Uses pure Solana SDK (no Anchor).

#![no_std]

extern crate alloc;
#[cfg(target_os = "none")]
extern crate self as std;
#[cfg(not(target_os = "none"))]
extern crate std;

pub mod prelude {
    pub use alloc::format;
    pub use alloc::string::{String, ToString};
    pub use alloc::vec;
    pub use alloc::vec::Vec;
    pub use core::matches;
    pub use core::option::Option::{self, None, Some};
    pub use core::prelude::rust_2024::*;
    pub use core::result::Result::{self, Err, Ok};
    pub use core::write;
}

#[cfg(target_os = "none")]
pub mod sync {
    pub use alloc::sync::Arc;
}

#[cfg(target_os = "none")]
pub mod future {
    pub use core::future::*;
}

#[cfg(target_os = "none")]
pub mod str {
    pub use core::str::*;
}

#[cfg(target_os = "none")]
pub mod fmt {
    pub use core::fmt::*;
}

#[cfg(target_os = "none")]
pub mod io {
    use alloc::vec::Vec;

    pub type Result<T> = core::result::Result<T, Error>;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct Error;

    pub trait Write {
        fn write_all(&mut self, buf: &[u8]) -> Result<()>;
    }

    impl Write for Vec<u8> {
        fn write_all(&mut self, buf: &[u8]) -> Result<()> {
            self.extend_from_slice(buf);
            Ok(())
        }
    }
}

pub mod deployment;
pub mod error;
pub mod provider_registry;
pub mod signers;
pub mod solana_types;
pub mod types;

pub use deployment::{DeploymentClient, TokenEscrowAccounts, TokenSettlementAccounts};
pub use error::SolanaError;
pub use provider_registry::ProviderClient;
pub use types::{Deployment, DeploymentStatus, Provider, ProviderStatus};

use crate::solana_types::Pubkey;
use alloc::format;

const DEFAULT_PROVIDER_REGISTRY_PROGRAM_ID: &str = "AY54i54WDK4zrJrWVn3D6x6qZbeGHfLB8LP3T7DA2zT";
const DEFAULT_DEPLOYMENT_PROGRAM_ID: &str = "AY54i54WDK4zrJrWVn3D6x6qZbeGHfLB8LP3T7DA2zU";
const PROVIDER_REGISTRY_PROGRAM_ID_ENV: &str = "EDGERUN_PROVIDER_REGISTRY_PROGRAM_ID";
const DEPLOYMENT_PROGRAM_ID_ENV: &str = "EDGERUN_DEPLOYMENT_PROGRAM_ID";
const ALLOW_DEV_PROGRAM_IDS_ENV: &str = "EDGERUN_ALLOW_DEV_PROGRAM_IDS";

pub fn provider_registry_program_id() -> Pubkey {
    default_program_id(DEFAULT_PROVIDER_REGISTRY_PROGRAM_ID)
}

pub fn deployment_program_id() -> Pubkey {
    default_program_id(DEFAULT_DEPLOYMENT_PROGRAM_ID)
}

pub fn try_provider_registry_program_id() -> Result<Pubkey, SolanaError> {
    configured_program_id(
        PROVIDER_REGISTRY_PROGRAM_ID_ENV,
        DEFAULT_PROVIDER_REGISTRY_PROGRAM_ID,
    )
}

pub fn try_deployment_program_id() -> Result<Pubkey, SolanaError> {
    configured_program_id(DEPLOYMENT_PROGRAM_ID_ENV, DEFAULT_DEPLOYMENT_PROGRAM_ID)
}

fn default_program_id(value: &str) -> Pubkey {
    value.parse().expect("default program id must be valid")
}

fn configured_program_id(env_name: &str, default: &str) -> Result<Pubkey, SolanaError> {
    #[cfg(not(target_os = "none"))]
    {
        let configured = match std::env::var(env_name) {
            Ok(value) => Some(value),
            Err(std::env::VarError::NotPresent) => None,
            Err(err) => {
                return Err(SolanaError::Transaction(format!(
                    "failed to read {env_name}: {err}"
                )));
            }
        };
        configured_program_id_from_value(
            env_name,
            configured.as_deref(),
            default,
            allow_dev_program_ids(),
        )
    }

    #[cfg(target_os = "none")]
    {
        Ok(default_program_id(default))
    }
}

fn configured_program_id_from_value(
    env_name: &str,
    configured: Option<&str>,
    default: &str,
    allow_dev: bool,
) -> Result<Pubkey, SolanaError> {
    match configured.map(str::trim) {
        Some("") => Err(SolanaError::Transaction(format!(
            "{env_name} is set but empty"
        ))),
        Some(value) => value
            .parse()
            .map_err(|err| SolanaError::Transaction(format!("invalid {env_name}: {err}"))),
        None if allow_dev => Ok(default_program_id(default)),
        None => Err(SolanaError::Transaction(format!(
            "missing {env_name}; set it to the deployed program id, or set {ALLOW_DEV_PROGRAM_IDS_ENV}=1 to use the built-in dev id"
        ))),
    }
}

#[cfg(not(target_os = "none"))]
fn allow_dev_program_ids() -> bool {
    std::env::var(ALLOW_DEV_PROGRAM_IDS_ENV)
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use crate::solana_types::Pubkey;

    #[test]
    fn test_program_ids() {
        let _ = super::provider_registry_program_id();
        let _ = super::deployment_program_id();
    }

    #[test]
    fn default_program_ids_are_explicit_dev_values() {
        assert_eq!(
            super::DEFAULT_PROVIDER_REGISTRY_PROGRAM_ID.parse::<Pubkey>(),
            Ok(super::provider_registry_program_id())
        );
        assert_eq!(
            super::DEFAULT_DEPLOYMENT_PROGRAM_ID.parse::<Pubkey>(),
            Ok(super::deployment_program_id())
        );
    }

    #[test]
    fn checked_program_id_requires_config_unless_dev_ids_are_allowed() {
        let err = match super::configured_program_id_from_value(
            super::PROVIDER_REGISTRY_PROGRAM_ID_ENV,
            None,
            super::DEFAULT_PROVIDER_REGISTRY_PROGRAM_ID,
            false,
        )
        .unwrap_err()
        {
            crate::SolanaError::Transaction(message) => message,
            other => panic!("unexpected error: {other:?}"),
        };
        assert!(err.contains(super::PROVIDER_REGISTRY_PROGRAM_ID_ENV));
        assert!(err.contains(super::ALLOW_DEV_PROGRAM_IDS_ENV));

        assert_eq!(
            super::configured_program_id_from_value(
                super::PROVIDER_REGISTRY_PROGRAM_ID_ENV,
                None,
                super::DEFAULT_PROVIDER_REGISTRY_PROGRAM_ID,
                true,
            )
            .unwrap(),
            super::DEFAULT_PROVIDER_REGISTRY_PROGRAM_ID
                .parse::<Pubkey>()
                .unwrap()
        );
    }

    #[test]
    fn checked_program_id_rejects_empty_or_invalid_config() {
        assert!(super::configured_program_id_from_value(
            super::DEPLOYMENT_PROGRAM_ID_ENV,
            Some(" "),
            super::DEFAULT_DEPLOYMENT_PROGRAM_ID,
            true,
        )
        .is_err());
        assert!(super::configured_program_id_from_value(
            super::DEPLOYMENT_PROGRAM_ID_ENV,
            Some("not-a-pubkey"),
            super::DEFAULT_DEPLOYMENT_PROGRAM_ID,
            true,
        )
        .is_err());
    }
}
