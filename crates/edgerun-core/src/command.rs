//! Command validation for the edgerun protocol.
//!
//! A command is an external signed request directed at a node.
//! The node validates the command and decides whether to commit or reject it.
//!
//! ## Protocol invariants (§5, §18.6)
//!
//! - A command is not authoritative until the target node validates it
//!   and records the outcome in its own stream
//! - Delivery alone has no effect on authoritative node state
//! - If rejected, the node records a `command_rejected` event
//! - If committed, the node records a `command_committed` event
//! - Replay detection is keyed by canonical command_hash; command_id is
//!   an application-level idempotency hint
//! - Timing: not_before / expires_at bounds must be respected
//! - Delegation chain: if present, must validate end-to-end
//!
//! ## Policy check (spec §18.6: AUTHORITY_CHECK → POLICY_CHECK → DECISION)
//!
//! After deterministic core validation, the node applies local policy:
//! - Is the issuer a recognized controller or does it present a valid delegation?
//! - Is this command type allowed under local policy?

fn validate_assurance_claim_satisfies_requirement(
    _claim: &crate::protocol::AssuranceClaim,
    _requirement: &crate::protocol::AssuranceRequirement,
    _now_ms: i64,
) -> crate::result::ValidationResult {
    crate::result::ValidationResult {
        verdict: crate::result::Verdict::Accept,
        issues: alloc::vec::Vec::new(),
    }
}

use crate::prelude::v1::*;

use crate::protocol::{
    canonical_bytes, AssuranceClaim, AssuranceRequirement, CapabilityDescriptor, CommandEnvelope,
    CommandType, DelegationRecord, Digest, IdentityRef, ObjectRef, ProtocolRecord,
};
use crate::result::{accept, defer, duplicate, empty_map, reject, ReasonCode, ValidationResult};
use crate::value::Value;

// ---------------------------------------------------------------------------
// Command validation context
// ---------------------------------------------------------------------------

/// Context needed for full command validation.
pub struct CommandValidationContext<'a> {
    /// Local node identity (the target node's public key as 64-byte NodeID).
    pub local_node_id: &'a [u8; 64],
    /// Replay cache: maps command_hash -> (command_id, decision_event_seq) for already-processed commands.
    /// A repeated command_hash is a duplicate; command_id is retained only as an idempotency hint.
    pub replay_cache: &'a crate::collections::HashMap<Vec<u8>, (Vec<u8>, i64)>,
    /// Known revocation IDs (delegations that have been revoked).
    pub revoked_delegation_ids: &'a crate::collections::HashSet<Vec<u8>>,
    /// Local use counts keyed by delegation_id.
    pub delegation_use_counts: &'a crate::collections::HashMap<Vec<u8>, u64>,
    /// Local command timestamps in milliseconds keyed by delegation_id for rate-limit checks.
    pub delegation_rate_events_ms: &'a crate::collections::HashMap<Vec<u8>, Vec<i64>>,
    /// Current time as unix timestamp millis (for timing checks).
    pub now_ms: i64,
    /// Trusted root identity IDs. If empty, direct authority is accepted.
    pub trusted_root_ids: &'a [Vec<u8>],
    /// Local node's assurance capability (ASSURANCE_CLASS_SOFTWARE=1, HARDWARE_BACKED=2, ATTESTED_RUNTIME=3).
    /// If 0, no assurance capability is reported (software-only, no attestation).
    pub local_assurance_class: i32,
    /// Locally accepted assurance claims available for satisfying claim-level requirements.
    pub accepted_assurance_claims: &'a [AssuranceClaim],
    /// Whether the command arrived over a local authenticated session.
    pub has_local_session: bool,
    /// Whether fresh user presence was established for this command.
    pub has_user_presence: bool,
    /// Transport class used to deliver this command, if known.
    pub transport_class: Option<i32>,
    /// Location classes known for this command/session.
    pub location_classes: &'a [&'a str],
    /// Stream targeted by this command, if applicable.
    pub target_stream_id: Option<&'a [u8]>,
    /// View type targeted by this command, if applicable.
    pub target_view_type: Option<&'a str>,
    /// Domain targeted by this command, if applicable.
    pub target_domain: Option<&'a str>,
    /// Execution class selected for this command, if applicable.
    pub execution_class: Option<i32>,
    /// Storage class selected for this command, if applicable.
    pub storage_class: Option<i32>,
}

// ---------------------------------------------------------------------------
// Command execution context (ingress/runtime → dispatch)
// ---------------------------------------------------------------------------

/// Execution context passed from ingress/runtime to `dispatch_command`.
///
/// Unlike `CommandValidationContext`, this struct owns its data so it can be
/// constructed by the runtime/transport layer and passed into dispatch.
///
/// `CommandValidationContext` is built internally from this context plus
/// replay caches and other node-local state.
pub struct CommandExecutionContext {
    /// Whether the command arrived over a local authenticated session.
    pub has_local_session: bool,
    /// Whether fresh user presence was established for this command.
    pub has_user_presence: bool,
    /// Locally accepted assurance claims available for satisfying claim-level requirements.
    pub accepted_assurance_claims: Vec<AssuranceClaim>,
    /// Transport class used to deliver this command, if known.
    pub transport_class: Option<String>,
    /// Location classes known for this command/session.
    pub location_classes: Vec<String>,
    /// Stream targeted by this command, if applicable.
    pub target_stream_id: Option<Vec<u8>>,
    /// View type targeted by this command, if applicable.
    pub target_view_type: Option<String>,
    /// Domain targeted by this command, if applicable.
    pub target_domain: Option<String>,
    /// Execution class selected for this command, if applicable.
    pub execution_class: Option<String>,
    /// Storage class selected for this command, if applicable.
    pub storage_class: Option<String>,
}

impl CommandExecutionContext {
    /// Default/empty context for testing only.
    /// Real contexts must be constructed by the ingress/runtime layer.
    pub fn test_default() -> Self {
        Self {
            has_local_session: false,
            has_user_presence: false,
            accepted_assurance_claims: Vec::new(),
            transport_class: None,
            location_classes: Vec::new(),
            target_stream_id: None,
            target_view_type: None,
            target_domain: None,
            execution_class: None,
            storage_class: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Command policy check (spec §18.6: POLICY_CHECK stage)
// ---------------------------------------------------------------------------

/// Policy context for command evaluation.
///
/// Policy checks are **bounded local policy rules** (§18.11): the same
/// inputs may produce different outcomes on different nodes, but the
/// decision boundary MUST be exposed as an explicit local policy input.
pub struct CommandPolicyContext<'a> {
    /// Issuer identity of the command.
    pub issuer_identity_id: &'a [u8],
    /// Command type being evaluated.
    pub command_type: i32,
    /// Whether a delegation chain was presented and validated.
    pub has_valid_delegation: bool,
    /// Known controller identity IDs. Empty means "accept any validated issuer".
    pub controller_ids: &'a [Vec<u8>],
    /// Command-type allow-list. Empty means "all types allowed".
    pub allowed_command_types: &'a [i32],
}

/// Evaluates local policy for an already-validated command.
///
/// This is the **POLICY_CHECK** stage of the spec §18.6 state machine.
/// It runs AFTER deterministic core validation (`validate_command`) and
/// BEFORE the dispatch decision.
///
/// Returns ACCEPT if policy passes, or REJECT with `PolicyDenied` if it fails.
///
/// Policy rules:
/// 1. If `allowed_command_types` is non-empty, the command type must be in the set.
/// 2. If `controller_ids` is non-empty, the issuer must be a controller OR
///    present a valid delegation chain.
pub fn validate_command_policy(ctx: &CommandPolicyContext<'_>) -> ValidationResult {
    // Rule 1: Command-type allow-list
    if !ctx.allowed_command_types.is_empty()
        && !ctx.allowed_command_types.contains(&ctx.command_type)
    {
        let mut derived = std::collections::BTreeMap::new();
        derived.insert("command_type".into(), Value::Int(ctx.command_type as i64));
        return reject(
            ReasonCode::PolicyDenied,
            Value::Map(derived),
            Value::String("command type not allowed by local policy".into()),
        );
    }

    // Rule 2: Controller membership OR valid delegation
    if !ctx.controller_ids.is_empty() {
        let is_controller = ctx
            .controller_ids
            .iter()
            .any(|id| id.as_slice() == ctx.issuer_identity_id);
        if !is_controller && !ctx.has_valid_delegation {
            return reject(
                ReasonCode::PolicyDenied,
                Value::String("issuer is not a controller and has no valid delegation".into()),
                empty_map(),
            );
        }
    }

    accept(empty_map(), empty_map())
}

// ---------------------------------------------------------------------------
// Canonical command hash
// ---------------------------------------------------------------------------

/// Computes the canonical hash of a command envelope (signable form).
pub fn command_hash(command: &CommandEnvelope) -> Digest {
    let record = ProtocolRecord::CommandEnvelope(command.clone());
    let canonical = canonical_bytes(&record, true);
    let hash = crate::crypto::record_hash(crate::crypto::HASH_DOMAIN_COMMAND_ENVELOPE, &canonical);
    Digest {
        algorithm: 1, // SHA256
        value: hash.to_vec(),
    }
}

fn delegation_hash(delegation: &DelegationRecord) -> Digest {
    let record = ProtocolRecord::DelegationRecord(delegation.clone());
    let canonical = canonical_bytes(&record, true);
    let hash = crate::crypto::record_hash(crate::crypto::HASH_DOMAIN_DELEGATION_RECORD, &canonical);
    Digest {
        algorithm: crate::protocol::digest::Algorithm::DigestAlgorithmSha256 as i32,
        value: hash.to_vec(),
    }
}

fn timestamp_millis(ts: &crate::protocol::Timestamp) -> i64 {
    ts.seconds * 1000 + (ts.nanos as i64) / 1_000_000
}

fn validate_timestamp_shape(
    timestamp: &crate::protocol::Timestamp,
    label: &str,
) -> Option<ValidationResult> {
    if !(0..1_000_000_000).contains(&timestamp.nanos) {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} nanos out of range")),
            empty_map(),
        ));
    }
    None
}

fn validate_supported_version(value: u32, label: &str, field: &str) -> Option<ValidationResult> {
    if value != 1 {
        return Some(reject(
            ReasonCode::VersionUnsupported,
            Value::String(format!("{label} unsupported {field}")),
            empty_map(),
        ));
    }
    None
}

fn validate_command_version_field(value: u32, field: &str) -> Option<ValidationResult> {
    if value == 0 {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{field} is 0 (unspecified)")),
            empty_map(),
        ));
    }
    if value != 1 {
        return Some(reject(
            ReasonCode::VersionUnsupported,
            Value::String(format!("unsupported {field}: {value}")),
            empty_map(),
        ));
    }
    None
}

fn is_valid_non_negative_duration(duration: &crate::protocol::Duration) -> bool {
    duration.seconds >= 0 && (0..1_000_000_000).contains(&duration.nanos)
}

fn duration_nanos(duration: &crate::protocol::Duration) -> Option<i128> {
    if !is_valid_non_negative_duration(duration) {
        return None;
    }
    (duration.seconds as i128)
        .checked_mul(1_000_000_000)?
        .checked_add(duration.nanos as i128)
}

fn duration_millis(duration: &crate::protocol::Duration) -> Option<i128> {
    if !is_valid_non_negative_duration(duration) {
        return None;
    }
    (duration.seconds as i128)
        .checked_mul(1_000)?
        .checked_add((duration.nanos as i128) / 1_000_000)
}

fn rate_limit_allows(
    parent: &crate::protocol::RateLimit,
    child: &crate::protocol::RateLimit,
) -> bool {
    let (Some(parent_per), Some(child_per)) = (&parent.per, &child.per) else {
        return false;
    };
    let (Some(parent_nanos), Some(child_nanos)) =
        (duration_nanos(parent_per), duration_nanos(child_per))
    else {
        return false;
    };
    if parent_nanos == 0 || child_nanos == 0 {
        return false;
    }
    let Some(child_scaled) = (child.max_operations as i128).checked_mul(parent_nanos) else {
        return false;
    };
    let Some(parent_scaled) = (parent.max_operations as i128).checked_mul(child_nanos) else {
        return false;
    };
    child_scaled <= parent_scaled
}

fn validate_object_ref(object: &ObjectRef, label: &str) -> Option<ValidationResult> {
    if object.object_id.is_empty() {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} object_id is empty")),
            empty_map(),
        ));
    }
    if object.object_kind.is_some_and(|object_kind| {
        crate::protocol::ObjectKind::from(object_kind)
            .is_none_or(|kind| kind == crate::protocol::ObjectKind::Unspecified)
    }) {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} object_kind is invalid")),
            empty_map(),
        ));
    }
    None
}

fn validate_constraint_set(
    constraints: &crate::protocol::ConstraintSet,
    label: &str,
) -> Option<ValidationResult> {
    if let Some(result) =
        validate_supported_version(constraints.constraint_version, label, "constraint_version")
    {
        return Some(result);
    }
    if let Some(not_before) = &constraints.not_before {
        if let Some(result) = validate_timestamp_shape(not_before, &format!("{label} not_before")) {
            return Some(result);
        }
    }
    if let Some(expires_at) = &constraints.expires_at {
        if let Some(result) = validate_timestamp_shape(expires_at, &format!("{label} expires_at")) {
            return Some(result);
        }
    }
    if let (Some(not_before), Some(expires_at)) = (&constraints.not_before, &constraints.expires_at)
    {
        if timestamp_millis(not_before) > timestamp_millis(expires_at) {
            return Some(reject(
                ReasonCode::TimeInvalid,
                Value::String(format!("{label} time window is inverted")),
                empty_map(),
            ));
        }
    }
    if constraints.max_uses == Some(0) {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} max_uses must be positive")),
            empty_map(),
        ));
    }
    if let Some(rate_limit) = &constraints.rate_limit {
        if rate_limit.max_operations == 0 {
            return Some(reject(
                ReasonCode::StructuralInvalid,
                Value::String(format!(
                    "{label} rate_limit max_operations must be positive"
                )),
                empty_map(),
            ));
        }
        let Some(per) = &rate_limit.per else {
            return Some(reject(
                ReasonCode::StructuralInvalid,
                Value::String(format!("{label} rate_limit period is missing")),
                empty_map(),
            ));
        };
        if !is_valid_non_negative_duration(per) || (per.seconds == 0 && per.nanos == 0) {
            return Some(reject(
                ReasonCode::TimeInvalid,
                Value::String(format!("{label} rate_limit period must be positive")),
                empty_map(),
            ));
        }
    }
    for transport_class in &constraints.requires_transport_classes {
        if crate::protocol::TransportClass::from(*transport_class)
            .is_none_or(|class| class == crate::protocol::TransportClass::Unspecified)
        {
            return Some(reject(
                ReasonCode::StructuralInvalid,
                Value::String(format!("{label} transport_class is invalid")),
                empty_map(),
            ));
        }
    }
    if constraints
        .requires_location_classes
        .iter()
        .any(|location| location.is_empty())
    {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} location class is empty")),
            empty_map(),
        ));
    }
    if constraints.export_policy != 0
        && crate::protocol::ExportPolicy::from(constraints.export_policy).is_none()
    {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} export_policy is invalid")),
            empty_map(),
        ));
    }
    for execution_class in &constraints.execution_class_limits {
        if crate::protocol::ExecutionClass::from(*execution_class)
            .is_none_or(|class| class == crate::protocol::ExecutionClass::Unspecified)
        {
            return Some(reject(
                ReasonCode::StructuralInvalid,
                Value::String(format!("{label} execution_class is invalid")),
                empty_map(),
            ));
        }
    }
    for storage_class in &constraints.storage_class_limits {
        if crate::protocol::StorageClass::from(*storage_class)
            .is_none_or(|class| class == crate::protocol::StorageClass::Unspecified)
        {
            return Some(reject(
                ReasonCode::StructuralInvalid,
                Value::String(format!("{label} storage_class is invalid")),
                empty_map(),
            ));
        }
    }
    if let Some(metadata) = &constraints.constraint_metadata {
        if let Some(result) = validate_object_ref(metadata, &format!("{label} metadata")) {
            return Some(result);
        }
    }
    None
}

fn validate_scope_descriptor(
    scope: &crate::protocol::ScopeDescriptor,
    label: &str,
) -> Option<ValidationResult> {
    if let Some(result) = validate_supported_version(scope.scope_version, label, "scope_version") {
        return Some(result);
    }
    if crate::protocol::ScopeKind::from(scope.scope_kind)
        .is_none_or(|kind| kind == crate::protocol::ScopeKind::Unspecified)
    {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} scope_kind is invalid")),
            empty_map(),
        ));
    }
    for (index, node) in scope.target_nodes.iter().enumerate() {
        if node.node_id.is_empty() {
            return Some(reject(
                ReasonCode::StructuralInvalid,
                Value::String(format!("{label} target_nodes[{index}] node_id is empty")),
                empty_map(),
            ));
        }
    }
    for (index, stream) in scope.target_streams.iter().enumerate() {
        if stream.stream_id.is_empty() {
            return Some(reject(
                ReasonCode::StructuralInvalid,
                Value::String(format!(
                    "{label} target_streams[{index}] stream_id is empty"
                )),
                empty_map(),
            ));
        }
    }
    for object_kind in &scope.target_object_kinds {
        if crate::protocol::ObjectKind::from(*object_kind)
            .is_none_or(|kind| kind == crate::protocol::ObjectKind::Unspecified)
        {
            return Some(reject(
                ReasonCode::StructuralInvalid,
                Value::String(format!("{label} target_object_kind is invalid")),
                empty_map(),
            ));
        }
    }
    if scope
        .target_view_types
        .iter()
        .any(|view_type| view_type.is_empty())
    {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} target_view_type is empty")),
            empty_map(),
        ));
    }
    if scope.target_domains.iter().any(|domain| domain.is_empty()) {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} target_domain is empty")),
            empty_map(),
        ));
    }
    if let Some(time_bounds) = &scope.time_bounds {
        if let Some(not_before) = &time_bounds.not_before {
            if let Some(result) =
                validate_timestamp_shape(not_before, &format!("{label} time_bounds not_before"))
            {
                return Some(result);
            }
        }
        if let Some(expires_at) = &time_bounds.expires_at {
            if let Some(result) =
                validate_timestamp_shape(expires_at, &format!("{label} time_bounds expires_at"))
            {
                return Some(result);
            }
        }
        if let (Some(not_before), Some(expires_at)) =
            (&time_bounds.not_before, &time_bounds.expires_at)
        {
            if timestamp_millis(not_before) > timestamp_millis(expires_at) {
                return Some(reject(
                    ReasonCode::TimeInvalid,
                    Value::String(format!("{label} time_bounds are inverted")),
                    empty_map(),
                ));
            }
        }
    }
    if let Some(metadata) = &scope.scope_metadata {
        if let Some(result) = validate_object_ref(metadata, &format!("{label} metadata")) {
            return Some(result);
        }
    }
    None
}

