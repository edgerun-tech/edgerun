// EdgeRun bootstrap provisioning
// Single-use contracts, node genesis, controller verification
// Coordinator is NOT authority — only assists/observes

// This crate is responsible for:
// - Building signable provisioning contract payloads
// - Verifying controller signatures on contracts
// - Computing contract hashes
// - Building node genesis claims
// - Verifying node genesis claims
// - Enforcing single-use state machine
// - Rejecting replayed/expired contracts
// - Producing dashboard-friendly status

#![forbid(unsafe_code)]

pub mod contract;
pub mod genesis;
pub mod verifier;
pub mod state;
pub mod errors;

// Re-exports
pub use contract::ProvisioningContract;
pub use genesis::NodeGenesisClaim;
pub use verifier::ProvisioningVerifier;
pub use state::ProvisioningState;
pub use errors::ProvisioningError;
