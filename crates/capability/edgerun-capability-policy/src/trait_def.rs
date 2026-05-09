use crate::collections::{BTreeSet, HashMap, VecDeque};
use crate::prelude::v1::*;
use crate::time::{Duration, SystemTime, UNIX_EPOCH};

use super::types::*;
use edgerun_capabilities::{
    CapabilityAccessClass, CapabilityConstraint, CapabilityConstraintKind, CapabilityDescriptor,
    CapabilityError, CapabilityGrant, CapabilityInvocation, CapabilityModality,
    CapabilityOperation, CapabilityRequest, CapabilityRevocation, CapabilityRole,
    CapabilitySelector, validate_descriptor, validate_grant,
};
use edgerun_crypto::sha::Digest;
use edgerun_protocols::core_protocol::protocol::{Duration as ProstDuration, Timestamp};
use edgerun_protocols::core_protocol::protocol::{IdentityRef, NodeRef};

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