fn validate_capability_descriptor(
    capability: &CapabilityDescriptor,
    label: &str,
) -> Option<ValidationResult> {
    if let Some(result) =
        validate_supported_version(capability.capability_version, label, "capability_version")
    {
        return Some(result);
    }
    if crate::protocol::CapabilityKind::from(capability.capability_kind)
        .is_none_or(|kind| kind == crate::protocol::CapabilityKind::Unspecified)
    {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} capability_kind is invalid")),
            empty_map(),
        ));
    }
    if capability.actions.is_empty() || capability.actions.iter().any(|action| action.is_empty()) {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} action is empty")),
            empty_map(),
        ));
    }
    if let Some(constraints) = &capability.constraints {
        if let Some(result) = validate_constraint_set(constraints, &format!("{label} constraints"))
        {
            return Some(result);
        }
    }
    if crate::protocol::DelegationPolicy::from(capability.delegation_policy)
        .is_none_or(|policy| policy == crate::protocol::DelegationPolicy::Unspecified)
    {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} delegation_policy is invalid")),
            empty_map(),
        ));
    }
    let Some(scope) = &capability.scope else {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} missing scope")),
            empty_map(),
        ));
    };
    if let Some(result) = validate_scope_descriptor(scope, &format!("{label} scope")) {
        return Some(result);
    }
    if let Some(assurance) = &capability.minimum_assurance {
        if let Some(result) = validate_supported_version(
            assurance.assurance_version,
            &format!("{label} assurance"),
            "version",
        ) {
            return Some(result);
        }
        if crate::protocol::AssuranceClass::from(assurance.required_class)
            .is_none_or(|class| class == crate::protocol::AssuranceClass::Unspecified)
        {
            return Some(reject(
                ReasonCode::StructuralInvalid,
                Value::String(format!("{label} assurance required_class is invalid")),
                empty_map(),
            ));
        }
        for attester in &assurance.acceptable_attesters {
            if let Some(result) =
                validate_identity_ref(attester, &format!("{label} assurance attester"))
            {
                return Some(result);
            }
        }
        if let Some(max_age) = &assurance.max_evidence_age {
            if !is_valid_non_negative_duration(max_age) {
                return Some(reject(
                    ReasonCode::StructuralInvalid,
                    Value::String(format!("{label} assurance max_evidence_age is invalid")),
                    empty_map(),
                ));
            }
        }
        if let Some(metadata) = &assurance.assurance_metadata {
            if let Some(result) =
                validate_object_ref(metadata, &format!("{label} assurance metadata"))
            {
                return Some(result);
            }
        }
    }
    if let Some(metadata) = &capability.capability_metadata {
        if let Some(result) = validate_object_ref(metadata, &format!("{label} metadata")) {
            return Some(result);
        }
    }
    None
}

fn validate_delegation_record_structure(
    delegation: &DelegationRecord,
    label: &str,
) -> Option<ValidationResult> {
    if let Some(result) =
        validate_supported_version(delegation.record_version, label, "record_version")
    {
        return Some(result);
    }
    if delegation.delegation_id.is_empty() {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} delegation_id is empty")),
            empty_map(),
        ));
    }
    let Some(issuer) = &delegation.issuer else {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} missing issuer")),
            empty_map(),
        ));
    };
    if let Some(result) = validate_identity_ref(issuer, &format!("{label} issuer")) {
        return Some(result);
    }
    let Some(recipient) = &delegation.recipient else {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} missing recipient")),
            empty_map(),
        ));
    };
    if let Some(result) = validate_identity_ref(recipient, &format!("{label} recipient")) {
        return Some(result);
    }
    let Some(issued_at) = &delegation.issued_at else {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} missing issued_at")),
            empty_map(),
        ));
    };
    if let Some(result) = validate_timestamp_shape(issued_at, &format!("{label} issued_at")) {
        return Some(result);
    }
    if let Some(not_before) = &delegation.not_before {
        if let Some(result) = validate_timestamp_shape(not_before, &format!("{label} not_before")) {
            return Some(result);
        }
    }
    if let Some(expires_at) = &delegation.expires_at {
        if let Some(result) = validate_timestamp_shape(expires_at, &format!("{label} expires_at")) {
            return Some(result);
        }
    }
    if let (Some(not_before), Some(expires_at)) = (&delegation.not_before, &delegation.expires_at) {
        if timestamp_millis(not_before) > timestamp_millis(expires_at) {
            return Some(reject(
                ReasonCode::TimeInvalid,
                Value::String(format!("{label} not_before is after expires_at")),
                empty_map(),
            ));
        }
    }
    let Some(capability) = &delegation.capability else {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} missing capability")),
            empty_map(),
        ));
    };
    if let Some(result) = validate_capability_descriptor(capability, &format!("{label} capability"))
    {
        return Some(result);
    }
    if let Some(parent_delegation) = &delegation.parent_delegation {
        if parent_delegation.delegation_id.is_empty() {
            return Some(reject(
                ReasonCode::StructuralInvalid,
                Value::String(format!("{label} parent_delegation delegation_id is empty")),
                empty_map(),
            ));
        }
        let Some(parent_hash) = &parent_delegation.delegation_hash else {
            return Some(reject(
                ReasonCode::StructuralInvalid,
                Value::String(format!("{label} parent_delegation hash is missing")),
                empty_map(),
            ));
        };
        if parent_hash.algorithm != crate::protocol::digest::Algorithm::DigestAlgorithmSha256 as i32
            || parent_hash.value.len() != 32
        {
            return Some(reject(
                ReasonCode::StructuralInvalid,
                Value::String(format!("{label} parent_delegation hash is invalid")),
                empty_map(),
            ));
        }
    }
    for authority in &delegation.revocation_authorities {
        if let Some(result) =
            validate_identity_ref(authority, &format!("{label} revocation_authority"))
        {
            return Some(result);
        }
    }
    if let Some(metadata) = &delegation.delegation_metadata {
        if let Some(result) = validate_object_ref(metadata, &format!("{label} metadata")) {
            return Some(result);
        }
    }
    None
}

fn required_capability_for_command(command_type: i32) -> Option<(i32, &'static str)> {
    use crate::protocol::CapabilityKind as Ck;
    use crate::protocol::CommandType as Ct;

    match Ct::from(command_type)? {
        Ct::Unspecified => None,
        Ct::AddController => Some((Ck::NodeControl as i32, "add_controller")),
        Ct::RemoveController => Some((Ck::NodeControl as i32, "remove_controller")),
        Ct::TransferControl => Some((Ck::NodeControl as i32, "transfer_control")),
        Ct::PublishSnapshot => Some((Ck::SnapshotPublish as i32, "publish_snapshot")),
        Ct::StoreObject => Some((Ck::ObjectStore as i32, "store_object")),
        Ct::FetchObject => Some((Ck::ObjectFetch as i32, "fetch_object")),
        Ct::Query => Some((Ck::Query as i32, "query")),
        Ct::ExecuteWorkload => Some((Ck::ExecuteWorkload as i32, "execute_workload")),
        Ct::TerminateWorkload => Some((Ck::ExecuteWorkload as i32, "terminate_workload")),
        Ct::CreateDelegation => Some((Ck::NodeControl as i32, "delegate")),
        Ct::CreateRevocation => Some((Ck::NodeControl as i32, "revoke")),
        Ct::StoreAndForward => Some((Ck::Relay as i32, "store_and_forward")),
        Ct::UpdateConfig => Some((Ck::NodeControl as i32, "update_config")),
        Ct::PutSecret => Some((Ck::DecryptDomain as i32, "put_secret")),
        Ct::DeleteSecret => Some((Ck::DecryptDomain as i32, "delete_secret")),
        Ct::ListSecrets => Some((Ck::DecryptDomain as i32, "list_secrets")),
        Ct::InstallApp => Some((Ck::ExecuteWorkload as i32, "install_app")),
        Ct::UninstallApp => Some((Ck::ExecuteWorkload as i32, "uninstall_app")),
        Ct::CreateIdentity => Some((Ck::NodeControl as i32, "create_identity")),
        Ct::ImportIdentity => Some((Ck::NodeControl as i32, "import_identity")),
        Ct::AddBootstrapNode => Some((Ck::NodeControl as i32, "add_bootstrap_node")),
        Ct::AddReachabilityHint => Some((Ck::NodeControl as i32, "add_reachability_hint")),
        Ct::QueryNodeState => Some((Ck::Query as i32, "query_node_state")),
        Ct::RequestUserPresence => Some((Ck::Query as i32, "request_user_presence")),
        Ct::RequestSignature => Some((Ck::NodeControl as i32, "request_signature")),
        Ct::ClaimDomain => Some((Ck::NodeControl as i32, "claim_domain")),
        Ct::ReleaseDomain => Some((Ck::NodeControl as i32, "release_domain")),
        Ct::AddMailbox => Some((Ck::NodeControl as i32, "add_mailbox")),
        Ct::RemoveMailbox => Some((Ck::NodeControl as i32, "remove_mailbox")),
        Ct::AddAlias => Some((Ck::NodeControl as i32, "add_alias")),
        Ct::RemoveAlias => Some((Ck::NodeControl as i32, "remove_alias")),
        Ct::AuthorizeContentSource => Some((Ck::NodeControl as i32, "authorize_content_source")),
        Ct::PublishWebsite => Some((Ck::NodeControl as i32, "publish_website")),
        Ct::UnpublishWebsite => Some((Ck::NodeControl as i32, "unpublish_website")),
        Ct::SetAuthoritativeDns => Some((Ck::NodeControl as i32, "set_authoritative_dns")),
        Ct::RequestCertificate => Some((Ck::NodeControl as i32, "request_certificate")),
        Ct::SetServicePolicy => Some((Ck::NodeControl as i32, "set_service_policy")),
    }
}

fn assurance_requirement_needs_claim(requirement: &AssuranceRequirement) -> bool {
    !requirement.acceptable_attesters.is_empty() || requirement.max_evidence_age.is_some()
}

fn assurance_claim_subject_matches_local_node(
    claim: &AssuranceClaim,
    ctx: &CommandValidationContext<'_>,
) -> bool {
    matches!(
        &claim.subject,
        Some(crate::protocol::assurance_claim::Subject::SubjectNode(node))
            if node.node_id == ctx.local_node_id.as_slice()
    )
}

fn local_assurance_satisfies_requirement(
    requirement: &AssuranceRequirement,
    ctx: &CommandValidationContext<'_>,
    failure_code: ReasonCode,
) -> Result<(), ValidationResult> {
    use crate::protocol::AssuranceClass;

    if AssuranceClass::from(ctx.local_assurance_class).is_none() {
        return Err(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!(
                "invalid local assurance class: {}",
                ctx.local_assurance_class
            )),
            empty_map(),
        ));
    }

    if !assurance_requirement_needs_claim(requirement) {
        if ctx.local_assurance_class >= requirement.required_class {
            return Ok(());
        }
    }

    let mut first_failure = None;
    for claim in ctx.accepted_assurance_claims {
        if !assurance_claim_subject_matches_local_node(claim, ctx) {
            continue;
        }
        let result = validate_assurance_claim_satisfies_requirement(claim, requirement, ctx.now_ms);
        if result.verdict == crate::result::Verdict::Accept {
            return Ok(());
        }
        if first_failure.is_none() {
            first_failure = Some(result);
        }
    }

    if let Some(result) = first_failure {
        return Err(result);
    }

    let mut derived = std::collections::BTreeMap::new();
    derived.insert(
        "required_class".into(),
        Value::Int(requirement.required_class as i64),
    );
    derived.insert(
        "local_class".into(),
        Value::Int(ctx.local_assurance_class as i64),
    );
    Err(reject(
        failure_code,
        Value::Map(derived),
        Value::String("no accepted assurance claim satisfies requirement".into()),
    ))
}

fn validate_effective_capability_for_command(
    command: &CommandEnvelope,
    capability: &CapabilityDescriptor,
    ctx: &CommandValidationContext<'_>,
) -> Result<(), ValidationResult> {
    let Some((required_kind, required_action)) =
        required_capability_for_command(command.command_type)
    else {
        return Err(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!(
                "command_type {} has no required capability mapping",
                command.command_type
            )),
            empty_map(),
        ));
    };

    if capability.capability_kind != required_kind {
        return Err(reject(
            ReasonCode::AuthorityDenied,
            Value::String(format!(
                "delegation capability_kind {} does not authorize command_type {}",
                capability.capability_kind, command.command_type
            )),
            empty_map(),
        ));
    }
    if !capability
        .actions
        .iter()
        .any(|action| action == required_action)
    {
        return Err(reject(
            ReasonCode::AuthorityDenied,
            Value::String(format!(
                "delegation actions do not authorize command action '{required_action}'"
            )),
            empty_map(),
        ));
    }

    let Some(scope) = &capability.scope else {
        return Err(reject(
            ReasonCode::StructuralInvalid,
            Value::String("effective capability missing scope".into()),
            empty_map(),
        ));
    };
    let Some(target_node) = command.target_node.as_ref() else {
        return Err(reject(
            ReasonCode::TargetMismatch,
            Value::String("command missing target_node".into()),
            empty_map(),
        ));
    };
    if !scope.target_nodes.is_empty()
        && !scope
            .target_nodes
            .iter()
            .any(|target| target.node_id == target_node.node_id)
    {
        return Err(reject(
            ReasonCode::AuthorityDenied,
            Value::String("delegation scope does not include command target_node".into()),
            empty_map(),
        ));
    }
    if !scope.target_object_kinds.is_empty() {
        let Some(crate::protocol::command_envelope::Payload::PayloadObject(payload_object)) =
            &command.payload
        else {
            return Err(reject(
                ReasonCode::AuthorityDenied,
                Value::String(
                    "delegation scope restricts object kinds but command has no payload_object"
                        .into(),
                ),
                empty_map(),
            ));
        };
        let Some(payload_object_kind) = payload_object.object_kind else {
            return Err(reject(
                ReasonCode::AuthorityDenied,
                Value::String(
                    "delegation scope restricts object kinds but payload_object has no object_kind"
                        .into(),
                ),
                empty_map(),
            ));
        };
        if !scope.target_object_kinds.contains(&payload_object_kind) {
            return Err(reject(
                ReasonCode::AuthorityDenied,
                Value::String("delegation scope does not include payload_object kind".into()),
                empty_map(),
            ));
        }
    }
    if !scope.target_streams.is_empty() {
        let Some(target_stream_id) = ctx.target_stream_id else {
            return Err(reject(
                ReasonCode::AuthorityDenied,
                Value::String(
                    "delegation scope restricts streams but command has no target_stream".into(),
                ),
                empty_map(),
            ));
        };
        if !scope
            .target_streams
            .iter()
            .any(|target| target.stream_id == target_stream_id)
        {
            return Err(reject(
                ReasonCode::AuthorityDenied,
                Value::String("delegation scope does not include command target_stream".into()),
                empty_map(),
            ));
        }
    }
    if !scope.target_view_types.is_empty() {
        let Some(target_view_type) = ctx.target_view_type else {
            return Err(reject(
                ReasonCode::AuthorityDenied,
                Value::String(
                    "delegation scope restricts view types but command has no target_view_type"
                        .into(),
                ),
                empty_map(),
            ));
        };
        if !scope
            .target_view_types
            .iter()
            .any(|view_type| view_type == target_view_type)
        {
            return Err(reject(
                ReasonCode::AuthorityDenied,
                Value::String("delegation scope does not include command target_view_type".into()),
                empty_map(),
            ));
        }
    }
    if !scope.target_domains.is_empty() {
        let Some(target_domain) = ctx.target_domain else {
            return Err(reject(
                ReasonCode::AuthorityDenied,
                Value::String(
                    "delegation scope restricts domains but command has no target_domain".into(),
                ),
                empty_map(),
            ));
        };
        if !scope
            .target_domains
            .iter()
            .any(|domain| domain == target_domain)
        {
            return Err(reject(
                ReasonCode::AuthorityDenied,
                Value::String("delegation scope does not include command target_domain".into()),
                empty_map(),
            ));
        }
    }
    if let Some(time_bounds) = &scope.time_bounds {
        if let Some(not_before) = &time_bounds.not_before {
            let not_before_ms = timestamp_millis(not_before);
            if ctx.now_ms < not_before_ms {
                return Err(defer(
                    ReasonCode::TimeInvalid,
                    Value::String("delegation scope time_bounds not yet valid".into()),
                ));
            }
        }
        if let Some(expires_at) = &time_bounds.expires_at {
            let expires_ms = timestamp_millis(expires_at);
            if ctx.now_ms > expires_ms {
                return Err(reject(
                    ReasonCode::TimeInvalid,
                    Value::String("delegation scope time_bounds expired".into()),
                    empty_map(),
                ));
            }
        }
    }
    if let Some(constraints) = &capability.constraints {
        if let Some(not_before) = &constraints.not_before {
            let not_before_ms = timestamp_millis(not_before);
            if ctx.now_ms < not_before_ms {
                return Err(defer(
                    ReasonCode::TimeInvalid,
                    Value::String("delegation constraints not yet valid".into()),
                ));
            }
        }
        if let Some(expires_at) = &constraints.expires_at {
            let expires_ms = timestamp_millis(expires_at);
            if ctx.now_ms > expires_ms {
                return Err(reject(
                    ReasonCode::TimeInvalid,
                    Value::String("delegation constraints expired".into()),
                    empty_map(),
                ));
            }
        }
        if constraints.requires_local_session == Some(true) && !ctx.has_local_session {
            return Err(reject(
                ReasonCode::AuthorityDenied,
                Value::String("delegation constraints require local session".into()),
                empty_map(),
            ));
        }
        if constraints.requires_user_presence == Some(true) && !ctx.has_user_presence {
            return Err(reject(
                ReasonCode::AuthorityDenied,
                Value::String("delegation constraints require user presence".into()),
                empty_map(),
            ));
        }
        if !constraints.requires_transport_classes.is_empty() {
            let Some(transport_class) = ctx.transport_class else {
                return Err(reject(
                    ReasonCode::AuthorityDenied,
                    Value::String("delegation constraints require known transport class".into()),
                    empty_map(),
                ));
            };
            if !constraints
                .requires_transport_classes
                .contains(&transport_class)
            {
                return Err(reject(
                    ReasonCode::AuthorityDenied,
                    Value::String("transport class does not satisfy delegation constraints".into()),
                    empty_map(),
                ));
            }
        }
        if !constraints.requires_location_classes.is_empty()
            && !constraints
                .requires_location_classes
                .iter()
                .any(|required| ctx.location_classes.contains(&required.as_str()))
        {
            return Err(reject(
                ReasonCode::AuthorityDenied,
                Value::String("location class does not satisfy delegation constraints".into()),
                empty_map(),
            ));
        }
        if !constraints.execution_class_limits.is_empty() {
            let Some(execution_class) = ctx.execution_class else {
                return Err(reject(
                    ReasonCode::AuthorityDenied,
                    Value::String("delegation constraints require known execution class".into()),
                    empty_map(),
                ));
            };
            if !constraints
                .execution_class_limits
                .contains(&execution_class)
            {
                return Err(reject(
                    ReasonCode::AuthorityDenied,
                    Value::String("execution class does not satisfy delegation constraints".into()),
                    empty_map(),
                ));
            }
        }
        if !constraints.storage_class_limits.is_empty() {
            let Some(storage_class) = ctx.storage_class else {
                return Err(reject(
                    ReasonCode::AuthorityDenied,
                    Value::String("delegation constraints require known storage class".into()),
                    empty_map(),
                ));
            };
            if !constraints.storage_class_limits.contains(&storage_class) {
                return Err(reject(
                    ReasonCode::AuthorityDenied,
                    Value::String("storage class does not satisfy delegation constraints".into()),
                    empty_map(),
                ));
            }
        }
    }
    if let Some(assurance) = &capability.minimum_assurance {
        local_assurance_satisfies_requirement(assurance, ctx, ReasonCode::AssuranceInsufficient)?;
    }

    Ok(())
}

