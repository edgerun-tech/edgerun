//! Native Edgerun protocol types.
//!
//! This module is generated from the old Rust-shaped Rust files, but it is
//! plain Rust inside edgerun-core. There is only the native rkyv protocol boundary
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

#[derive(Clone, Debug)]
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
    let normalized = if signable {
        signable_record(record)
    } else {
        record.clone()
    };
    alloc::format!("edgerun-rkyv-v0|signable={signable}|{normalized:?}").into_bytes()
}

fn signable_record(record: &ProtocolRecord) -> ProtocolRecord {
    match record {
        ProtocolRecord::CommandEnvelope(value) => {
            let mut value = value.clone();
            value.signature = None;
            ProtocolRecord::CommandEnvelope(value)
        }
        ProtocolRecord::EventEnvelope(value) => {
            let mut value = value.clone();
            value.signature = None;
            ProtocolRecord::EventEnvelope(value)
        }
        ProtocolRecord::DelegationRecord(value) => {
            let mut value = value.clone();
            value.signature = None;
            ProtocolRecord::DelegationRecord(value)
        }
        ProtocolRecord::RevocationRecord(value) => {
            let mut value = value.clone();
            value.signature = None;
            ProtocolRecord::RevocationRecord(value)
        }
        ProtocolRecord::IdentityRecord(value) => {
            let mut value = value.clone();
            value.signature = None;
            ProtocolRecord::IdentityRecord(value)
        }
        ProtocolRecord::RouteAdvertisement(value) => {
            let mut value = value.clone();
            value.signature = None;
            ProtocolRecord::RouteAdvertisement(value)
        }
        ProtocolRecord::AssuranceClaim(value) => {
            let mut value = value.clone();
            value.signature = None;
            ProtocolRecord::AssuranceClaim(value)
        }
        ProtocolRecord::SnapshotDescriptor(value) => {
            let mut value = value.clone();
            value.signature = None;
            ProtocolRecord::SnapshotDescriptor(value)
        }
        other => other.clone(),
    }
}
