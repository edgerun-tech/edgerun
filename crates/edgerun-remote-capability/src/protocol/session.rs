//! Session lifecycle helpers: open, accept, reject, and request construction.

use edgerun_capabilities::{CapabilityDescriptor, CapabilityError};
use edgerun_proto::edgerun::v0::capability::CapabilityGrant;
use edgerun_proto::edgerun::v0::capability_runtime::{
    CapabilitySessionAccept, CapabilitySessionOpen,
};

/// Build a capability request from a session open.
pub fn session_open_as_request(
    open: &CapabilitySessionOpen,
    descriptor: &CapabilityDescriptor,
    requester: Option<edgerun_proto::edgerun::v0::common::IdentityRef>,
    requester_node: Option<edgerun_proto::edgerun::v0::common::NodeRef>,
) -> edgerun_capabilities::CapabilityRequest {
    edgerun_capabilities::CapabilityRequest {
        request_version: open.version,
        request_id: open.session_id.clone(),
        requester,
        requester_node,
        selector: open.selector.clone().or_else(|| {
            Some(selector_from_descriptor(
                descriptor,
                open.requested_access_class,
            ))
        }),
        requested_operations: open.requested_operations.clone(),
        requested_constraints: open.requested_constraints.clone(),
        purpose: String::new(),
        requested_duration: None,
        correlation_id: open.correlation_id.clone(),
        signature: None,
    }
}

/// Build an accept from a policy grant.
pub fn session_accept_from_grant(
    open: &CapabilitySessionOpen,
    grant: &CapabilityGrant,
) -> CapabilitySessionAccept {
    CapabilitySessionAccept {
        version: open.version,
        session_id: open.session_id.clone(),
        accepted: true,
        granted_operations: grant.granted_operations.clone(),
        granted_access_class: grant.access_class,
        error_reason: String::new(),
        grant_id: grant.grant_id.clone(),
    }
}

/// Build a rejected session accept.
pub fn session_reject(
    open: &CapabilitySessionOpen,
    reason: impl Into<String>,
) -> CapabilitySessionAccept {
    CapabilitySessionAccept {
        version: open.version,
        session_id: open.session_id.clone(),
        accepted: false,
        granted_operations: Vec::new(),
        granted_access_class: edgerun_capabilities::CapabilityAccessClass::Unspecified as i32,
        error_reason: reason.into(),
        grant_id: Vec::new(),
    }
}

/// Unconditionally accept a session open (no policy check).
pub fn accept_session_open_unchecked(open: &CapabilitySessionOpen) -> CapabilitySessionAccept {
    CapabilitySessionAccept {
        version: open.version,
        session_id: open.session_id.clone(),
        accepted: true,
        granted_operations: open.requested_operations.clone(),
        granted_access_class: open.requested_access_class,
        error_reason: String::new(),
        grant_id: open.session_id.clone(),
    }
}

/// Default requester identity for remote capability sessions.
pub fn default_remote_requester() -> edgerun_proto::edgerun::v0::common::IdentityRef {
    edgerun_proto::edgerun::v0::common::IdentityRef {
        identity_id: b"remote-capability-client".to_vec(),
        identity_kind: None,
        key_hint: None,
    }
}

pub fn default_remote_requester_opt() -> Option<edgerun_proto::edgerun::v0::common::IdentityRef> {
    Some(default_remote_requester())
}

fn selector_from_descriptor(
    descriptor: &CapabilityDescriptor,
    requested_access_class: i32,
) -> edgerun_capabilities::CapabilitySelector {
    edgerun_capabilities::CapabilitySelector {
        capability_id: descriptor.capability_id.clone(),
        role: descriptor.role,
        modalities: descriptor.modalities.clone(),
        event_kinds: descriptor.event_kinds.clone(),
        operations: descriptor.operations.clone(),
        access_class: requested_access_class,
        provider_identity: descriptor.provider_identity.clone(),
        provider_node: descriptor.provider_node.clone(),
        provider_instance_id: descriptor.provider_instance_id.clone(),
    }
}