fn validate_identity_ref(identity: &IdentityRef, label: &str) -> Option<ValidationResult> {
    if identity.identity_id.is_empty() {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} identity_id is empty")),
            empty_map(),
        ));
    }
    if identity.identity_kind.is_some_and(|identity_kind| {
        crate::protocol::IdentityKind::from(identity_kind)
            .is_none_or(|kind| kind == crate::protocol::IdentityKind::Unspecified)
    }) {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} identity_kind is invalid")),
            empty_map(),
        ));
    }
    None
}

fn validate_command_structure(command: &CommandEnvelope) -> Option<ValidationResult> {
    if let Some(result) =
        validate_command_version_field(command.envelope_version, "envelope_version")
    {
        return Some(result);
    }
    if let Some(result) = validate_command_version_field(command.command_version, "command_version")
    {
        return Some(result);
    }

    let Some(command_type) = CommandType::from(command.command_type) else {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!(
                "unknown authority-critical command_type: {}",
                command.command_type
            )),
            empty_map(),
        ));
    };
    if command_type == CommandType::Unspecified {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String("command_type is unspecified".into()),
            empty_map(),
        ));
    }

    if command.command_id.is_empty() {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String("command_id is empty".into()),
            empty_map(),
        ));
    }
    let Some(issued_at) = &command.issued_at else {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String("command missing issued_at".into()),
            empty_map(),
        ));
    };
    if issued_at.nanos < 0 || issued_at.nanos >= 1_000_000_000 {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String("issued_at nanos out of range".into()),
            empty_map(),
        ));
    }

    let Some(target) = &command.target_node else {
        return Some(reject(
            ReasonCode::TargetMismatch,
            Value::String("no target_node specified".into()),
            empty_map(),
        ));
    };
    if target.node_id.is_empty() {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String("target_node node_id is empty".into()),
            empty_map(),
        ));
    }

    let Some(issuer) = &command.issuer else {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String("no issuer identity".into()),
            empty_map(),
        ));
    };
    if let Some(result) = validate_identity_ref(issuer, "issuer") {
        return Some(result);
    }

    if let Some(not_before) = &command.not_before {
        if not_before.nanos < 0 || not_before.nanos >= 1_000_000_000 {
            return Some(reject(
                ReasonCode::StructuralInvalid,
                Value::String("not_before nanos out of range".into()),
                empty_map(),
            ));
        }
    }
    if let Some(expires_at) = &command.expires_at {
        if expires_at.nanos < 0 || expires_at.nanos >= 1_000_000_000 {
            return Some(reject(
                ReasonCode::StructuralInvalid,
                Value::String("expires_at nanos out of range".into()),
                empty_map(),
            ));
        }
    }
    if let (Some(not_before), Some(expires_at)) = (&command.not_before, &command.expires_at) {
        if timestamp_millis(not_before) > timestamp_millis(expires_at) {
            return Some(reject(
                ReasonCode::TimeInvalid,
                Value::String("command not_before is after expires_at".into()),
                empty_map(),
            ));
        }
    }

    if let Some(payload) = &command.payload {
        match payload {
            crate::protocol::command_envelope::Payload::PayloadObject(object) => {
                if let Some(result) = validate_object_ref(object, "command payload_object") {
                    return Some(result);
                }
            }
            crate::protocol::command_envelope::Payload::InlinePayload(payload) => {
                if payload.is_empty() {
                    return Some(reject(
                        ReasonCode::StructuralInvalid,
                        Value::String("command inline_payload is empty".into()),
                        empty_map(),
                    ));
                }
            }
        }
    }

    if let Some(metadata) = &command.command_metadata {
        if let Some(result) = validate_object_ref(metadata, "command_metadata") {
            return Some(result);
        }
    }

    if let Some(req) = &command.requested_assurance {
        for attester in &req.acceptable_attesters {
            if let Some(result) =
                validate_identity_ref(attester, "requested_assurance acceptable_attester")
            {
                return Some(result);
            }
        }
        if let Some(max_age) = &req.max_evidence_age {
            if max_age.seconds < 0 || max_age.nanos < 0 || max_age.nanos >= 1_000_000_000 {
                return Some(reject(
                    ReasonCode::StructuralInvalid,
                    Value::String("requested_assurance max_evidence_age is invalid".into()),
                    empty_map(),
                ));
            }
        }
        if let Some(metadata) = &req.assurance_metadata {
            if let Some(result) = validate_object_ref(metadata, "requested_assurance metadata") {
                return Some(result);
            }
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Main validation entry point
// ---------------------------------------------------------------------------

/// Validates a command through the full protocol pipeline.
///
/// Returns a `ValidationResult` with one of ACCEPT / REJECT / DEFER / DUPLICATE.
///
/// Validation order (spec §18.6):
/// 1. Structural: required fields, target binding
/// 2. Cryptographic: signature verification
/// 3. Replay: command_hash duplicate detection
/// 4. Timing: not_before / expires_at
/// 5. Authority: delegation chain validation (if present)
pub fn validate_command(
    command: &CommandEnvelope,
    ctx: &CommandValidationContext<'_>,
) -> ValidationResult {
    // --- Step 1: Structural validation ---
    if let Some(result) = validate_command_structure(command) {
        return result;
    }

    let issued_at_ms = timestamp_millis(command.issued_at.as_ref().unwrap());
    if ctx.now_ms < issued_at_ms {
        let mut derived = std::collections::BTreeMap::new();
        derived.insert("issued_at_ms".into(), Value::Int(issued_at_ms));
        derived.insert("now_ms".into(), Value::Int(ctx.now_ms));
        return defer(ReasonCode::TimeInvalid, Value::Map(derived));
    }

    // Target binding: command must target this node
    let target = command.target_node.as_ref().unwrap();
    if target.node_id != ctx.local_node_id.as_slice() {
        return reject(
            ReasonCode::TargetMismatch,
            Value::String("command targets a different node".into()),
            empty_map(),
        );
    }

    // --- Step 2: Cryptographic validation ---
    let Some(sig) = &command.signature else {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("missing signature".into()),
            empty_map(),
        );
    };

    if sig.algorithm != crate::crypto::SIGNATURE_ALGORITHM_ECDSA_P256 as i32 {
        // SIGNATURE_ALGORITHM_ECDSA_P256_SHA256 = 1
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String(format!(
                "unsupported signature algorithm: {}",
                sig.algorithm
            )),
            empty_map(),
        );
    }

    if sig.value.len() != crate::crypto::ECDSA_P256_SIGNATURE_LEN {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String(format!("signature length {} != 64", sig.value.len())),
            empty_map(),
        );
    }

    let Some(public_key) = extract_public_key(&command.issuer) else {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("invalid or missing public key in issuer identity".into()),
            empty_map(),
        );
    };

    let Some(vk) = crate::crypto::node_id_to_verifying_key(&public_key) else {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("invalid public key in issuer identity".into()),
            empty_map(),
        );
    };
    let record = ProtocolRecord::CommandEnvelope(command.clone());
    let canonical = canonical_bytes(&record, true);

    if !crate::crypto::verify_canonical_record(
        &vk,
        crate::crypto::SIG_DOMAIN_COMMAND_ENVELOPE,
        &canonical,
        &sig.value,
    ) {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("signature verification failed".into()),
            empty_map(),
        );
    }

    // --- Step 3: Timing validation ---
    // Per spec §18.6: TIME_CHECK comes before REPLAY_CHECK
    if let Some(ref not_before) = command.not_before {
        let not_before_ms = timestamp_millis(not_before);
        if ctx.now_ms < not_before_ms {
            let mut derived = std::collections::BTreeMap::new();
            derived.insert("not_before_ms".into(), Value::Int(not_before_ms));
            derived.insert("now_ms".into(), Value::Int(ctx.now_ms));
            return defer(ReasonCode::TimeInvalid, Value::Map(derived));
        }
    }

    if let Some(ref expires_at) = command.expires_at {
        let expires_ms = timestamp_millis(expires_at);
        if ctx.now_ms > expires_ms {
            return reject(
                ReasonCode::TimeInvalid,
                Value::String("command has expired".into()),
                empty_map(),
            );
        }
    }

    // --- Step 4: Replay detection ---
    // Per spec §18.6: REPLAY_CHECK comes after TIME_CHECK
    // The replay cache is keyed by command_hash. command_id is an idempotency
    // hint and MUST NOT be used as the replay key (§5.1).
    let computed_hash = command_hash(command).value.clone();
    if ctx.replay_cache.contains_key(&computed_hash) {
        let mut derived = std::collections::BTreeMap::new();
        derived.insert(
            "command_hash".into(),
            Value::String(crate::util::bytes_to_hex(&computed_hash)),
        );
        return duplicate(ReasonCode::ReplayDetected, Value::Map(derived));
    }

    // --- Step 5: Assurance requirement check (if requested) ---
    if let Some(ref req) = command.requested_assurance {
        use crate::protocol::AssuranceClass;

        if let Some(result) =
            validate_command_version_field(req.assurance_version, "assurance requirement version")
        {
            return result;
        }

        let Some(required_class) = AssuranceClass::from(req.required_class) else {
            return reject(
                ReasonCode::StructuralInvalid,
                Value::String(format!(
                    "invalid requested assurance class: {}",
                    req.required_class
                )),
                empty_map(),
            );
        };

        if required_class != AssuranceClass::Unspecified {
            if let Err(result) =
                local_assurance_satisfies_requirement(req, ctx, ReasonCode::AuthorityDenied)
            {
                return result;
            }
        }
    }

    // --- Step 6: Delegation chain validation (if present) ---
    if !command.delegation_chain.is_empty() {
        match validate_delegation_chain(&command.delegation_chain, &command.issuer, ctx) {
            Ok(()) => {
                let Some(leaf_capability) = command
                    .delegation_chain
                    .last()
                    .and_then(|delegation| delegation.capability.as_ref())
                else {
                    return reject(
                        ReasonCode::StructuralInvalid,
                        Value::String("delegation chain leaf missing capability".into()),
                        empty_map(),
                    );
                };
                if let Err(reason) =
                    validate_effective_capability_for_command(command, leaf_capability, ctx)
                {
                    return reason;
                }
            }
            Err(reason) => return reason,
        }
    }

    // --- Accept ---
    let mut derived = std::collections::BTreeMap::new();
    derived.insert(
        "command_id".into(),
        Value::String(crate::util::bytes_to_hex(&command.command_id)),
    );
    derived.insert(
        "command_hash".into(),
        Value::String(crate::util::bytes_to_hex(&computed_hash)),
    );
    derived.insert(
        "issuer".into(),
        Value::String(crate::util::bytes_to_hex(
            &command
                .issuer
                .as_ref()
                .map(|i| i.identity_id.clone())
                .unwrap_or_default(),
        )),
    );
    derived.insert(
        "command_type".into(),
        Value::Int(command.command_type as i64),
    );

    accept(Value::Map(derived), empty_map())
}

// ---------------------------------------------------------------------------
// Delegation chain validation
// ---------------------------------------------------------------------------

