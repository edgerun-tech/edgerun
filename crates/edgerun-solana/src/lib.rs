//! Edgerun Solana Client
//!
//! Client library for interacting with Edgerun's on-chain programs:
//! - ProviderRegistry: provider registration, collateral, reputation
//! - DeploymentContract: deployment lifecycle, per-second billing
//!
//! Uses pure Solana SDK (no Anchor).

#![no_std]

extern crate alloc;
#[cfg(not(target_os = "none"))]
extern crate std;
#[cfg(target_os = "none")]
extern crate self as std;

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

pub use deployment::DeploymentClient;
pub use error::SolanaError;
pub use provider_registry::ProviderClient;
pub use types::{Deployment, DeploymentStatus, Provider, ProviderStatus};

use crate::solana_types::Pubkey;

const PROVIDER_REGISTRY_PROGRAM_ID: &str = " AY54i54WDK4zrJrWVn3D6x6qZbeGHfLB8LP3T7DA2zT"; // fake test
const DEPLOYMENT_PROGRAM_ID: &str = " AY54i54WDK4zrJrWVn3D6x6qZbeGHfLB8LP3T7DA2zU"; // fake test

pub fn provider_registry_program_id() -> Pubkey {
    PROVIDER_REGISTRY_PROGRAM_ID.trim().parse().unwrap()
}

pub fn deployment_program_id() -> Pubkey {
    DEPLOYMENT_PROGRAM_ID.trim().parse().unwrap()
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_program_ids() {
        let _ = super::provider_registry_program_id();
        let _ = super::deployment_program_id();
    }
}
