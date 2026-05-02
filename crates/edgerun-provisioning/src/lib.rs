// EdgeRun bootstrap provisioning
// Uses edgerun-proto generated types - no parallel types
// Single-use contracts, node genesis, controller verification

#![forbid(unsafe_code)]

pub mod contract;
pub mod genesis;
pub mod verifier;
pub mod state;
pub mod errors;

// Re-exports using edgerun-proto types
pub use contract::ProvisioningContract;
pub use genesis::NodeGenesisClaim;
pub use verifier::ProvisioningVerifier;
pub use state::ProvisioningState;
pub use errors::ProvisioningError;
