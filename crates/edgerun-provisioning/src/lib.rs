// EdgeRun bootstrap provisioning
// Uses edgerun-proto generated types - no parallel types
// Single-use contracts, node genesis, controller verification

#![forbid(unsafe_code)]

pub mod contract;
pub mod errors;
pub mod genesis;
pub mod state;
pub mod verifier;

// Re-exports using edgerun-proto types
pub use contract::ProvisioningContract;
pub use errors::ProvisioningError;
pub use genesis::NodeGenesisClaim;
pub use state::ProvisioningState;
pub use verifier::ProvisioningVerifier;
