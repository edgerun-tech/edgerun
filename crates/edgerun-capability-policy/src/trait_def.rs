use std::collections::{BTreeSet, HashMap, VecDeque};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::types::*;
use edgerun_capabilities::{
    validate_descriptor, validate_grant, CapabilityAccessClass, CapabilityConstraint,
    CapabilityConstraintKind, CapabilityDescriptor, CapabilityError, CapabilityGrant,
    CapabilityInvocation, CapabilityModality, CapabilityOperation, CapabilityRequest,
    CapabilityRevocation, CapabilityRole, CapabilitySelector,
};
use edgerun_crypto::sha2::Digest;
use edgerun_proto::edgerun::v0::common::{IdentityRef, NodeRef};
use prost_types::{Duration as ProstDuration, Timestamp};

pub trait PolicyEngine {
    fn evaluate_request(
        &mut self,
        descriptor: &CapabilityDescriptor,
        request: &CapabilityRequest,
        context: &PolicyContext,
    ) -> Result<PolicyDecision, CapabilityError>;

    fn authorize_invocation(
        &mut self,
        invocation: &CapabilityInvocation,
        context: &PolicyContext,
    ) -> Result<(), CapabilityError>;

    fn revoke(
        &mut self,
        grant_id: &[u8],
        reason: RevocationReason,
    ) -> Result<CapabilityRevocation, CapabilityError>;

    fn grant_record(&self, grant_id: &[u8]) -> Option<&GrantRecord>;
}
