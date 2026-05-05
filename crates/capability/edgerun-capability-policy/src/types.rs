use crate::collections::{BTreeSet, HashMap, VecDeque};
use crate::prelude::v1::*;
use crate::time::{Duration, SystemTime, UNIX_EPOCH};

use edgerun_capabilities::{
    validate_descriptor, validate_grant, CapabilityAccessClass, CapabilityConstraint,
    CapabilityConstraintKind, CapabilityDescriptor, CapabilityError, CapabilityGrant,
    CapabilityInvocation, CapabilityModality, CapabilityOperation, CapabilityRequest,
    CapabilityRevocation, CapabilityRole, CapabilitySelector,
};
use edgerun_core::protocol::{Duration as ProstDuration, Timestamp};
use edgerun_core::protocol::{IdentityRef, NodeRef};
use edgerun_crypto::sha2::Digest;

#[derive(Clone, Debug, PartialEq)]
pub struct PolicyContext {
    pub requester: Option<IdentityRef>,
    pub requester_node: Option<NodeRef>,
    pub is_local: bool,
    pub user_present: bool,
    pub biometric_present: bool,
    pub hardware_protected: bool,
    pub now: SystemTime,
}

impl Default for PolicyContext {
    fn default() -> Self {
        Self {
            requester: None,
            requester_node: None,
            is_local: true,
            user_present: false,
            biometric_present: false,
            hardware_protected: false,
            now: SystemTime::now(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PolicyDecision {
    Allow { grant: Box<CapabilityGrant> },
    Deny { reason: &'static str },
    RequireInteraction { reason: &'static str },
}

#[derive(Clone, Debug, PartialEq)]
pub enum RevocationReason {
    PolicyChanged,
    UserRevoked,
    Expired,
    Superseded,
    Misuse,
    Other(String),
}

impl RevocationReason {
    pub fn as_str(&self) -> &str {
        match self {
            Self::PolicyChanged => "policy_changed",
            Self::UserRevoked => "user_revoked",
            Self::Expired => "expired",
            Self::Superseded => "superseded",
            Self::Misuse => "misuse",
            Self::Other(reason) => reason.as_str(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct GrantRecord {
    pub grant: CapabilityGrant,
    pub issued_at: SystemTime,
    pub expires_at: Option<SystemTime>,
    pub revoked: Option<CapabilityRevocation>,
    pub invocation_count: u64,
    pub(crate) invocation_timestamps: VecDeque<SystemTime>,
}

impl GrantRecord {
    pub fn is_revoked(&self) -> bool {
        self.revoked.is_some()
    }
}
