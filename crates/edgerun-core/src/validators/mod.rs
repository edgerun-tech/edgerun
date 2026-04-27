//! Protocol-level validators for the edgerun core protocol.
use crate::prelude::v1::*;

mod canonical;
mod command;
mod control;
mod crypto;
mod delegation;
mod event_semantics;
mod helpers;
mod identity;
mod network;
mod object;
mod proof;
mod query;
mod reachability;
mod snapshot;
mod stream;
mod trust;

pub use canonical::validate_canonical_case;
pub use command::validate_command_case;
pub use control::validate_control_change_case;
pub use crypto::validate_crypto_case;
pub use delegation::validate_delegation_case;
pub use helpers::FixtureVerifier;
pub use identity::validate_identity_record;
pub use network::{validate_network_case, validate_route_advertisement};
pub use object::validate_object_case;
pub use proof::{
    validate_aggregate_summary_proof, validate_event_set_proof,
    validate_federated_aggregate_descriptor, validate_object_assertion_proof,
    validate_proof_bundle, validate_snapshot_set_proof, validate_trust_policy_proof,
    ProofStructuralResult,
};
pub use query::validate_query_case;
pub use snapshot::validate_snapshot_case;
pub use stream::validate_stream_append_case;
pub use trust::validate_trust_case;

#[cfg(test)]
mod tests;