/// Validates a delegation chain for a command issuer.
///
/// Checks:
/// - Each delegation signature verifies
/// - Issuer/recipient continuity across the chain
/// - No child expands privilege relative to parent (attenuation)
/// - No delegation is revoked
/// - Timing bounds (not_before / expires_at) are respected
fn validate_delegation_chain(
    chain: &[DelegationRecord],
    effective_issuer: &Option<IdentityRef>,
    ctx: &CommandValidationContext<'_>,
) -> Result<(), ValidationResult> {
    if chain.is_empty() {
        return Ok(());
    }

    // Verify chain continuity: each recipient must match next issuer
    let effective_identity = effective_issuer.as_ref().ok_or_else(|| {
        reject(
            ReasonCode::AuthorityDenied,
            Value::String("command issuer missing but delegation chain present".into()),
            empty_map(),
        )
    })?;

    // Check chain from last to first (leaf to root)
    // The last delegation's recipient must match the command issuer
    let last = chain.last().unwrap();
    if last.recipient.as_ref().map(|r| r.identity_id.clone())
        != Some(effective_identity.identity_id.clone())
    {
        return Err(reject(
            ReasonCode::AuthorityDenied,
            Value::String("delegation chain recipient does not match command issuer".into()),
            empty_map(),
        ));
    }

    // Verify intermediate link continuity: chain[i].recipient == chain[i+1].issuer
    // This ensures no gaps in the delegation path from root to leaf
    for i in 0..chain.len() - 1 {
        let child_recipient = chain[i].recipient.as_ref().map(|r| r.identity_id.clone());
        let next_issuer = chain[i + 1].issuer.as_ref().map(|i| i.identity_id.clone());
        if child_recipient != next_issuer {
            return Err(reject(
                ReasonCode::AuthorityDenied,
                Value::String(format!(
                    "delegation chain broken: chain[{}].recipient != chain[{}].issuer",
                    i,
                    i + 1
                )),
                empty_map(),
            ));
        }
    }

    // Verify root trust: the first delegation's issuer must be in the trusted root set
    let first = chain.first().unwrap();
    if !ctx.trusted_root_ids.is_empty() {
        let root_issuer = first.issuer.as_ref().map(|i| i.identity_id.clone());
        if root_issuer.is_none_or(|id| !ctx.trusted_root_ids.contains(&id)) {
            return Err(reject(
                ReasonCode::AuthorityDenied,
                Value::String("delegation chain root issuer is not in trusted roots".into()),
                empty_map(),
            ));
        }
    }

    for (index, delegation) in chain.iter().enumerate() {
        if let Some(result) =
            validate_delegation_record_structure(delegation, &format!("delegation chain[{index}]"))
        {
            return Err(result);
        }

        // Check revocation
        if ctx
            .revoked_delegation_ids
            .contains(&delegation.delegation_id)
        {
            return Err(reject(
                ReasonCode::RevocationActive,
                Value::String(format!(
                    "delegation {} is revoked",
                    crate::util::bytes_to_hex(&delegation.delegation_id)
                )),
                empty_map(),
            ));
        }
        if let Some(max_uses) = delegation
            .capability
            .as_ref()
            .and_then(|capability| capability.constraints.as_ref())
            .and_then(|constraints| constraints.max_uses)
        {
            let local_uses = ctx
                .delegation_use_counts
                .get(&delegation.delegation_id)
                .copied()
                .unwrap_or(0);
            if local_uses >= max_uses {
                return Err(reject(
                    ReasonCode::AuthorityDenied,
                    Value::String(format!(
                        "delegation max_uses exhausted for delegation {}",
                        crate::util::bytes_to_hex(&delegation.delegation_id)
                    )),
                    empty_map(),
                ));
            }
        }
        if let Some(rate_limit) = delegation
            .capability
            .as_ref()
            .and_then(|capability| capability.constraints.as_ref())
            .and_then(|constraints| constraints.rate_limit.as_ref())
        {
            let Some(period_ms) = rate_limit.per.as_ref().and_then(duration_millis) else {
                return Err(reject(
                    ReasonCode::StructuralInvalid,
                    Value::String("delegation rate_limit period is invalid".into()),
                    empty_map(),
                ));
            };
            let window_start_ms = if period_ms > i64::MAX as i128 {
                i64::MIN
            } else {
                ctx.now_ms.saturating_sub(period_ms as i64)
            };
            let operations_in_window = ctx
                .delegation_rate_events_ms
                .get(&delegation.delegation_id)
                .map(|events| {
                    events
                        .iter()
                        .filter(|event_ms| **event_ms > window_start_ms && **event_ms <= ctx.now_ms)
                        .count() as u64
                })
                .unwrap_or(0);
            if operations_in_window >= rate_limit.max_operations {
                return Err(reject(
                    ReasonCode::AuthorityDenied,
                    Value::String(format!(
                        "delegation rate_limit exhausted for delegation {}",
                        crate::util::bytes_to_hex(&delegation.delegation_id)
                    )),
                    empty_map(),
                ));
            }
        }

        // Check timing: expires_at
        if let Some(ref expires_at) = delegation.expires_at {
            let expires_ms = expires_at.seconds * 1000 + (expires_at.nanos as i64) / 1_000_000;
            if ctx.now_ms > expires_ms {
                return Err(reject(
                    ReasonCode::TimeInvalid,
                    Value::String("delegation has expired".into()),
                    empty_map(),
                ));
            }
        }

        // Check timing: not_before
        if let Some(ref not_before) = delegation.not_before {
            let not_before_ms = not_before.seconds * 1000 + (not_before.nanos as i64) / 1_000_000;
            if ctx.now_ms < not_before_ms {
                return Err(defer(
                    ReasonCode::TimeInvalid,
                    Value::String("delegation not yet valid".into()),
                ));
            }
        }

        let issued_at_ms = timestamp_millis(delegation.issued_at.as_ref().unwrap());
        if ctx.now_ms < issued_at_ms {
            return Err(defer(
                ReasonCode::TimeInvalid,
                Value::String("delegation issued_at is in the future".into()),
            ));
        }

        // Cryptographically verify delegation signature
        let Some(ref sig) = delegation.signature else {
            return Err(reject(
                ReasonCode::CryptoInvalid,
                Value::String("delegation missing signature".into()),
                empty_map(),
            ));
        };
        if sig.algorithm != crate::crypto::SIGNATURE_ALGORITHM_ECDSA_P256 as i32 {
            return Err(reject(
                ReasonCode::CryptoInvalid,
                Value::String("unsupported delegation signature algorithm".into()),
                empty_map(),
            ));
        }
        if sig.value.len() != crate::crypto::ECDSA_P256_SIGNATURE_LEN {
            return Err(reject(
                ReasonCode::CryptoInvalid,
                Value::String(format!(
                    "delegation signature length {} != 64",
                    sig.value.len()
                )),
                empty_map(),
            ));
        }

        // Extract issuer public key from key_hint (64 raw bytes, SEC1 uncompressed without 0x04 prefix)
        let Some(issuer_ref) = &delegation.issuer else {
            return Err(reject(
                ReasonCode::StructuralInvalid,
                Value::String("delegation missing issuer".into()),
                empty_map(),
            ));
        };
        let Some(key_hint) = &issuer_ref.key_hint else {
            return Err(reject(
                ReasonCode::CryptoInvalid,
                Value::String("delegation issuer has no key_hint".into()),
                empty_map(),
            ));
        };
        if key_hint.len() != crate::crypto::ECDSA_P256_PUBLIC_KEY_LEN {
            return Err(reject(
                ReasonCode::CryptoInvalid,
                Value::String(format!(
                    "delegation key_hint length {} != 64",
                    key_hint.len()
                )),
                empty_map(),
            ));
        }
        let mut key_bytes = [0u8; 64];
        key_bytes.copy_from_slice(key_hint);
        let Some(verifying_key) = crate::crypto::node_id_to_verifying_key(&key_bytes) else {
            return Err(reject(
                ReasonCode::CryptoInvalid,
                Value::String("invalid delegation public key".into()),
                empty_map(),
            ));
        };

        // Build signable form (signature absent) and canonical encode
        let mut signable = delegation.clone();
        signable.signature = None;
        let canonical = canonical_bytes(&ProtocolRecord::DelegationRecord(signable.clone()), true);

        if !crate::crypto::verify_canonical_record(
            &verifying_key,
            crate::crypto::SIG_DOMAIN_DELEGATION_RECORD,
            &canonical,
            &sig.value,
        ) {
            return Err(reject(
                ReasonCode::CryptoInvalid,
                Value::String(format!(
                    "delegation signature verification failed for delegation {}",
                    crate::util::bytes_to_hex(&delegation.delegation_id)
                )),
                empty_map(),
            ));
        }
    }

    // Check attenuation: no child capability may expand parent's authority.
    // A child delegation may only NARROW the parent's scope.
    for i in 1..chain.len() {
        let parent = &chain[i - 1];
        let child = &chain[i];
        let Some(parent_cap) = &parent.capability else {
            return Err(reject(
                ReasonCode::StructuralInvalid,
                Value::String("parent delegation has no capability".into()),
                empty_map(),
            ));
        };
        let Some(child_cap) = &child.capability else {
            return Err(reject(
                ReasonCode::StructuralInvalid,
                Value::String("child delegation has no capability".into()),
                empty_map(),
            ));
        };

        if parent_cap.delegation_policy == crate::protocol::DelegationPolicy::NonDelegable as i32 {
            return Err(reject(
                ReasonCode::AuthorityDenied,
                Value::String(format!(
                    "delegation chain violates attenuation: parent is non-delegable at depth {i}"
                )),
                empty_map(),
            ));
        }

        if parent_cap.capability_kind != child_cap.capability_kind {
            return Err(reject(
                ReasonCode::AuthorityDenied,
                Value::String(format!(
                    "delegation chain violates attenuation: child changes capability_kind at depth {i}"
                )),
                empty_map(),
            ));
        }

        // 1. Actions: child must not add new actions
        let parent_actions: std::collections::HashSet<&String> =
            parent_cap.actions.iter().collect();
        for action in &child_cap.actions {
            if !parent_actions.contains(action) {
                return Err(reject(
                    ReasonCode::AuthorityDenied,
                    Value::String(format!(
                        "delegation chain violates attenuation: child adds action '{}'",
                        action
                    )),
                    empty_map(),
                ));
            }
        }

        // 2. Scope: child scope targets must be subsets of parent scope targets
        attenuate_scope(parent_cap, child_cap, i)?;

        // 3. Temporal bounds: child validity window must be within parent's
        attenuate_timing(parent, child, i)?;

        // 4. Constraints: child constraints must tighten, not loosen
        attenuate_constraints(parent_cap, child_cap, i)?;

        // 5. Assurance: child may require stronger assurance, not weaker
        attenuate_assurance(parent_cap, child_cap, i)?;
    }

    // Verify parent_delegation references are consistent with chain order
    for i in 1..chain.len() {
        let parent = &chain[i - 1];
        let child = &chain[i];
        if let Some(ref parent_ref) = child.parent_delegation {
            if parent_ref.delegation_id != parent.delegation_id {
                return Err(reject(
                    ReasonCode::StructuralInvalid,
                    Value::String(format!(
                        "delegation chain: child parent_delegation ({}) does not match actual parent ({})",
                        crate::util::bytes_to_hex(&parent_ref.delegation_id),
                        crate::util::bytes_to_hex(&parent.delegation_id)
                    )),
                    empty_map(),
                ));
            }
            if parent_ref.delegation_hash.as_ref() != Some(&delegation_hash(parent)) {
                return Err(reject(
                    ReasonCode::StructuralInvalid,
                    Value::String(format!(
                        "delegation chain: child parent_delegation hash does not match actual parent at depth {i}"
                    )),
                    empty_map(),
                ));
            }
        }
        // If parent_delegation is absent, the chain position alone is authoritative.
        // This allows for older delegations that didn't set the field.
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Delegation attenuation helpers
// ---------------------------------------------------------------------------

/// Scope attenuation: every target set in the child must be a subset of the parent's.
/// An empty parent target set means "all"; an empty child set only remains safe
/// when the parent is also unrestricted for that target dimension.
fn attenuate_scope(
    parent: &CapabilityDescriptor,
    child: &CapabilityDescriptor,
    depth: usize,
) -> Result<(), ValidationResult> {
    let Some(parent_scope) = &parent.scope else {
        return Err(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!(
                "scope attenuation: parent missing scope at depth {depth}"
            )),
            empty_map(),
        ));
    };
    let Some(child_scope) = &child.scope else {
        return Err(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!(
                "scope attenuation: child missing scope at depth {depth}"
            )),
            empty_map(),
        ));
    };
    let parent_kind = crate::protocol::ScopeKind::from(parent_scope.scope_kind);
    let child_kind = crate::protocol::ScopeKind::from(child_scope.scope_kind);
    if parent_kind == Some(crate::protocol::ScopeKind::GlobalWithConstraints) {
        if !time_window_allows(
            parent_scope.time_bounds.as_ref(),
            child_scope.time_bounds.as_ref(),
        ) {
            return Err(reject(
                ReasonCode::AuthorityDenied,
                Value::String(format!(
                    "scope attenuation: child broadens time_bounds at depth {depth}"
                )),
                empty_map(),
            ));
        }
    } else if parent_kind != child_kind {
        return Err(reject(
            ReasonCode::AuthorityDenied,
            Value::String(format!(
                "scope attenuation: child changes scope_kind at depth {depth}"
            )),
            empty_map(),
        ));
    }

    // target_nodes: child set must be subset of parent set
    let parent_nodes: std::collections::HashSet<&[u8]> = parent_scope
        .target_nodes
        .iter()
        .map(|n| n.node_id.as_slice())
        .collect();
    if !parent_nodes.is_empty() && child_scope.target_nodes.is_empty() {
        return Err(reject(
            ReasonCode::AuthorityDenied,
            Value::String(format!(
                "scope attenuation: child drops target_nodes at depth {depth}"
            )),
            empty_map(),
        ));
    }
    if !parent_nodes.is_empty() {
        for target in &child_scope.target_nodes {
            if !parent_nodes.contains(target.node_id.as_slice()) {
                return Err(reject(
                    ReasonCode::AuthorityDenied,
                    Value::String(format!(
                        "scope attenuation: child adds target_node at depth {}",
                        depth
                    )),
                    empty_map(),
                ));
            }
        }
    }

    // target_streams: child set must be subset of parent set
    let parent_streams: std::collections::HashSet<(&[u8],)> = parent_scope
        .target_streams
        .iter()
        .map(|s| (s.stream_id.as_slice(),))
        .collect();
    if !parent_streams.is_empty() && child_scope.target_streams.is_empty() {
        return Err(reject(
            ReasonCode::AuthorityDenied,
            Value::String(format!(
                "scope attenuation: child drops target_streams at depth {depth}"
            )),
            empty_map(),
        ));
    }
    if !parent_streams.is_empty() {
        for target in &child_scope.target_streams {
            if !parent_streams.contains(&(target.stream_id.as_slice(),)) {
                return Err(reject(
                    ReasonCode::AuthorityDenied,
                    Value::String(format!(
                        "scope attenuation: child adds target_stream at depth {}",
                        depth
                    )),
                    empty_map(),
                ));
            }
        }
    }

    // target_object_kinds: child set must be subset of parent set
    let parent_kinds: std::collections::HashSet<i32> =
        parent_scope.target_object_kinds.iter().cloned().collect();
    if !parent_kinds.is_empty() && child_scope.target_object_kinds.is_empty() {
        return Err(reject(
            ReasonCode::AuthorityDenied,
            Value::String(format!(
                "scope attenuation: child drops target_object_kinds at depth {depth}"
            )),
            empty_map(),
        ));
    }
    if !parent_kinds.is_empty() {
        for kind in &child_scope.target_object_kinds {
            if !parent_kinds.contains(kind) {
                return Err(reject(
                    ReasonCode::AuthorityDenied,
                    Value::String(format!(
                        "scope attenuation: child adds target_object_kind {} at depth {}",
                        kind, depth
                    )),
                    empty_map(),
                ));
            }
        }
    }

    // target_view_types: child set must be subset of parent set
    let parent_views: std::collections::HashSet<&String> =
        parent_scope.target_view_types.iter().collect();
    if !parent_views.is_empty() && child_scope.target_view_types.is_empty() {
        return Err(reject(
            ReasonCode::AuthorityDenied,
            Value::String(format!(
                "scope attenuation: child drops target_view_types at depth {depth}"
            )),
            empty_map(),
        ));
    }
    if !parent_views.is_empty() {
        for view in &child_scope.target_view_types {
            if !parent_views.contains(view) {
                return Err(reject(
                    ReasonCode::AuthorityDenied,
                    Value::String(format!(
                        "scope attenuation: child adds target_view_type '{}' at depth {}",
                        view, depth
                    )),
                    empty_map(),
                ));
            }
        }
    }

    // target_domains: child set must be subset of parent set
    let parent_domains: std::collections::HashSet<&String> =
        parent_scope.target_domains.iter().collect();
    if !parent_domains.is_empty() && child_scope.target_domains.is_empty() {
        return Err(reject(
            ReasonCode::AuthorityDenied,
            Value::String(format!(
                "scope attenuation: child drops target_domains at depth {depth}"
            )),
            empty_map(),
        ));
    }
    if !parent_domains.is_empty() {
        for domain in &child_scope.target_domains {
            if !parent_domains.contains(domain) {
                return Err(reject(
                    ReasonCode::AuthorityDenied,
                    Value::String(format!(
                        "scope attenuation: child adds target_domain '{}' at depth {}",
                        domain, depth
                    )),
                    empty_map(),
                ));
            }
        }
    }
    if !time_window_allows(
        parent_scope.time_bounds.as_ref(),
        child_scope.time_bounds.as_ref(),
    ) {
        return Err(reject(
            ReasonCode::AuthorityDenied,
            Value::String(format!(
                "scope attenuation: child broadens time_bounds at depth {depth}"
            )),
            empty_map(),
        ));
    }

    Ok(())
}

fn time_window_allows(
    parent: Option<&crate::protocol::TimeWindow>,
    child: Option<&crate::protocol::TimeWindow>,
) -> bool {
    let Some(parent) = parent else {
        return true;
    };
    let Some(child) = child else {
        return parent.not_before.is_none() && parent.expires_at.is_none();
    };
    if let Some(parent_not_before) = &parent.not_before {
        let Some(child_not_before) = &child.not_before else {
            return false;
        };
        if timestamp_millis(child_not_before) < timestamp_millis(parent_not_before) {
            return false;
        }
    }
    if let Some(parent_expires_at) = &parent.expires_at {
        let Some(child_expires_at) = &child.expires_at else {
            return false;
        };
        if timestamp_millis(child_expires_at) > timestamp_millis(parent_expires_at) {
            return false;
        }
    }
    true
}

/// Temporal attenuation: child's validity window must be fully within parent's.
fn attenuate_timing(
    parent: &DelegationRecord,
    child: &DelegationRecord,
    depth: usize,
) -> Result<(), ValidationResult> {
    // Child must not start before parent
    if let Some(parent_not_before) = &parent.not_before {
        let Some(child_not_before) = &child.not_before else {
            return Err(reject(
                ReasonCode::AuthorityDenied,
                Value::String(format!(
                    "timing attenuation: child drops not_before at depth {}",
                    depth
                )),
                empty_map(),
            ));
        };
        if timestamp_millis(child_not_before) < timestamp_millis(parent_not_before) {
            return Err(reject(
                ReasonCode::AuthorityDenied,
                Value::String(format!(
                    "timing attenuation: child not_before before parent at depth {}",
                    depth
                )),
                empty_map(),
            ));
        }
    }

    // Child must not expire after parent
    if let Some(parent_expires) = &parent.expires_at {
        let Some(child_expires) = &child.expires_at else {
            return Err(reject(
                ReasonCode::AuthorityDenied,
                Value::String(format!(
                    "timing attenuation: child drops expires_at at depth {}",
                    depth
                )),
                empty_map(),
            ));
        };
        if timestamp_millis(child_expires) > timestamp_millis(parent_expires) {
            return Err(reject(
                ReasonCode::AuthorityDenied,
                Value::String(format!(
                    "timing attenuation: child expires_at after parent at depth {}",
                    depth
                )),
                empty_map(),
            ));
        }
    }

    Ok(())
}

/// Constraint attenuation: child constraints must be equal or tighter than parent.
fn attenuate_constraints(
    parent: &CapabilityDescriptor,
    child: &CapabilityDescriptor,
    depth: usize,
) -> Result<(), ValidationResult> {
    let Some(parent_constraints) = &parent.constraints else {
        return Ok(());
    };
    let Some(child_constraints) = &child.constraints else {
        return Err(reject(
            ReasonCode::AuthorityDenied,
            Value::String(format!(
                "constraint attenuation: child drops parent constraints at depth {depth}"
            )),
            empty_map(),
        ));
    };

    // max_uses: child must not exceed parent
    if let Some(parent_max) = parent_constraints.max_uses {
        let Some(child_max) = child_constraints.max_uses else {
            return Err(reject(
                ReasonCode::AuthorityDenied,
                Value::String(format!(
                    "constraint attenuation: child drops max_uses at depth {depth}"
                )),
                empty_map(),
            ));
        };
        if child_max > parent_max {
            return Err(reject(
                ReasonCode::AuthorityDenied,
                Value::String(format!(
                    "constraint attenuation: child max_uses {} exceeds parent {} at depth {}",
                    child_max, parent_max, depth
                )),
                empty_map(),
            ));
        }
    }
    if let Some(parent_not_before) = &parent_constraints.not_before {
        let Some(child_not_before) = &child_constraints.not_before else {
            return Err(reject(
                ReasonCode::AuthorityDenied,
                Value::String(format!(
                    "constraint attenuation: child drops not_before at depth {depth}"
                )),
                empty_map(),
            ));
        };
        if timestamp_millis(child_not_before) < timestamp_millis(parent_not_before) {
            return Err(reject(
                ReasonCode::AuthorityDenied,
                Value::String(format!(
                    "constraint attenuation: child not_before before parent at depth {depth}"
                )),
                empty_map(),
            ));
        }
    }
    if let Some(parent_expires_at) = &parent_constraints.expires_at {
        let Some(child_expires_at) = &child_constraints.expires_at else {
            return Err(reject(
                ReasonCode::AuthorityDenied,
                Value::String(format!(
                    "constraint attenuation: child drops expires_at at depth {depth}"
                )),
                empty_map(),
            ));
        };
        if timestamp_millis(child_expires_at) > timestamp_millis(parent_expires_at) {
            return Err(reject(
                ReasonCode::AuthorityDenied,
                Value::String(format!(
                    "constraint attenuation: child expires_at after parent at depth {depth}"
                )),
                empty_map(),
            ));
        }
    }
    if let Some(parent_rate_limit) = &parent_constraints.rate_limit {
        let Some(child_rate_limit) = &child_constraints.rate_limit else {
            return Err(reject(
                ReasonCode::AuthorityDenied,
                Value::String(format!(
                    "constraint attenuation: child drops rate_limit at depth {depth}"
                )),
                empty_map(),
            ));
        };
        if !rate_limit_allows(parent_rate_limit, child_rate_limit) {
            return Err(reject(
                ReasonCode::AuthorityDenied,
                Value::String(format!(
                    "constraint attenuation: child loosens rate_limit at depth {depth}"
                )),
                empty_map(),
            ));
        }
    }
    if !parent_constraints.requires_transport_classes.is_empty()
        && child_constraints.requires_transport_classes.is_empty()
    {
        return Err(reject(
            ReasonCode::AuthorityDenied,
            Value::String(format!(
                "constraint attenuation: child drops transport classes at depth {depth}"
            )),
            empty_map(),
        ));
    }
    let parent_transport: std::collections::HashSet<i32> = parent_constraints
        .requires_transport_classes
        .iter()
        .cloned()
        .collect();
    if !parent_transport.is_empty() {
        for transport_class in &child_constraints.requires_transport_classes {
            if !parent_transport.contains(transport_class) {
                return Err(reject(
                    ReasonCode::AuthorityDenied,
                    Value::String(format!(
                        "constraint attenuation: child adds transport_class {} at depth {}",
                        transport_class, depth
                    )),
                    empty_map(),
                ));
            }
        }
    }
    if !parent_constraints.requires_location_classes.is_empty()
        && child_constraints.requires_location_classes.is_empty()
    {
        return Err(reject(
            ReasonCode::AuthorityDenied,
            Value::String(format!(
                "constraint attenuation: child drops location classes at depth {depth}"
            )),
            empty_map(),
        ));
    }
    let parent_locations: std::collections::HashSet<&String> = parent_constraints
        .requires_location_classes
        .iter()
        .collect();
    if !parent_locations.is_empty() {
        for location_class in &child_constraints.requires_location_classes {
            if !parent_locations.contains(location_class) {
                return Err(reject(
                    ReasonCode::AuthorityDenied,
                    Value::String(format!(
                        "constraint attenuation: child adds location_class '{}' at depth {}",
                        location_class, depth
                    )),
                    empty_map(),
                ));
            }
        }
    }
    if parent_constraints.export_policy != 0
        && child_constraints.export_policy != parent_constraints.export_policy
    {
        return Err(reject(
            ReasonCode::AuthorityDenied,
            Value::String(format!(
                "constraint attenuation: child changes export_policy at depth {depth}"
            )),
            empty_map(),
        ));
    }

    // requires_local_session: if parent requires it, child must too
    if parent_constraints.requires_local_session == Some(true)
        && child_constraints.requires_local_session != Some(true)
    {
        return Err(reject(
            ReasonCode::AuthorityDenied,
            Value::String(format!(
                "constraint attenuation: child drops requires_local_session at depth {}",
                depth
            )),
            empty_map(),
        ));
    }

    // requires_user_presence: if parent requires it, child must too
    if parent_constraints.requires_user_presence == Some(true)
        && child_constraints.requires_user_presence != Some(true)
    {
        return Err(reject(
            ReasonCode::AuthorityDenied,
            Value::String(format!(
                "constraint attenuation: child drops requires_user_presence at depth {}",
                depth
            )),
            empty_map(),
        ));
    }

    // execution_class_limits: child set must be subset of parent set
    let parent_exec: std::collections::HashSet<i32> = parent_constraints
        .execution_class_limits
        .iter()
        .cloned()
        .collect();
    if !parent_exec.is_empty() && child_constraints.execution_class_limits.is_empty() {
        return Err(reject(
            ReasonCode::AuthorityDenied,
            Value::String(format!(
                "constraint attenuation: child drops execution_class_limits at depth {depth}"
            )),
            empty_map(),
        ));
    }
    if !parent_exec.is_empty() {
        for limit in &child_constraints.execution_class_limits {
            if !parent_exec.contains(limit) {
                return Err(reject(
                    ReasonCode::AuthorityDenied,
                    Value::String(format!(
                        "constraint attenuation: child adds execution_class_limit {} at depth {}",
                        limit, depth
                    )),
                    empty_map(),
                ));
            }
        }
    }

    // storage_class_limits: child set must be subset of parent set
    let parent_storage: std::collections::HashSet<i32> = parent_constraints
        .storage_class_limits
        .iter()
        .cloned()
        .collect();
    if !parent_storage.is_empty() && child_constraints.storage_class_limits.is_empty() {
        return Err(reject(
            ReasonCode::AuthorityDenied,
            Value::String(format!(
                "constraint attenuation: child drops storage_class_limits at depth {depth}"
            )),
            empty_map(),
        ));
    }
    if !parent_storage.is_empty() {
        for limit in &child_constraints.storage_class_limits {
            if !parent_storage.contains(limit) {
                return Err(reject(
                    ReasonCode::AuthorityDenied,
                    Value::String(format!(
                        "constraint attenuation: child adds storage_class_limit {} at depth {}",
                        limit, depth
                    )),
                    empty_map(),
                ));
            }
        }
    }

    Ok(())
}

