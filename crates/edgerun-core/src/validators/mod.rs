//! Protocol-level validators for the edgerun core protocol.
mod helpers;
mod reachability;
mod event_semantics;
mod canonical;
mod crypto;
mod trust;
mod delegation;
mod command;
mod query;
mod control;
mod network;
mod stream;
mod snapshot;
mod object;
mod proof;

pub use canonical::validate_canonical_case;
pub use crypto::validate_crypto_case;
pub use trust::validate_trust_case;
pub use delegation::validate_delegation_case;
pub use command::validate_command_case;
pub use query::validate_query_case;
pub use control::validate_control_change_case;
pub use network::validate_network_case;
pub use stream::validate_stream_append_case;
pub use snapshot::validate_snapshot_case;
pub use object::validate_object_case;
pub use proof::{
    validate_snapshot_set_proof, validate_event_set_proof,
    validate_object_assertion_proof, validate_aggregate_summary_proof,
    validate_trust_policy_proof, ProofStructuralResult,
};
pub use helpers::FixtureVerifier;

#[cfg(test)]
mod tests;
