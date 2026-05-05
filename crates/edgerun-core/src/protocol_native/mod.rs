//! Native Edgerun protocol types.
//!
//! This module is generated from the old protobuf-shaped Rust files, but it is
//! plain Rust inside edgerun-core. There is no edgerun-proto crate and no prost
//! derive in this module.

extern crate alloc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Timestamp {
    pub seconds: i64,
    pub nanos: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Duration {
    pub seconds: i64,
    pub nanos: i32,
}

#[path = "gen/edgerun.v0.access.rs"]
pub mod access;

#[path = "gen/edgerun.v0.app.rs"]
pub mod app;

#[path = "gen/edgerun.v0.appabi.rs"]
pub mod appabi;

#[path = "gen/edgerun.v0.capability.rs"]
pub mod capability;

#[path = "gen/edgerun.v0.capability_runtime.rs"]
pub mod capability_runtime;

#[path = "gen/edgerun.v0.common.rs"]
pub mod common;

#[path = "gen/edgerun.v0.identity.rs"]
pub mod identity;

#[path = "gen/edgerun.v0.network.rs"]
pub mod network;

#[path = "gen/edgerun.v0.object.rs"]
pub mod object;

#[path = "gen/edgerun.v0.server_resources.rs"]
pub mod server_resources;

#[path = "gen/edgerun.v0.stream.rs"]
pub mod stream;

#[path = "gen/edgerun.v0.trust.rs"]
pub mod trust;

#[path = "gen/edgerun.v0.ui.rs"]
pub mod ui;

#[path = "gen/edgerun.wallet.v0.rs"]
pub mod edgerun_wallet_v0;

pub use access::*;
pub use app::*;
pub use appabi::*;
pub use capability_runtime::*;
pub use common::*;
pub use edgerun_wallet_v0::*;
pub use identity::*;
pub use network::*;
pub use object::*;
pub use server_resources::*;
pub use stream::*;
pub use ui::*;

// Explicit reexports prevent name collisions between capability::* and trust::*.
pub use capability::{
    CapabilityAccessClass, CapabilityConstraint, CapabilityConstraintKind, CapabilityEventKind,
    CapabilityGrant, CapabilityInvocation, CapabilityModality, CapabilityOperation,
    CapabilityRequest, CapabilityResult as CapabilityRuntimeResult, CapabilityRevocation,
    CapabilityRole,
};

pub use trust::{
    AggregateTrustPolicy, AssuranceClaim, AssuranceRequirement, CapabilityDescriptor,
    CapabilityKind, ConstraintSet, DelegationPolicy, DelegationRecord, ExportPolicy,
    RevocationKind, RevocationRecord, RouteSelectionPolicy, RouteTrustAssignment,
    RouteTrustAssignments, ScopeDescriptor, ScopeKind,
};

pub fn enum_from_i32<T>(value: i32) -> Option<T>
where
    T: NativeEnum,
{
    T::from_i32(value)
}

pub trait NativeEnum: Sized {
    fn from_i32(value: i32) -> Option<Self>;
}

#[derive(Clone)]
pub enum ProtocolRecord {
    CommandEnvelope(CommandEnvelope),
    EventEnvelope(EventEnvelope),
    CommandResultPayload(CommandResultPayload),
    DelegationRecord(DelegationRecord),
    RevocationRecord(RevocationRecord),
    IdentityRecord(IdentityRecord),
    RouteAdvertisement(RouteAdvertisement),
    AssuranceClaim(AssuranceClaim),
    SnapshotDescriptor(SnapshotDescriptor),
    ObjectRef(ObjectRef),
    Digest(Digest),
    Signature(Signature),
}

pub fn canonical_bytes(record: &ProtocolRecord, signable: bool) -> alloc::vec::Vec<u8> {
    match record {
        ProtocolRecord::CommandEnvelope(command) => {
            if signable {
                crate::wire_command::command_signable_bytes(command)
            } else {
                crate::wire_command::command_full_bytes(command)
            }
        }
        ProtocolRecord::EventEnvelope(event) => {
            if signable {
                crate::wire_stream::event_signable_wire_bytes(event)
            } else {
                crate::wire_stream::event_full_wire_bytes(event)
            }
        }
        ProtocolRecord::DelegationRecord(delegation) => {
            if signable {
                crate::wire_trust::delegation_record_signable_bytes(delegation)
            } else {
                crate::wire_trust::delegation_record_full_bytes(delegation)
            }
        }
        ProtocolRecord::RevocationRecord(revocation) => {
            if signable {
                crate::wire_trust::revocation_record_signable_bytes(revocation)
            } else {
                crate::wire_trust::revocation_record_full_bytes(revocation)
            }
        }
        ProtocolRecord::IdentityRecord(identity) => {
            if signable {
                crate::wire_trust::identity_record_signable_bytes(identity)
            } else {
                crate::wire_trust::identity_record_full_bytes(identity)
            }
        }
        ProtocolRecord::AssuranceClaim(claim) => {
            if signable {
                crate::wire_trust::assurance_claim_signable_bytes(claim)
            } else {
                crate::wire_trust::assurance_claim_full_bytes(claim)
            }
        }
        ProtocolRecord::CommandResultPayload(payload) => {
            crate::wire_command::command_result_bytes(payload)
        }
        other => edgerun_wire::canonical_bytes(&edgerun_wire::struct_value(alloc::vec![
            edgerun_wire::field(
                1,
                edgerun_wire::text(match other {
                    ProtocolRecord::RouteAdvertisement(_) => "RouteAdvertisement",
                    ProtocolRecord::SnapshotDescriptor(_) => "SnapshotDescriptor",
                    ProtocolRecord::ObjectRef(_) => "ObjectRef",
                    ProtocolRecord::Digest(_) => "Digest",
                    ProtocolRecord::Signature(_) => "Signature",
                    _ => "ProtocolRecord",
                })
            ),
            edgerun_wire::field(2, edgerun_wire::boolv(signable)),
        ])),
    }
}