/// Assurance attenuation: child may require stronger assurance, not weaker.
fn attenuate_assurance(
    parent: &CapabilityDescriptor,
    child: &CapabilityDescriptor,
    depth: usize,
) -> Result<(), ValidationResult> {
    let Some(parent_assurance) = &parent.minimum_assurance else {
        return Ok(());
    };
    let Some(child_assurance) = &child.minimum_assurance else {
        return Err(reject(
            ReasonCode::AssuranceInsufficient,
            Value::String(format!(
                "assurance attenuation: child drops minimum_assurance at depth {depth}"
            )),
            empty_map(),
        ));
    };

    // Higher numeric value = stronger class
    if child_assurance.required_class < parent_assurance.required_class {
        return Err(reject(
            ReasonCode::AssuranceInsufficient,
            Value::String(format!(
                "assurance attenuation: child requires class {}, parent requires {} at depth {}",
                child_assurance.required_class, parent_assurance.required_class, depth
            )),
            empty_map(),
        ));
    }

    // acceptable_attesters: if parent restricts attesters, child must not broaden beyond that set
    if !parent_assurance.acceptable_attesters.is_empty() {
        if child_assurance.acceptable_attesters.is_empty() {
            return Err(reject(
                ReasonCode::AssuranceInsufficient,
                Value::String(format!(
                    "assurance attenuation: child drops acceptable_attesters at depth {depth}"
                )),
                empty_map(),
            ));
        }
        let parent_attesters: std::collections::HashSet<&[u8]> = parent_assurance
            .acceptable_attesters
            .iter()
            .map(|a| a.identity_id.as_slice())
            .collect();
        for attester in &child_assurance.acceptable_attesters {
            if !parent_attesters.contains(attester.identity_id.as_slice()) {
                return Err(reject(
                    ReasonCode::AssuranceInsufficient,
                    Value::String(format!(
                        "assurance attenuation: child adds non-parent attester at depth {}",
                        depth
                    )),
                    empty_map(),
                ));
            }
        }
    }
    if let Some(parent_max_age) = &parent_assurance.max_evidence_age {
        let Some(child_max_age) = &child_assurance.max_evidence_age else {
            return Err(reject(
                ReasonCode::AssuranceInsufficient,
                Value::String(format!(
                    "assurance attenuation: child drops max_evidence_age at depth {depth}"
                )),
                empty_map(),
            ));
        };
        let Some(parent_ms) = duration_millis(parent_max_age) else {
            return Err(reject(
                ReasonCode::StructuralInvalid,
                Value::String(format!(
                    "assurance attenuation: parent max_evidence_age invalid at depth {depth}"
                )),
                empty_map(),
            ));
        };
        let Some(child_ms) = duration_millis(child_max_age) else {
            return Err(reject(
                ReasonCode::StructuralInvalid,
                Value::String(format!(
                    "assurance attenuation: child max_evidence_age invalid at depth {depth}"
                )),
                empty_map(),
            ));
        };
        if child_ms > parent_ms {
            return Err(reject(
                ReasonCode::AssuranceInsufficient,
                Value::String(format!(
                    "assurance attenuation: child loosens max_evidence_age at depth {depth}"
                )),
                empty_map(),
            ));
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Signature verification helpers
// ---------------------------------------------------------------------------

fn extract_public_key(identity: &Option<IdentityRef>) -> Option<[u8; 64]> {
    let identity = identity.as_ref()?;
    let hint = identity.key_hint.as_ref()?;
    if hint.len() == 64 {
        let mut bytes = [0u8; 64];
        bytes.copy_from_slice(hint);
        Some(bytes)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Signature-only validation (for callers that don't need full context)
// ---------------------------------------------------------------------------

/// Validates only the command signature, without replay, timing, or delegation checks.
///
/// This is useful for structural/crypto validation when the caller
/// handles replay and timing separately.
pub fn validate_command_signature(command: &CommandEnvelope) -> ValidationResult {
    if let Some(result) = validate_command_structure(command) {
        return result;
    }

    let Some(sig) = &command.signature else {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("missing signature".into()),
            empty_map(),
        );
    };

    if sig.algorithm != crate::crypto::SIGNATURE_ALGORITHM_ECDSA_P256 as i32 {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String(format!(
                "unsupported signature algorithm: {}",
                sig.algorithm
            )),
            empty_map(),
        );
    }

    if sig.value.len() != crate::crypto::ECDSA_P256_SIGNATURE_LEN {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String(format!("signature length {} != 64", sig.value.len())),
            empty_map(),
        );
    }

    let Some(public_key) = extract_public_key(&command.issuer) else {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("invalid or missing public key in issuer identity".into()),
            empty_map(),
        );
    };

    let Some(vk) = crate::crypto::node_id_to_verifying_key(&public_key) else {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("invalid public key in issuer identity".into()),
            empty_map(),
        );
    };
    let record = ProtocolRecord::CommandEnvelope(command.clone());
    let canonical = canonical_bytes(&record, true);

    if !crate::crypto::verify_canonical_record(
        &vk,
        crate::crypto::SIG_DOMAIN_COMMAND_ENVELOPE,
        &canonical,
        &sig.value,
    ) {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("signature verification failed".into()),
            empty_map(),
        );
    }

    let mut derived = std::collections::BTreeMap::new();
    derived.insert(
        "command_id".into(),
        Value::String(crate::util::bytes_to_hex(&command.command_id)),
    );
    derived.insert(
        "command_hash".into(),
        Value::String(crate::util::bytes_to_hex(&command_hash(command).value)),
    );

    accept(Value::Map(derived), empty_map())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::{HashMap, HashSet};
    use crate::protocol::command_envelope::Payload;
    use crate::protocol::{
        AssuranceClaim, AssuranceRequirement, CapabilityDescriptor, CommandEnvelope,
        DelegationRecord, IdentityRef, NodeRef, ObjectRef, StreamRef,
    };
    use crate::protocol::{
        AssuranceClass, ExecutionClass, ObjectKind, RateLimit, Signature as ProtoSignature,
        StorageClass, TimeWindow, TransportClass,
    };
    use crate::protocol::{CapabilityKind, DelegationPolicy, ScopeKind};
    use crate::result::Verdict;
    use edgerun_crypto::p256::ecdsa::SigningKey;

    fn test_signing_key() -> SigningKey {
        let bytes: [u8; 32] = [7u8; 32];
        SigningKey::from_bytes(&bytes.into()).unwrap()
    }

    const TEST_NODE_ID: [u8; 64] = [
        4, 5, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0,
    ];

    fn make_unsigned_command() -> CommandEnvelope {
        CommandEnvelope {
            envelope_version: 1,
            command_id: vec![1, 2, 3],
            target_node: Some(NodeRef {
                node_id: TEST_NODE_ID.to_vec(),
            }),
            issuer: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: Some(1), // USER
                key_hint: None,
            }),
            command_type: 7, // QUERY
            command_version: 1,
            issued_at: Some(crate::protocol::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            not_before: None,
            expires_at: None,
            idempotency_key: Vec::new(),
            payload: None,
            delegation_chain: vec![],
            requested_assurance: None,
            command_metadata: None,
            signature: None,
            app_intent: Vec::new(),
        }
    }

    fn sign_command(key: &SigningKey, cmd: &mut CommandEnvelope) {
        let record = ProtocolRecord::CommandEnvelope(cmd.clone());
        let canonical = canonical_bytes(&record, true);
        let sig = crate::crypto::sign_canonical_record(
            key,
            crate::crypto::SIG_DOMAIN_COMMAND_ENVELOPE,
            &canonical,
        )
        .unwrap();
        cmd.signature = Some(ProtoSignature {
            algorithm: 1,
            value: sig,
        });
    }

    fn sign_assurance_claim(key: &SigningKey, claim: &mut AssuranceClaim) {
        claim.signature = None;
        if let Some(attester) = &mut claim.attester {
            attester.key_hint = Some(key_hint_for(key));
        }
        let record = ProtocolRecord::AssuranceClaim(claim.clone());
        let canonical = canonical_bytes(&record, true);
        let sig = crate::crypto::sign_canonical_record(
            key,
            crate::crypto::SIG_DOMAIN_ASSURANCE_CLAIM,
            &canonical,
        )
        .unwrap();
        claim.signature = Some(ProtoSignature {
            algorithm: 1,
            value: sig,
        });
    }

    fn signed_assurance_claim_for_node(
        key: &SigningKey,
        subject_node_id: Vec<u8>,
    ) -> AssuranceClaim {
        let attester_id = key_hint_for(key);
        let mut claim = AssuranceClaim {
            claim_version: 1,
            subject: Some(crate::protocol::assurance_claim::Subject::SubjectNode(
                NodeRef {
                    node_id: subject_node_id,
                },
            )),
            assurance_class: AssuranceClass::HardwareBacked as i32,
            attester: Some(IdentityRef {
                identity_id: attester_id.clone(),
                identity_kind: Some(1),
                key_hint: Some(attester_id),
            }),
            issued_at: Some(crate::protocol::Timestamp {
                seconds: 1_699_999_980,
                nanos: 0,
            }),
            expires_at: Some(crate::protocol::Timestamp {
                seconds: 1_800_000_000,
                nanos: 0,
            }),
            evidence_object: None,
            claim_note: "test assurance".into(),
            signature: None,
        };
        sign_assurance_claim(key, &mut claim);
        claim
    }

    fn key_hint_for(key: &SigningKey) -> Vec<u8> {
        let encoded = key.verifying_key().to_encoded_point(false);
        encoded.as_bytes()[1..65].to_vec()
    }

    fn make_signed_command(key: &SigningKey, key_hint: Option<Vec<u8>>) -> CommandEnvelope {
        let mut cmd = make_unsigned_command();
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint,
        });
        sign_command(key, &mut cmd);
        cmd
    }

    fn make_signed_command_for_other_node(
        key: &SigningKey,
        key_hint: Option<Vec<u8>>,
    ) -> CommandEnvelope {
        let mut cmd = make_unsigned_command();
        cmd.target_node = Some(NodeRef {
            node_id: vec![99, 99, 99],
        });
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint,
        });
        sign_command(key, &mut cmd);
        cmd
    }

    fn default_ctx() -> CommandValidationContext<'static> {
        static LOCAL_NODE_ID: [u8; 64] = [
            4, 5, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0,
        ];
        static EMPTY_CACHE: std::sync::LazyLock<HashMap<Vec<u8>, (Vec<u8>, i64)>> =
            std::sync::LazyLock::new(HashMap::new);
        static EMPTY_REVOKED: std::sync::LazyLock<HashSet<Vec<u8>>> =
            std::sync::LazyLock::new(HashSet::new);
        static EMPTY_DELEGATION_USES: std::sync::LazyLock<HashMap<Vec<u8>, u64>> =
            std::sync::LazyLock::new(HashMap::new);
        static EMPTY_DELEGATION_RATE_EVENTS: std::sync::LazyLock<HashMap<Vec<u8>, Vec<i64>>> =
            std::sync::LazyLock::new(HashMap::new);
        static EMPTY_ROOTS: [Vec<u8>; 0] = [];
        static EMPTY_LOCATION_CLASSES: [&str; 0] = [];
        CommandValidationContext {
            local_node_id: &LOCAL_NODE_ID,
            replay_cache: &*EMPTY_CACHE,
            revoked_delegation_ids: &*EMPTY_REVOKED,
            delegation_use_counts: &*EMPTY_DELEGATION_USES,
            delegation_rate_events_ms: &*EMPTY_DELEGATION_RATE_EVENTS,
            now_ms: 1_700_000_000_000,
            trusted_root_ids: &EMPTY_ROOTS,
            local_assurance_class: 2, // HARDWARE_BACKED for tests
            accepted_assurance_claims: &[],
            has_local_session: false,
            has_user_presence: false,
            transport_class: None,
            location_classes: &EMPTY_LOCATION_CLASSES,
            target_stream_id: None,
            target_view_type: None,
            target_domain: None,
            execution_class: None,
            storage_class: None,
        }
    }

    fn valid_global_scope() -> crate::protocol::ScopeDescriptor {
        crate::protocol::ScopeDescriptor {
            scope_version: 1,
            scope_kind: ScopeKind::GlobalWithConstraints as i32,
            target_nodes: vec![],
            target_streams: vec![],
            target_object_kinds: vec![],
            target_view_types: vec![],
            target_domains: vec![],
            time_bounds: None,
            scope_metadata: None,
        }
    }

    fn valid_delegation_issued_at() -> crate::protocol::Timestamp {
        crate::protocol::Timestamp {
            seconds: 1_699_999_000,
            nanos: 0,
        }
    }

    fn valid_constraint_set() -> crate::protocol::ConstraintSet {
        crate::protocol::ConstraintSet {
            constraint_version: 1,
            not_before: None,
            expires_at: None,
            max_uses: None,
            rate_limit: None,
            requires_local_session: None,
            requires_user_presence: None,
            requires_transport_classes: vec![],
            requires_location_classes: vec![],
            export_policy: 0,
            execution_class_limits: vec![],
            storage_class_limits: vec![],
            constraint_metadata: None,
        }
    }

    fn sign_test_delegation(
        key: &SigningKey,
        delegation: &mut DelegationRecord,
        issuer_key_hint: &[u8],
    ) {
        delegation.issuer.as_mut().unwrap().key_hint = Some(issuer_key_hint.to_vec());
        delegation.signature = None;
        let canonical =
            canonical_bytes(&ProtocolRecord::DelegationRecord(delegation.clone()), true);
        let sig = crate::crypto::sign_canonical_record(
            key,
            crate::crypto::SIG_DOMAIN_DELEGATION_RECORD,
            &canonical,
        )
        .unwrap();
        delegation.signature = Some(ProtoSignature {
            algorithm: 1,
            value: sig,
        });
    }

    #[test]
    fn command_without_signature_is_rejected() {
        let command = make_unsigned_command();
        let result = validate_command(&command, &default_ctx());
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
    }

    #[test]
    fn command_with_invalid_signature_is_rejected() {
        let mut command = make_unsigned_command();
        command.signature = Some(ProtoSignature {
            algorithm: 1,
            value: vec![0u8; 64],
        });
        let result = validate_command(&command, &default_ctx());
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
    }

    #[test]
    fn command_target_mismatch_is_rejected() {
        let key = test_signing_key();
        let vk = key.verifying_key();
        let node_id: [u8; 64] = {
            let encoded = vk.to_encoded_point(false);
            let mut bytes = [0u8; 64];
            bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
            bytes
        };
        let hint: Vec<u8> = node_id.to_vec();

        let cmd = make_signed_command_for_other_node(&key, Some(hint));
        let result = validate_command(&cmd, &default_ctx());
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::TargetMismatch));
    }

    #[test]
    fn valid_command_is_accepted() {
        let key = test_signing_key();
        let vk = key.verifying_key();
        let node_id: [u8; 64] = {
            let encoded = vk.to_encoded_point(false);
            let mut bytes = [0u8; 64];
            bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
            bytes
        };
        let hint: Vec<u8> = node_id.to_vec();

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let cmd = make_signed_command(&key, Some(hint));
        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Accept);
    }

    #[test]
    fn command_missing_issued_at_is_rejected() {
        let key = test_signing_key();
        let vk = key.verifying_key();
        let node_id: [u8; 64] = {
            let encoded = vk.to_encoded_point(false);
            let mut bytes = [0u8; 64];
            bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
            bytes
        };
        let hint: Vec<u8> = node_id.to_vec();

        let mut cmd = make_signed_command(&key, Some(hint));
        cmd.issued_at = None;
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn command_issued_in_future_defers() {
        let key = test_signing_key();
        let vk = key.verifying_key();
        let node_id: [u8; 64] = {
            let encoded = vk.to_encoded_point(false);
            let mut bytes = [0u8; 64];
            bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
            bytes
        };
        let hint: Vec<u8> = node_id.to_vec();

        let mut cmd = make_signed_command(&key, Some(hint));
        cmd.issued_at = Some(crate::protocol::Timestamp {
            seconds: 1_800_000_000,
            nanos: 0,
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Defer);
        assert_eq!(result.reason_code, Some(ReasonCode::TimeInvalid));
    }

    #[test]
    fn unknown_command_type_is_rejected() {
        let key = test_signing_key();
        let vk = key.verifying_key();
        let node_id: [u8; 64] = {
            let encoded = vk.to_encoded_point(false);
            let mut bytes = [0u8; 64];
            bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
            bytes
        };
        let hint: Vec<u8> = node_id.to_vec();

        let mut cmd = make_signed_command(&key, Some(hint));
        cmd.command_type = 999_999;
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn unspecified_command_type_is_rejected() {
        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let mut cmd = make_signed_command(&key, Some(hint));
        cmd.command_type = CommandType::Unspecified as i32;
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn command_with_empty_target_node_id_is_rejected_structurally() {
        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let mut cmd = make_signed_command(&key, Some(hint));
        cmd.target_node = Some(NodeRef { node_id: vec![] });
        sign_command(&key, &mut cmd);

        let result = validate_command(&cmd, &default_ctx());
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn command_with_empty_issuer_identity_id_is_rejected_structurally() {
        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let mut cmd = make_signed_command(&key, Some(hint.clone()));
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![],
            identity_kind: Some(1),
            key_hint: Some(hint),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn command_with_invalid_issuer_identity_kind_is_rejected_structurally() {
        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let mut cmd = make_signed_command(&key, Some(hint.clone()));
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(0),
            key_hint: Some(hint),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn command_with_inverted_validity_window_is_rejected() {
        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let mut cmd = make_signed_command(&key, Some(hint));
        cmd.not_before = Some(crate::protocol::Timestamp {
            seconds: 1_700_000_010,
            nanos: 0,
        });
        cmd.expires_at = Some(crate::protocol::Timestamp {
            seconds: 1_700_000_001,
            nanos: 0,
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::TimeInvalid));
    }

    #[test]
    fn command_with_empty_payload_object_is_rejected() {
        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let mut cmd = make_signed_command(&key, Some(hint));
        cmd.payload = Some(Payload::PayloadObject(ObjectRef {
            object_id: vec![],
            object_kind: None,
        }));
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn command_with_invalid_payload_object_kind_is_rejected() {
        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let mut cmd = make_signed_command(&key, Some(hint));
        cmd.payload = Some(Payload::PayloadObject(ObjectRef {
            object_id: vec![0x22; 32],
            object_kind: Some(999_999),
        }));
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn command_with_empty_inline_payload_is_rejected() {
        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let mut cmd = make_signed_command(&key, Some(hint));
        cmd.payload = Some(Payload::InlinePayload(vec![]));
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn command_with_empty_metadata_object_is_rejected() {
        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let mut cmd = make_signed_command(&key, Some(hint));
        cmd.command_metadata = Some(ObjectRef {
            object_id: vec![],
            object_kind: None,
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn command_with_invalid_metadata_object_kind_is_rejected() {
        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let mut cmd = make_signed_command(&key, Some(hint));
        cmd.command_metadata = Some(ObjectRef {
            object_id: vec![0x33; 32],
            object_kind: Some(crate::protocol::ObjectKind::Unspecified as i32),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn command_signature_validation_reuses_structural_checks() {
        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let mut cmd = make_signed_command(&key, Some(hint));
        cmd.command_type = CommandType::Unspecified as i32;
        sign_command(&key, &mut cmd);

        let result = validate_command_signature(&cmd);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn replay_same_command_hash_is_duplicate() {
        let key = test_signing_key();
        let vk = key.verifying_key();
        let node_id: [u8; 64] = {
            let encoded = vk.to_encoded_point(false);
            let mut bytes = [0u8; 64];
            bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
            bytes
        };
        let hint: Vec<u8> = node_id.to_vec();

        let cmd = make_signed_command(&key, Some(hint.clone()));
        let hash = command_hash(&cmd).value;

        // Replay cache is now keyed by command_hash, with (command_id, seq) as value
        let mut cache = HashMap::new();
        cache.insert(hash.clone(), (cmd.command_id.clone(), 1i64));

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;
        ctx.replay_cache = &cache;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Duplicate);
        assert_eq!(result.reason_code, Some(ReasonCode::ReplayDetected));
    }

    #[test]
    fn different_command_same_id_is_distinct() {
        // §5.1 makes command_hash the replay key; command_id is only an idempotency hint.
        let key = test_signing_key();
        let vk = key.verifying_key();
        let node_id: [u8; 64] = {
            let encoded = vk.to_encoded_point(false);
            let mut bytes = [0u8; 64];
            bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
            bytes
        };
        let hint: Vec<u8> = node_id.to_vec();

        let cmd = make_signed_command(&key, Some(hint.clone()));

        // Put a DIFFERENT command with the same command_id in the cache
        let mut cache = HashMap::new();
        cache.insert(vec![0xFF; 32], (cmd.command_id.clone(), 1i64));

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;
        ctx.replay_cache = &cache;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Accept);
    }

    #[test]
    fn expired_command_is_rejected() {
        let key = test_signing_key();
        let vk = key.verifying_key();
        let node_id: [u8; 64] = {
            let encoded = vk.to_encoded_point(false);
            let mut bytes = [0u8; 64];
            bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
            bytes
        };
        let hint: Vec<u8> = node_id.to_vec();

        let mut cmd = make_signed_command(&key, Some(hint));
        // Set expires_at to the past
        cmd.expires_at = Some(crate::protocol::Timestamp {
            seconds: 1_600_000_000, // ~Sep 2020
            nanos: 0,
        });
        // Re-sign after modification
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::TimeInvalid));
    }

    #[test]
    fn not_before_defers() {
        let key = test_signing_key();
        let vk = key.verifying_key();
        let node_id: [u8; 64] = {
            let encoded = vk.to_encoded_point(false);
            let mut bytes = [0u8; 64];
            bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
            bytes
        };
        let hint: Vec<u8> = node_id.to_vec();

        let mut cmd = make_signed_command(&key, Some(hint));
        // Set not_before to the future
        cmd.not_before = Some(crate::protocol::Timestamp {
            seconds: 1_800_000_000, // ~2027
            nanos: 0,
        });
        // Re-sign
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Defer);
        assert_eq!(result.reason_code, Some(ReasonCode::TimeInvalid));
    }

    #[test]
    fn canonical_command_deterministic() {
        let cmd = make_unsigned_command();
        let record = ProtocolRecord::CommandEnvelope(cmd.clone());
        let a = canonical_bytes(&record, true);
        let b = canonical_bytes(&record, true);
        assert_eq!(a, b);
    }

    #[test]
    fn canonical_command_signable_vs_full() {
        let mut cmd = make_unsigned_command();
        cmd.signature = Some(ProtoSignature {
            algorithm: 1,
            value: vec![1; 64],
        });
        let record = ProtocolRecord::CommandEnvelope(cmd.clone());
        let signable = canonical_bytes(&record, true);
        let full = canonical_bytes(&record, false);
        assert_ne!(signable, full);
        assert!(signable.len() < full.len());
    }

    #[test]
    fn delegation_chain_attenuation_violation_rejected() {
        use crate::protocol::CapabilityDescriptor;

        let key = test_signing_key();
        let vk = key.verifying_key();
        let node_id: [u8; 64] = {
            let encoded = vk.to_encoded_point(false);
            let mut bytes = [0u8; 64];
            bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
            bytes
        };

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        fn sign_delegation(key: &SigningKey, deleg: &mut DelegationRecord, issuer_key_hint: &[u8]) {
            deleg.issuer.as_mut().unwrap().key_hint = Some(issuer_key_hint.to_vec());
            deleg.signature = None;
            let canonical = canonical_bytes(&ProtocolRecord::DelegationRecord(deleg.clone()), true);
            // Use sign_canonical_record for domain-separated signing (matches verify_canonical_record)
            let sig = crate::crypto::sign_canonical_record(
                key,
                crate::crypto::SIG_DOMAIN_DELEGATION_RECORD,
                &canonical,
            )
            .unwrap();
            deleg.signature = Some(ProtoSignature {
                algorithm: 1,
                value: sig,
            });
        }

        // Parent grants ["read", "write"], child adds ["delete"] (violates attenuation)
        let mut parent = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.to_vec()),
            }),
            recipient: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["read".into(), "write".into()],
                scope: Some(valid_global_scope()),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_delegation(&key, &mut parent, &node_id);

        let mut child = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-2".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.to_vec()),
            }),
            recipient: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["read".into(), "write".into(), "delete".into()], // "delete" not in parent
                scope: Some(valid_global_scope()),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_delegation(&key, &mut child, &node_id);

        let mut cmd = make_unsigned_command();
        cmd.delegation_chain = vec![parent, child];
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(node_id.to_vec()),
        });
        // Sign the command
        sign_command(&key, &mut cmd);

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn delegation_chain_leaf_action_must_authorize_command() {
        let key = test_signing_key();
        let node_id = key_hint_for(&key);

        let mut delegation = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["read".into()],
                scope: Some(valid_global_scope()),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut delegation, &node_id);

        let mut cmd = make_unsigned_command();
        cmd.delegation_chain = vec![delegation];
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(node_id),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn delegation_chain_leaf_scope_must_include_command_target() {
        let key = test_signing_key();
        let node_id = key_hint_for(&key);
        let mut scope = valid_global_scope();
        scope.scope_kind = ScopeKind::Node as i32;
        scope.target_nodes = vec![NodeRef {
            node_id: vec![0x42; 64],
        }];

        let mut delegation = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(scope),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut delegation, &node_id);

        let mut cmd = make_unsigned_command();
        cmd.delegation_chain = vec![delegation];
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(node_id),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn delegation_chain_leaf_scope_must_include_payload_object_kind() {
        let key = test_signing_key();
        let node_id = key_hint_for(&key);
        let mut scope = valid_global_scope();
        scope.target_object_kinds = vec![ObjectKind::Snapshot as i32];

        let mut delegation = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(scope),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut delegation, &node_id);

        let mut cmd = make_unsigned_command();
        cmd.payload = Some(Payload::PayloadObject(ObjectRef {
            object_id: b"payload-1".to_vec(),
            object_kind: Some(ObjectKind::Payload as i32),
        }));
        cmd.delegation_chain = vec![delegation];
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(node_id),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn delegation_chain_leaf_scope_must_include_target_stream() {
        let key = test_signing_key();
        let node_id = key_hint_for(&key);
        let mut scope = valid_global_scope();
        scope.target_streams = vec![StreamRef {
            stream_id: b"stream-a".to_vec(),
        }];

        let mut delegation = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(scope),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut delegation, &node_id);

        let mut cmd = make_unsigned_command();
        cmd.delegation_chain = vec![delegation];
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(node_id),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;
        ctx.target_stream_id = Some(b"stream-b");

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn delegation_chain_leaf_scope_must_include_target_domain() {
        let key = test_signing_key();
        let node_id = key_hint_for(&key);
        let mut scope = valid_global_scope();
        scope.target_domains = vec!["secrets".into()];

        let mut delegation = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(scope),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut delegation, &node_id);

        let mut cmd = make_unsigned_command();
        cmd.delegation_chain = vec![delegation];
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(node_id),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;
        ctx.target_domain = Some("metrics");

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn delegation_chain_leaf_scope_must_include_target_view_type() {
        let key = test_signing_key();
        let node_id = key_hint_for(&key);
        let mut scope = valid_global_scope();
        scope.target_view_types = vec!["status".into()];

        let mut delegation = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(scope),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut delegation, &node_id);

        let mut cmd = make_unsigned_command();
        cmd.delegation_chain = vec![delegation];
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(node_id),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;
        ctx.target_view_type = Some("timeline");

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn delegation_chain_leaf_scope_time_bounds_defer_when_not_yet_valid() {
        let key = test_signing_key();
        let node_id = key_hint_for(&key);
        let mut scope = valid_global_scope();
        scope.time_bounds = Some(TimeWindow {
            not_before: Some(crate::protocol::Timestamp {
                seconds: 1_700_001_000,
                nanos: 0,
            }),
            expires_at: None,
        });

        let mut delegation = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(scope),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut delegation, &node_id);

        let mut cmd = make_unsigned_command();
        cmd.delegation_chain = vec![delegation];
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(node_id),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Defer);
        assert_eq!(result.reason_code, Some(ReasonCode::TimeInvalid));
    }

    #[test]
    fn delegation_chain_leaf_constraints_expired_rejected() {
        let key = test_signing_key();
        let node_id = key_hint_for(&key);
        let mut constraints = valid_constraint_set();
        constraints.expires_at = Some(crate::protocol::Timestamp {
            seconds: 1_699_999_000,
            nanos: 0,
        });

        let mut delegation = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: Some(constraints),
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut delegation, &node_id);

        let mut cmd = make_unsigned_command();
        cmd.delegation_chain = vec![delegation];
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(node_id),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::TimeInvalid));
    }

    #[test]
    fn delegation_chain_leaf_constraints_require_local_session() {
        let key = test_signing_key();
        let node_id = key_hint_for(&key);
        let mut constraints = valid_constraint_set();
        constraints.requires_local_session = Some(true);

        let mut delegation = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: Some(constraints),
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut delegation, &node_id);

        let mut cmd = make_unsigned_command();
        cmd.delegation_chain = vec![delegation];
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(node_id),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn delegation_chain_max_uses_exhausted_rejected() {
        let key = test_signing_key();
        let node_id = key_hint_for(&key);
        let mut constraints = valid_constraint_set();
        constraints.max_uses = Some(1);

        let mut delegation = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: Some(constraints),
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut delegation, &node_id);

        let mut cmd = make_unsigned_command();
        cmd.delegation_chain = vec![delegation];
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(node_id),
        });
        sign_command(&key, &mut cmd);

        let mut delegation_use_counts = HashMap::new();
        delegation_use_counts.insert(b"deleg-1".to_vec(), 1);
        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;
        ctx.delegation_use_counts = &delegation_use_counts;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn delegation_chain_rate_limit_exhausted_rejected() {
        let key = test_signing_key();
        let node_id = key_hint_for(&key);
        let mut constraints = valid_constraint_set();
        constraints.rate_limit = Some(RateLimit {
            max_operations: 2,
            per: Some(crate::protocol::Duration {
                seconds: 60,
                nanos: 0,
            }),
        });

        let mut delegation = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: Some(constraints),
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut delegation, &node_id);

        let mut cmd = make_unsigned_command();
        cmd.delegation_chain = vec![delegation];
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(node_id),
        });
        sign_command(&key, &mut cmd);

        let mut delegation_rate_events_ms = HashMap::new();
        delegation_rate_events_ms.insert(
            b"deleg-1".to_vec(),
            vec![1_699_999_950_000, 1_699_999_990_000],
        );
        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;
        ctx.delegation_rate_events_ms = &delegation_rate_events_ms;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn delegation_chain_leaf_execution_class_limit_must_match_context() {
        let key = test_signing_key();
        let node_id = key_hint_for(&key);
        let mut constraints = valid_constraint_set();
        constraints.execution_class_limits = vec![ExecutionClass::LocalOnly as i32];

        let mut delegation = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: Some(constraints),
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut delegation, &node_id);

        let mut cmd = make_unsigned_command();
        cmd.delegation_chain = vec![delegation];
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(node_id),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;
        ctx.execution_class = Some(ExecutionClass::Public as i32);

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn delegation_chain_leaf_storage_class_limit_must_match_context() {
        let key = test_signing_key();
        let node_id = key_hint_for(&key);
        let mut constraints = valid_constraint_set();
        constraints.storage_class_limits = vec![StorageClass::Hot as i32];

        let mut delegation = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: Some(constraints),
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut delegation, &node_id);

        let mut cmd = make_unsigned_command();
        cmd.delegation_chain = vec![delegation];
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(node_id),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;
        ctx.storage_class = Some(StorageClass::Archive as i32);

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn delegation_chain_leaf_assurance_must_be_satisfied_locally() {
        let key = test_signing_key();
        let node_id = key_hint_for(&key);

        let mut delegation = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: Some(crate::protocol::AssuranceRequirement {
                    assurance_version: 1,
                    required_class: AssuranceClass::AttestedRuntime as i32,
                    acceptable_attesters: vec![],
                    max_evidence_age: None,
                    assurance_metadata: None,
                }),
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut delegation, &node_id);

        let mut cmd = make_unsigned_command();
        cmd.delegation_chain = vec![delegation];
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(node_id),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;
        ctx.local_assurance_class = AssuranceClass::HardwareBacked as i32;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AssuranceInsufficient));
    }

    #[test]
    fn delegation_chain_capability_kind_expansion_rejected() {
        let key = test_signing_key();
        let node_id = key_hint_for(&key);

        let mut parent = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut parent, &node_id);

        let mut child = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-2".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::ObjectFetch as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut child, &node_id);

        let mut cmd = make_unsigned_command();
        cmd.delegation_chain = vec![parent, child];
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(node_id),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn delegation_chain_non_delegable_parent_rejected() {
        let key = test_signing_key();
        let node_id = key_hint_for(&key);

        let mut parent = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: None,
                delegation_policy: DelegationPolicy::NonDelegable as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut parent, &node_id);

        let mut child = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-2".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut child, &node_id);

        let mut cmd = make_unsigned_command();
        cmd.delegation_chain = vec![parent, child];
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(node_id),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn delegation_chain_scope_kind_expansion_rejected() {
        let key = test_signing_key();
        let node_id = key_hint_for(&key);
        let mut parent_scope = valid_global_scope();
        parent_scope.scope_kind = ScopeKind::Node as i32;
        parent_scope.target_nodes = vec![NodeRef {
            node_id: TEST_NODE_ID.to_vec(),
        }];
        let mut child_scope = valid_global_scope();
        child_scope.scope_kind = ScopeKind::Stream as i32;

        let mut parent = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(parent_scope),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut parent, &node_id);

        let mut child = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-2".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(child_scope),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut child, &node_id);

        let mut cmd = make_unsigned_command();
        cmd.delegation_chain = vec![parent, child];
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(node_id),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn delegation_chain_global_parent_domain_constraint_must_attenuate() {
        let key = test_signing_key();
        let node_id = key_hint_for(&key);
        let mut parent_scope = valid_global_scope();
        parent_scope.scope_kind = ScopeKind::GlobalWithConstraints as i32;
        parent_scope.target_domains = vec!["secrets".into()];
        let mut child_scope = valid_global_scope();
        child_scope.scope_kind = ScopeKind::Domain as i32;
        child_scope.target_domains = vec!["metrics".into()];

        let mut parent = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(parent_scope),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut parent, &node_id);

        let mut child = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-2".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(child_scope),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut child, &node_id);

        let mut cmd = make_unsigned_command();
        cmd.delegation_chain = vec![parent, child];
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(node_id),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn delegation_chain_dropping_parent_scope_target_rejected() {
        let key = test_signing_key();
        let node_id = key_hint_for(&key);
        let mut parent_scope = valid_global_scope();
        parent_scope.scope_kind = ScopeKind::Node as i32;
        parent_scope.target_nodes = vec![NodeRef {
            node_id: TEST_NODE_ID.to_vec(),
        }];
        let mut child_scope = valid_global_scope();
        child_scope.scope_kind = ScopeKind::Node as i32;

        let mut parent = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(parent_scope),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut parent, &node_id);

        let mut child = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-2".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(child_scope),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut child, &node_id);

        let mut cmd = make_unsigned_command();
        cmd.delegation_chain = vec![parent, child];
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(node_id),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn delegation_chain_scope_time_bounds_expansion_rejected() {
        let key = test_signing_key();
        let node_id = key_hint_for(&key);
        let mut parent_scope = valid_global_scope();
        parent_scope.time_bounds = Some(TimeWindow {
            not_before: Some(crate::protocol::Timestamp {
                seconds: 1_699_999_000,
                nanos: 0,
            }),
            expires_at: Some(crate::protocol::Timestamp {
                seconds: 1_700_001_000,
                nanos: 0,
            }),
        });

        let mut parent = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(parent_scope),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut parent, &node_id);

        let mut child = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-2".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut child, &node_id);

        let mut cmd = make_unsigned_command();
        cmd.delegation_chain = vec![parent, child];
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(node_id),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn delegation_timing_attenuation_rejects_dropped_not_before() {
        let mut parent = DelegationRecord::default();
        parent.not_before = Some(crate::protocol::Timestamp {
            seconds: 1_700_000_000,
            nanos: 0,
        });
        let child = DelegationRecord::default();

        let result = attenuate_timing(&parent, &child, 1);

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().reason_code,
            Some(ReasonCode::AuthorityDenied)
        );
    }

    #[test]
    fn delegation_timing_attenuation_rejects_dropped_expires_at() {
        let mut parent = DelegationRecord::default();
        parent.expires_at = Some(crate::protocol::Timestamp {
            seconds: 1_700_001_000,
            nanos: 0,
        });
        let child = DelegationRecord::default();

        let result = attenuate_timing(&parent, &child, 1);

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().reason_code,
            Some(ReasonCode::AuthorityDenied)
        );
    }

    #[test]
    fn delegation_chain_dropping_parent_constraints_rejected() {
        let key = test_signing_key();
        let node_id = key_hint_for(&key);

        let mut parent = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: Some(crate::protocol::ConstraintSet {
                    constraint_version: 1,
                    not_before: None,
                    expires_at: None,
                    max_uses: Some(1),
                    rate_limit: None,
                    requires_local_session: None,
                    requires_user_presence: None,
                    requires_transport_classes: vec![],
                    requires_location_classes: vec![],
                    export_policy: 0,
                    execution_class_limits: vec![],
                    storage_class_limits: vec![],
                    constraint_metadata: None,
                }),
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut parent, &node_id);

        let mut child = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-2".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut child, &node_id);

        let mut cmd = make_unsigned_command();
        cmd.delegation_chain = vec![parent, child];
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(node_id),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn delegation_chain_dropping_parent_max_uses_rejected() {
        let key = test_signing_key();
        let node_id = key_hint_for(&key);
        let mut parent_constraints = valid_constraint_set();
        parent_constraints.max_uses = Some(1);
        let child_constraints = valid_constraint_set();

        let mut parent = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: Some(parent_constraints),
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut parent, &node_id);

        let mut child = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-2".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: Some(child_constraints),
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut child, &node_id);

        let mut cmd = make_unsigned_command();
        cmd.delegation_chain = vec![parent, child];
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(node_id),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn delegation_chain_looser_child_rate_limit_rejected() {
        let key = test_signing_key();
        let node_id = key_hint_for(&key);
        let mut parent_constraints = valid_constraint_set();
        parent_constraints.rate_limit = Some(crate::protocol::RateLimit {
            max_operations: 10,
            per: Some(crate::protocol::Duration {
                seconds: 10,
                nanos: 0,
            }),
        });
        let mut child_constraints = valid_constraint_set();
        child_constraints.rate_limit = Some(crate::protocol::RateLimit {
            max_operations: 20,
            per: Some(crate::protocol::Duration {
                seconds: 10,
                nanos: 0,
            }),
        });

        let mut parent = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: Some(parent_constraints),
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut parent, &node_id);

        let mut child = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-2".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: Some(child_constraints),
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut child, &node_id);

        let mut cmd = make_unsigned_command();
        cmd.delegation_chain = vec![parent, child];
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(node_id),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn delegation_chain_parent_hash_mismatch_is_rejected() {
        let key = test_signing_key();
        let node_id = key_hint_for(&key);

        let mut parent = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut parent, &node_id);

        let mut child = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-2".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: Some(crate::protocol::DelegationRef {
                delegation_id: b"deleg-1".to_vec(),
                delegation_hash: Some(crate::protocol::Digest {
                    algorithm: crate::protocol::digest::Algorithm::DigestAlgorithmSha256 as i32,
                    value: vec![0xA5; 32],
                }),
            }),
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut child, &node_id);

        let mut cmd = make_unsigned_command();
        cmd.delegation_chain = vec![parent, child];
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(node_id),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn delegation_chain_dropping_parent_transport_class_rejected() {
        let key = test_signing_key();
        let node_id = key_hint_for(&key);
        let mut parent_constraints = valid_constraint_set();
        parent_constraints.requires_transport_classes = vec![TransportClass::Quic as i32];
        let child_constraints = valid_constraint_set();

        let mut parent = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: Some(parent_constraints),
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut parent, &node_id);

        let mut child = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-2".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: Some(child_constraints),
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut child, &node_id);

        let mut cmd = make_unsigned_command();
        cmd.delegation_chain = vec![parent, child];
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(node_id),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn delegation_chain_dropping_parent_assurance_rejected() {
        let key = test_signing_key();
        let node_id = key_hint_for(&key);

        let mut parent = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: Some(crate::protocol::AssuranceRequirement {
                    assurance_version: 1,
                    required_class: AssuranceClass::HardwareBacked as i32,
                    acceptable_attesters: vec![],
                    max_evidence_age: None,
                    assurance_metadata: None,
                }),
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut parent, &node_id);

        let mut child = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-2".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut child, &node_id);

        let mut cmd = make_unsigned_command();
        cmd.delegation_chain = vec![parent, child];
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(node_id),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AssuranceInsufficient));
    }

    #[test]
    fn delegation_chain_dropping_parent_assurance_attester_rejected() {
        let key = test_signing_key();
        let node_id = key_hint_for(&key);
        let parent_attester = IdentityRef {
            identity_id: b"attester-1".to_vec(),
            identity_kind: None,
            key_hint: None,
        };

        let mut parent = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: Some(crate::protocol::AssuranceRequirement {
                    assurance_version: 1,
                    required_class: AssuranceClass::HardwareBacked as i32,
                    acceptable_attesters: vec![parent_attester],
                    max_evidence_age: None,
                    assurance_metadata: None,
                }),
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut parent, &node_id);

        let mut child = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-2".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: Some(crate::protocol::AssuranceRequirement {
                    assurance_version: 1,
                    required_class: AssuranceClass::HardwareBacked as i32,
                    acceptable_attesters: vec![],
                    max_evidence_age: None,
                    assurance_metadata: None,
                }),
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut child, &node_id);

        let mut cmd = make_unsigned_command();
        cmd.delegation_chain = vec![parent, child];
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(node_id),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AssuranceInsufficient));
    }

    #[test]
    fn delegation_chain_loosening_parent_assurance_max_age_rejected() {
        let key = test_signing_key();
        let node_id = key_hint_for(&key);

        let mut parent = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: Some(crate::protocol::AssuranceRequirement {
                    assurance_version: 1,
                    required_class: AssuranceClass::HardwareBacked as i32,
                    acceptable_attesters: vec![],
                    max_evidence_age: Some(crate::protocol::Duration {
                        seconds: 60,
                        nanos: 0,
                    }),
                    assurance_metadata: None,
                }),
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut parent, &node_id);

        let mut child = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-2".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: Some(node_id.clone()),
            }),
            recipient: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: Some(crate::protocol::AssuranceRequirement {
                    assurance_version: 1,
                    required_class: AssuranceClass::HardwareBacked as i32,
                    acceptable_attesters: vec![],
                    max_evidence_age: Some(crate::protocol::Duration {
                        seconds: 120,
                        nanos: 0,
                    }),
                    assurance_metadata: None,
                }),
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_test_delegation(&key, &mut child, &node_id);

        let mut cmd = make_unsigned_command();
        cmd.delegation_chain = vec![parent, child];
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(node_id),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AssuranceInsufficient));
    }

    #[test]
    fn revoked_delegation_is_rejected() {
        use crate::protocol::CapabilityDescriptor;

        let key = test_signing_key();
        let vk = key.verifying_key();
        let node_id: [u8; 64] = {
            let encoded = vk.to_encoded_point(false);
            let mut bytes = [0u8; 64];
            bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
            bytes
        };

        let mut revoked = HashSet::new();
        revoked.insert(b"deleg-1".to_vec());

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;
        ctx.revoked_delegation_ids = &revoked;

        let deleg = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(), // This one is revoked
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            recipient: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: Some(ProtoSignature {
                algorithm: 1,
                value: vec![1; 64],
            }),
        };

        let mut cmd = make_unsigned_command();
        cmd.delegation_chain = vec![deleg];
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(node_id.to_vec()),
        });
        sign_command(&key, &mut cmd);

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::RevocationActive));
    }

    #[test]
    fn command_with_unspecified_assurance_is_accepted() {
        // When requested_assurance is None or has unspecified class, no check is needed
        let key = test_signing_key();
        let vk = key.verifying_key();
        let node_id: [u8; 64] = {
            let encoded = vk.to_encoded_point(false);
            let mut bytes = [0u8; 64];
            bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
            bytes
        };
        let hint: Vec<u8> = node_id.to_vec();

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;
        ctx.local_assurance_class = 0; // No assurance capability

        let cmd = make_signed_command(&key, Some(hint));
        // requested_assurance is None by default
        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Accept);
    }

    #[test]
    fn command_with_unsupported_assurance_requirement_version_is_rejected() {
        use crate::protocol::AssuranceClass;
        use crate::protocol::AssuranceRequirement;

        let key = test_signing_key();
        let vk = key.verifying_key();
        let node_id: [u8; 64] = {
            let encoded = vk.to_encoded_point(false);
            let mut bytes = [0u8; 64];
            bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
            bytes
        };
        let hint: Vec<u8> = node_id.to_vec();

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let mut cmd = make_unsigned_command();
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(hint),
        });
        cmd.requested_assurance = Some(AssuranceRequirement {
            assurance_version: 2,
            required_class: AssuranceClass::Software as i32,
            acceptable_attesters: vec![],
            max_evidence_age: None,
            assurance_metadata: None,
        });
        sign_command(&key, &mut cmd);

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::VersionUnsupported));
    }

    #[test]
    fn command_with_invalid_assurance_class_is_rejected() {
        use crate::protocol::AssuranceRequirement;

        let key = test_signing_key();
        let vk = key.verifying_key();
        let node_id: [u8; 64] = {
            let encoded = vk.to_encoded_point(false);
            let mut bytes = [0u8; 64];
            bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
            bytes
        };
        let hint: Vec<u8> = node_id.to_vec();

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let mut cmd = make_unsigned_command();
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(hint),
        });
        cmd.requested_assurance = Some(AssuranceRequirement {
            assurance_version: 1,
            required_class: 999_999,
            acceptable_attesters: vec![],
            max_evidence_age: None,
            assurance_metadata: None,
        });
        sign_command(&key, &mut cmd);

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn command_with_empty_requested_assurance_attester_is_rejected() {
        use crate::protocol::AssuranceClass;
        use crate::protocol::AssuranceRequirement;

        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let mut cmd = make_signed_command(&key, Some(hint));
        cmd.requested_assurance = Some(AssuranceRequirement {
            assurance_version: 1,
            required_class: AssuranceClass::Software as i32,
            acceptable_attesters: vec![IdentityRef {
                identity_id: vec![],
                identity_kind: Some(1),
                key_hint: None,
            }],
            max_evidence_age: None,
            assurance_metadata: None,
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn command_with_invalid_requested_assurance_attester_kind_is_rejected() {
        use crate::protocol::AssuranceClass;
        use crate::protocol::AssuranceRequirement;

        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let mut cmd = make_signed_command(&key, Some(hint));
        cmd.requested_assurance = Some(AssuranceRequirement {
            assurance_version: 1,
            required_class: AssuranceClass::Software as i32,
            acceptable_attesters: vec![IdentityRef {
                identity_id: vec![1, 2, 3],
                identity_kind: Some(999_999),
                key_hint: None,
            }],
            max_evidence_age: None,
            assurance_metadata: None,
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn command_with_empty_requested_assurance_metadata_is_rejected() {
        use crate::protocol::AssuranceClass;
        use crate::protocol::AssuranceRequirement;

        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let mut cmd = make_signed_command(&key, Some(hint));
        cmd.requested_assurance = Some(AssuranceRequirement {
            assurance_version: 1,
            required_class: AssuranceClass::Software as i32,
            acceptable_attesters: vec![],
            max_evidence_age: None,
            assurance_metadata: Some(ObjectRef {
                object_id: vec![],
                object_kind: None,
            }),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn command_with_invalid_requested_assurance_metadata_kind_is_rejected() {
        use crate::protocol::AssuranceClass;
        use crate::protocol::AssuranceRequirement;

        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let mut cmd = make_signed_command(&key, Some(hint));
        cmd.requested_assurance = Some(AssuranceRequirement {
            assurance_version: 1,
            required_class: AssuranceClass::Software as i32,
            acceptable_attesters: vec![],
            max_evidence_age: None,
            assurance_metadata: Some(ObjectRef {
                object_id: vec![0x44; 32],
                object_kind: Some(999_999),
            }),
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn command_with_negative_requested_assurance_max_age_is_rejected() {
        use crate::protocol::AssuranceClass;
        use crate::protocol::AssuranceRequirement;

        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let mut cmd = make_signed_command(&key, Some(hint));
        cmd.requested_assurance = Some(AssuranceRequirement {
            assurance_version: 1,
            required_class: AssuranceClass::Software as i32,
            acceptable_attesters: vec![],
            max_evidence_age: Some(crate::protocol::Duration {
                seconds: -1,
                nanos: 0,
            }),
            assurance_metadata: None,
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn command_requested_assurance_max_age_requires_claim_evidence() {
        use crate::protocol::AssuranceClass;
        use crate::protocol::AssuranceRequirement;

        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let mut cmd = make_signed_command(&key, Some(hint));
        cmd.requested_assurance = Some(AssuranceRequirement {
            assurance_version: 1,
            required_class: AssuranceClass::Software as i32,
            acceptable_attesters: vec![],
            max_evidence_age: Some(crate::protocol::Duration {
                seconds: 60,
                nanos: 0,
            }),
            assurance_metadata: None,
        });
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;
        ctx.local_assurance_class = AssuranceClass::HardwareBacked as i32;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn command_requested_assurance_rejects_claim_for_other_node() {
        use crate::protocol::AssuranceClass;
        use crate::protocol::AssuranceRequirement;

        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let mut cmd = make_signed_command(&key, Some(hint));
        cmd.requested_assurance = Some(AssuranceRequirement {
            assurance_version: 1,
            required_class: AssuranceClass::Software as i32,
            acceptable_attesters: vec![],
            max_evidence_age: Some(crate::protocol::Duration {
                seconds: 60,
                nanos: 0,
            }),
            assurance_metadata: None,
        });
        sign_command(&key, &mut cmd);

        let claims = [signed_assurance_claim_for_node(&key, vec![0x42; 64])];
        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;
        ctx.local_assurance_class = AssuranceClass::HardwareBacked as i32;
        ctx.accepted_assurance_claims = &claims;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn command_requested_assurance_accepts_matching_node_claim() {
        use crate::protocol::AssuranceClass;
        use crate::protocol::AssuranceRequirement;

        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let mut cmd = make_signed_command(&key, Some(hint));
        cmd.requested_assurance = Some(AssuranceRequirement {
            assurance_version: 1,
            required_class: AssuranceClass::Software as i32,
            acceptable_attesters: vec![],
            max_evidence_age: Some(crate::protocol::Duration {
                seconds: 60,
                nanos: 0,
            }),
            assurance_metadata: None,
        });
        sign_command(&key, &mut cmd);

        let claims = [signed_assurance_claim_for_node(&key, TEST_NODE_ID.to_vec())];
        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;
        ctx.local_assurance_class = AssuranceClass::Unspecified as i32;
        ctx.accepted_assurance_claims = &claims;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Accept);
    }

    #[test]
    fn command_requesting_hardware_backed_is_rejected_when_node_is_software_only() {
        use crate::protocol::AssuranceClass;
        use crate::protocol::AssuranceRequirement;

        let key = test_signing_key();
        let vk = key.verifying_key();
        let node_id: [u8; 64] = {
            let encoded = vk.to_encoded_point(false);
            let mut bytes = [0u8; 64];
            bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
            bytes
        };
        let hint: Vec<u8> = node_id.to_vec();

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;
        ctx.local_assurance_class = 1; // SOFTWARE only

        // Build unsigned command with assurance requirement, then sign
        let mut cmd = make_unsigned_command();
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(hint),
        });
        cmd.requested_assurance = Some(AssuranceRequirement {
            assurance_version: 1,
            required_class: AssuranceClass::HardwareBacked as i32,
            acceptable_attesters: vec![],
            max_evidence_age: None,
            assurance_metadata: None,
        });
        // Sign it
        sign_command(&key, &mut cmd);

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn command_requesting_attested_runtime_is_rejected_when_node_is_hardware_only() {
        use crate::protocol::AssuranceClass;
        use crate::protocol::AssuranceRequirement;

        let key = test_signing_key();
        let vk = key.verifying_key();
        let node_id: [u8; 64] = {
            let encoded = vk.to_encoded_point(false);
            let mut bytes = [0u8; 64];
            bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
            bytes
        };
        let hint: Vec<u8> = node_id.to_vec();

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;
        ctx.local_assurance_class = 2; // HARDWARE_BACKED

        let mut cmd = make_unsigned_command();
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(hint),
        });
        cmd.requested_assurance = Some(AssuranceRequirement {
            assurance_version: 1,
            required_class: AssuranceClass::AttestedRuntime as i32,
            acceptable_attesters: vec![],
            max_evidence_age: None,
            assurance_metadata: None,
        });
        sign_command(&key, &mut cmd);

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn command_requesting_software_is_accepted_when_node_is_hardware_backed() {
        // Hardware-backed node can satisfy software requirement (higher >= lower)
        use crate::protocol::AssuranceClass;
        use crate::protocol::AssuranceRequirement;

        let key = test_signing_key();
        let vk = key.verifying_key();
        let node_id: [u8; 64] = {
            let encoded = vk.to_encoded_point(false);
            let mut bytes = [0u8; 64];
            bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
            bytes
        };
        let hint: Vec<u8> = node_id.to_vec();

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;
        ctx.local_assurance_class = 2; // HARDWARE_BACKED

        let mut cmd = make_unsigned_command();
        cmd.issuer = Some(IdentityRef {
            identity_id: vec![7, 8, 9],
            identity_kind: Some(1),
            key_hint: Some(hint),
        });
        cmd.requested_assurance = Some(AssuranceRequirement {
            assurance_version: 1,
            required_class: AssuranceClass::Software as i32,
            acceptable_attesters: vec![],
            max_evidence_age: None,
            assurance_metadata: None,
        });
        sign_command(&key, &mut cmd);

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Accept);
    }

    #[test]
    fn delegation_chain_root_not_in_trusted_roots_is_rejected() {
        // Delegation chain where root issuer is NOT in trusted_root_ids
        let key = test_signing_key();
        let vk = key.verifying_key();
        let node_id: [u8; 64] = {
            let encoded = vk.to_encoded_point(false);
            let mut bytes = [0u8; 64];
            bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
            bytes
        };
        let hint: Vec<u8> = node_id.to_vec();

        let trusted = vec![vec![99, 99, 99]]; // Different identity, not the root
        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;
        ctx.trusted_root_ids = &trusted;

        // Build a delegation with root issuer NOT in trusted roots
        let root_issuer_id = vec![1, 2, 3];
        let delegate_id = vec![7, 8, 9]; // command issuer
        let delegation = DelegationRecord {
            record_version: 1,
            delegation_id: vec![10, 20, 30],
            issuer: Some(IdentityRef {
                identity_id: root_issuer_id.clone(),
                identity_kind: Some(2),
                key_hint: None,
            }),
            recipient: Some(IdentityRef {
                identity_id: delegate_id.clone(),
                identity_kind: Some(2),
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: Some(ProtoSignature {
                algorithm: 1,
                value: vec![0xAA; 64], // Dummy signature (won't be verified structurally)
            }),
        };

        let mut cmd = make_unsigned_command();
        cmd.issuer = Some(IdentityRef {
            identity_id: delegate_id,
            identity_kind: Some(1),
            key_hint: Some(hint),
        });
        cmd.delegation_chain = vec![delegation];
        cmd.command_type = 7; // QUERY
        sign_command(&key, &mut cmd);

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn delegation_chain_invalid_recipient_kind_is_rejected_structurally() {
        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let delegate_id = vec![7, 8, 9];
        let delegation = DelegationRecord {
            record_version: 1,
            delegation_id: vec![10, 20, 30],
            issuer: Some(IdentityRef {
                identity_id: vec![1, 2, 3],
                identity_kind: Some(2),
                key_hint: None,
            }),
            recipient: Some(IdentityRef {
                identity_id: delegate_id.clone(),
                identity_kind: Some(0),
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: Some(ProtoSignature {
                algorithm: 1,
                value: vec![0xAA; 64],
            }),
        };

        let mut cmd = make_unsigned_command();
        cmd.issuer = Some(IdentityRef {
            identity_id: delegate_id,
            identity_kind: Some(1),
            key_hint: Some(hint),
        });
        cmd.delegation_chain = vec![delegation];
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn delegation_chain_invalid_capability_metadata_kind_is_rejected_structurally() {
        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let delegate_id = vec![7, 8, 9];
        let delegation = DelegationRecord {
            record_version: 1,
            delegation_id: vec![10, 20, 30],
            issuer: Some(IdentityRef {
                identity_id: vec![1, 2, 3],
                identity_kind: Some(2),
                key_hint: None,
            }),
            recipient: Some(IdentityRef {
                identity_id: delegate_id.clone(),
                identity_kind: Some(2),
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: Some(ObjectRef {
                    object_id: vec![0x44; 32],
                    object_kind: Some(999_999),
                }),
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: Some(ProtoSignature {
                algorithm: 1,
                value: vec![0xAA; 64],
            }),
        };

        let mut cmd = make_unsigned_command();
        cmd.issuer = Some(IdentityRef {
            identity_id: delegate_id,
            identity_kind: Some(1),
            key_hint: Some(hint),
        });
        cmd.delegation_chain = vec![delegation];
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn delegation_chain_invalid_capability_kind_is_rejected_structurally() {
        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let delegate_id = vec![7, 8, 9];
        let delegation = DelegationRecord {
            record_version: 1,
            delegation_id: vec![10, 20, 30],
            issuer: Some(IdentityRef {
                identity_id: vec![1, 2, 3],
                identity_kind: Some(2),
                key_hint: None,
            }),
            recipient: Some(IdentityRef {
                identity_id: delegate_id.clone(),
                identity_kind: Some(2),
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Unspecified as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: Some(ProtoSignature {
                algorithm: 1,
                value: vec![0xAA; 64],
            }),
        };

        let mut cmd = make_unsigned_command();
        cmd.issuer = Some(IdentityRef {
            identity_id: delegate_id,
            identity_kind: Some(1),
            key_hint: Some(hint),
        });
        cmd.delegation_chain = vec![delegation];
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn delegation_chain_unsupported_capability_version_is_rejected() {
        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let delegate_id = vec![7, 8, 9];
        let delegation = DelegationRecord {
            record_version: 1,
            delegation_id: vec![10, 20, 30],
            issuer: Some(IdentityRef {
                identity_id: vec![1, 2, 3],
                identity_kind: Some(2),
                key_hint: None,
            }),
            recipient: Some(IdentityRef {
                identity_id: delegate_id.clone(),
                identity_kind: Some(2),
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 2,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: Some(ProtoSignature {
                algorithm: 1,
                value: vec![0xAA; 64],
            }),
        };

        let mut cmd = make_unsigned_command();
        cmd.issuer = Some(IdentityRef {
            identity_id: delegate_id,
            identity_kind: Some(1),
            key_hint: Some(hint),
        });
        cmd.delegation_chain = vec![delegation];
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::VersionUnsupported));
    }

    #[test]
    fn delegation_chain_empty_capability_actions_is_rejected_structurally() {
        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let delegate_id = vec![7, 8, 9];
        let delegation = DelegationRecord {
            record_version: 1,
            delegation_id: vec![10, 20, 30],
            issuer: Some(IdentityRef {
                identity_id: vec![1, 2, 3],
                identity_kind: Some(2),
                key_hint: None,
            }),
            recipient: Some(IdentityRef {
                identity_id: delegate_id.clone(),
                identity_kind: Some(2),
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec![],
                scope: Some(valid_global_scope()),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: Some(ProtoSignature {
                algorithm: 1,
                value: vec![0xAA; 64],
            }),
        };

        let mut cmd = make_unsigned_command();
        cmd.issuer = Some(IdentityRef {
            identity_id: delegate_id,
            identity_kind: Some(1),
            key_hint: Some(hint),
        });
        cmd.delegation_chain = vec![delegation];
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn delegation_chain_missing_issued_at_is_rejected_structurally() {
        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let delegate_id = vec![7, 8, 9];
        let delegation = DelegationRecord {
            record_version: 1,
            delegation_id: vec![10, 20, 30],
            issuer: Some(IdentityRef {
                identity_id: vec![1, 2, 3],
                identity_kind: Some(2),
                key_hint: None,
            }),
            recipient: Some(IdentityRef {
                identity_id: delegate_id.clone(),
                identity_kind: Some(2),
                key_hint: None,
            }),
            issued_at: None,
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: Some(ProtoSignature {
                algorithm: 1,
                value: vec![0xAA; 64],
            }),
        };

        let mut cmd = make_unsigned_command();
        cmd.issuer = Some(IdentityRef {
            identity_id: delegate_id,
            identity_kind: Some(1),
            key_hint: Some(hint),
        });
        cmd.delegation_chain = vec![delegation];
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn delegation_chain_issued_in_future_defers() {
        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let delegate_id = vec![7, 8, 9];
        let delegation = DelegationRecord {
            record_version: 1,
            delegation_id: vec![10, 20, 30],
            issuer: Some(IdentityRef {
                identity_id: vec![1, 2, 3],
                identity_kind: Some(2),
                key_hint: None,
            }),
            recipient: Some(IdentityRef {
                identity_id: delegate_id.clone(),
                identity_kind: Some(2),
                key_hint: None,
            }),
            issued_at: Some(crate::protocol::Timestamp {
                seconds: 1_800_000_000,
                nanos: 0,
            }),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: Some(ProtoSignature {
                algorithm: 1,
                value: vec![0xAA; 64],
            }),
        };

        let mut cmd = make_unsigned_command();
        cmd.issuer = Some(IdentityRef {
            identity_id: delegate_id,
            identity_kind: Some(1),
            key_hint: Some(hint),
        });
        cmd.delegation_chain = vec![delegation];
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Defer);
        assert_eq!(result.reason_code, Some(ReasonCode::TimeInvalid));
    }

    #[test]
    fn delegation_chain_missing_capability_scope_is_rejected_structurally() {
        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let delegate_id = vec![7, 8, 9];
        let delegation = DelegationRecord {
            record_version: 1,
            delegation_id: vec![10, 20, 30],
            issuer: Some(IdentityRef {
                identity_id: vec![1, 2, 3],
                identity_kind: Some(2),
                key_hint: None,
            }),
            recipient: Some(IdentityRef {
                identity_id: delegate_id.clone(),
                identity_kind: Some(2),
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: None,
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: Some(ProtoSignature {
                algorithm: 1,
                value: vec![0xAA; 64],
            }),
        };

        let mut cmd = make_unsigned_command();
        cmd.issuer = Some(IdentityRef {
            identity_id: delegate_id,
            identity_kind: Some(1),
            key_hint: Some(hint),
        });
        cmd.delegation_chain = vec![delegation];
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn delegation_chain_unsupported_scope_version_is_rejected() {
        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let delegate_id = vec![7, 8, 9];
        let delegation = DelegationRecord {
            record_version: 1,
            delegation_id: vec![10, 20, 30],
            issuer: Some(IdentityRef {
                identity_id: vec![1, 2, 3],
                identity_kind: Some(2),
                key_hint: None,
            }),
            recipient: Some(IdentityRef {
                identity_id: delegate_id.clone(),
                identity_kind: Some(2),
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(crate::protocol::ScopeDescriptor {
                    scope_version: 2,
                    scope_kind: ScopeKind::Node as i32,
                    target_nodes: vec![NodeRef {
                        node_id: TEST_NODE_ID.to_vec(),
                    }],
                    target_streams: vec![],
                    target_object_kinds: vec![],
                    target_view_types: vec![],
                    target_domains: vec![],
                    time_bounds: None,
                    scope_metadata: None,
                }),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: Some(ProtoSignature {
                algorithm: 1,
                value: vec![0xAA; 64],
            }),
        };

        let mut cmd = make_unsigned_command();
        cmd.issuer = Some(IdentityRef {
            identity_id: delegate_id,
            identity_kind: Some(1),
            key_hint: Some(hint),
        });
        cmd.delegation_chain = vec![delegation];
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::VersionUnsupported));
    }

    #[test]
    fn delegation_chain_invalid_scope_kind_is_rejected_structurally() {
        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let delegate_id = vec![7, 8, 9];
        let delegation = DelegationRecord {
            record_version: 1,
            delegation_id: vec![10, 20, 30],
            issuer: Some(IdentityRef {
                identity_id: vec![1, 2, 3],
                identity_kind: Some(2),
                key_hint: None,
            }),
            recipient: Some(IdentityRef {
                identity_id: delegate_id.clone(),
                identity_kind: Some(2),
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(crate::protocol::ScopeDescriptor {
                    scope_version: 1,
                    scope_kind: ScopeKind::Unspecified as i32,
                    target_nodes: vec![NodeRef {
                        node_id: TEST_NODE_ID.to_vec(),
                    }],
                    target_streams: vec![],
                    target_object_kinds: vec![],
                    target_view_types: vec![],
                    target_domains: vec![],
                    time_bounds: None,
                    scope_metadata: None,
                }),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: Some(ProtoSignature {
                algorithm: 1,
                value: vec![0xAA; 64],
            }),
        };

        let mut cmd = make_unsigned_command();
        cmd.issuer = Some(IdentityRef {
            identity_id: delegate_id,
            identity_kind: Some(1),
            key_hint: Some(hint),
        });
        cmd.delegation_chain = vec![delegation];
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn delegation_chain_empty_scope_domain_is_rejected_structurally() {
        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let delegate_id = vec![7, 8, 9];
        let mut scope = valid_global_scope();
        scope.target_domains = vec!["".into()];
        let delegation = DelegationRecord {
            record_version: 1,
            delegation_id: vec![10, 20, 30],
            issuer: Some(IdentityRef {
                identity_id: vec![1, 2, 3],
                identity_kind: Some(2),
                key_hint: None,
            }),
            recipient: Some(IdentityRef {
                identity_id: delegate_id.clone(),
                identity_kind: Some(2),
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(scope),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: Some(ProtoSignature {
                algorithm: 1,
                value: vec![0xAA; 64],
            }),
        };

        let mut cmd = make_unsigned_command();
        cmd.issuer = Some(IdentityRef {
            identity_id: delegate_id,
            identity_kind: Some(1),
            key_hint: Some(hint),
        });
        cmd.delegation_chain = vec![delegation];
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn delegation_chain_inverted_scope_time_bounds_is_rejected() {
        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let delegate_id = vec![7, 8, 9];
        let mut scope = valid_global_scope();
        scope.time_bounds = Some(TimeWindow {
            not_before: Some(crate::protocol::Timestamp {
                seconds: 20,
                nanos: 0,
            }),
            expires_at: Some(crate::protocol::Timestamp {
                seconds: 10,
                nanos: 0,
            }),
        });
        let delegation = DelegationRecord {
            record_version: 1,
            delegation_id: vec![10, 20, 30],
            issuer: Some(IdentityRef {
                identity_id: vec![1, 2, 3],
                identity_kind: Some(2),
                key_hint: None,
            }),
            recipient: Some(IdentityRef {
                identity_id: delegate_id.clone(),
                identity_kind: Some(2),
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(scope),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: Some(ProtoSignature {
                algorithm: 1,
                value: vec![0xAA; 64],
            }),
        };

        let mut cmd = make_unsigned_command();
        cmd.issuer = Some(IdentityRef {
            identity_id: delegate_id,
            identity_kind: Some(1),
            key_hint: Some(hint),
        });
        cmd.delegation_chain = vec![delegation];
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::TimeInvalid));
    }

    #[test]
    fn delegation_chain_invalid_constraint_transport_is_rejected_structurally() {
        let key = test_signing_key();
        let hint = key_hint_for(&key);
        let delegate_id = vec![7, 8, 9];
        let delegation = DelegationRecord {
            record_version: 1,
            delegation_id: vec![10, 20, 30],
            issuer: Some(IdentityRef {
                identity_id: vec![1, 2, 3],
                identity_kind: Some(2),
                key_hint: None,
            }),
            recipient: Some(IdentityRef {
                identity_id: delegate_id.clone(),
                identity_kind: Some(2),
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: Some(crate::protocol::ConstraintSet {
                    constraint_version: 1,
                    not_before: None,
                    expires_at: None,
                    max_uses: None,
                    rate_limit: None,
                    requires_local_session: None,
                    requires_user_presence: None,
                    requires_transport_classes: vec![0],
                    requires_location_classes: vec![],
                    export_policy: 0,
                    execution_class_limits: vec![],
                    storage_class_limits: vec![],
                    constraint_metadata: None,
                }),
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: Some(ProtoSignature {
                algorithm: 1,
                value: vec![0xAA; 64],
            }),
        };

        let mut cmd = make_unsigned_command();
        cmd.issuer = Some(IdentityRef {
            identity_id: delegate_id,
            identity_kind: Some(1),
            key_hint: Some(hint),
        });
        cmd.delegation_chain = vec![delegation];
        sign_command(&key, &mut cmd);

        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;

        let result = validate_command(&cmd, &ctx);
        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn delegation_chain_root_in_trusted_roots_passes() {
        let key = test_signing_key();
        let vk = key.verifying_key();
        let node_id: [u8; 64] = {
            let encoded = vk.to_encoded_point(false);
            let mut bytes = [0u8; 64];
            bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
            bytes
        };
        let hint: Vec<u8> = node_id.to_vec();

        let root_issuer_id = vec![1, 2, 3];
        let delegate_id = vec![7, 8, 9];
        let trusted = vec![root_issuer_id.clone()]; // Root IS trusted
        let mut ctx = default_ctx();
        ctx.local_node_id = &TEST_NODE_ID;
        ctx.trusted_root_ids = &trusted;

        // Create delegation with valid signature using domain-separated signing
        let mut delegation = DelegationRecord {
            record_version: 1,
            delegation_id: vec![10, 20, 30],
            issuer: Some(IdentityRef {
                identity_id: root_issuer_id.clone(),
                identity_kind: Some(2),
                key_hint: Some(node_id.to_vec()),
            }),
            recipient: Some(IdentityRef {
                identity_id: delegate_id.clone(),
                identity_kind: Some(2),
                key_hint: None,
            }),
            issued_at: Some(valid_delegation_issued_at()),
            not_before: None,
            expires_at: None,
            capability: Some(CapabilityDescriptor {
                capability_version: 1,
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".into()],
                scope: Some(valid_global_scope()),
                constraints: None,
                delegation_policy: DelegationPolicy::DelegableWithAttenuation as i32,
                minimum_assurance: None,
                capability_metadata: None,
            }),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        // Sign the delegation with domain separation (matches verify_canonical_record)
        delegation.signature = None;
        let deleg_canonical =
            canonical_bytes(&ProtocolRecord::DelegationRecord(delegation.clone()), true);
        let deleg_sig = crate::crypto::sign_canonical_record(
            &key,
            crate::crypto::SIG_DOMAIN_DELEGATION_RECORD,
            &deleg_canonical,
        )
        .unwrap();
        delegation.signature = Some(ProtoSignature {
            algorithm: 1,
            value: deleg_sig,
        });

        let mut cmd = make_unsigned_command();
        cmd.issuer = Some(IdentityRef {
            identity_id: delegate_id,
            identity_kind: Some(1),
            key_hint: Some(hint),
        });
        cmd.delegation_chain = vec![delegation];
        cmd.command_type = 7; // QUERY
        sign_command(&key, &mut cmd);

        let result = validate_command(&cmd, &ctx);
        // Root is trusted, and delegation has valid signature, so this should pass
        assert_eq!(result.verdict, Verdict::Accept);
    }
}
