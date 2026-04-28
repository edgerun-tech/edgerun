//! Protobuf-native protocol validators.
//!
//! This module provides direct validation functions over protobuf-generated types,
//! bridging the gap between the conformance suite (validators.rs, YAML/JSON-based)
//! and the runtime protocol path (command.rs, stream crate).
//!
//! Each validator takes and returns protobuf-native types with the unified
//! ACCEPT / REJECT / DEFER / DUPLICATE outcome model.

use crate::prelude::v1::*;

use crate::protocol::{
    canonical_bytes, ActionLifecyclePayload, AssuranceClaim, AssuranceRequirement,
    CapabilityDescriptor, ChunkManifest, CommandEnvelope, CommandResultPayload, CommandSentPayload,
    CommandType, ConstraintSet, DelegationRecord, EventEnvelope, LogicalObjectDescriptor,
    NodeGenesisPayload, ProtocolRecord, QueryRequest, QueryResultFragment, RelayEnvelope,
    RevocationRecord, ScopeDescriptor, SecretDeletePayload, SecretPutPayload, SessionAccept,
    SessionHello, SnapshotDescriptor, StoredRepresentationHeader,
};
use crate::result::{accept, defer, duplicate, empty_map, reject, ReasonCode, ValidationResult};
use crate::value::Value;
use edgerun_proto::edgerun::v0::stream::{CollectionCreatedPayload, CollectionDeletedPayload};

// ---------------------------------------------------------------------------
// Stream append validation (protobuf-native)
// ---------------------------------------------------------------------------

/// Validates a candidate event for appending to a stream.
///
/// Checks:
/// - Genesis event: seq=0, no prev_hash
/// - Non-genesis: seq = prev_seq + 1, prev_hash matches
/// - Writer identity consistency
/// - Signature verification
/// - Duplicate detection
///
/// This is the protobuf-native counterpart to
/// `validators.rs::validate_stream_append_case`.
pub fn validate_stream_append(
    candidate: &EventEnvelope,
    current_head: Option<&EventEnvelope>,
    writer_key_hint: Option<&[u8; 64]>,
) -> ValidationResult {
    // Structural: required fields
    if candidate.envelope_version != 1 {
        return reject(
            ReasonCode::VersionUnsupported,
            Value::String("unsupported EventEnvelope version".into()),
            empty_map(),
        );
    }
    if candidate.stream_id.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("empty stream_id".into()),
            empty_map(),
        );
    }
    let Some(event_type) =
        edgerun_proto::edgerun::v0::stream::EventType::from_i32(candidate.event_type)
    else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("invalid event_type".into()),
            empty_map(),
        );
    };
    if event_type == edgerun_proto::edgerun::v0::stream::EventType::Unspecified {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("invalid event_type".into()),
            empty_map(),
        );
    }
    if candidate.event_version == 0 {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("event_version is zero".into()),
            empty_map(),
        );
    }
    let Some(recorded_at) = &candidate.recorded_at else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("missing recorded_at".into()),
            empty_map(),
        );
    };
    if let Some(result) = validate_timestamp_shape(recorded_at, "EventEnvelope recorded_at") {
        return result;
    }
    if let Some(effective_at) = &candidate.effective_at {
        if let Some(result) = validate_timestamp_shape(effective_at, "EventEnvelope effective_at") {
            return result;
        }
    }
    if let Some(result) = validate_optional_object_ref(
        candidate.payload_object.as_ref(),
        "EventEnvelope payload_object",
    ) {
        return result;
    }
    for (index, related_event) in candidate.related_events.iter().enumerate() {
        if let Some(result) = validate_event_ref(
            related_event,
            &format!("EventEnvelope related_events[{index}]"),
        ) {
            return result;
        }
    }
    for (index, related_command) in candidate.related_commands.iter().enumerate() {
        if let Some(result) = validate_command_ref(
            Some(related_command),
            &format!("EventEnvelope related_commands[{index}]"),
        ) {
            return result;
        }
    }
    for (index, related_object) in candidate.related_objects.iter().enumerate() {
        if let Some(result) = validate_required_object_ref(
            related_object,
            &format!("EventEnvelope related_objects[{index}]"),
        ) {
            return result;
        }
    }
    for (index, related_delegation) in candidate.related_delegations.iter().enumerate() {
        if let Some(result) = validate_delegation_ref(
            related_delegation,
            &format!("EventEnvelope related_delegations[{index}]"),
        ) {
            return result;
        }
    }
    for (index, related_revocation) in candidate.related_revocations.iter().enumerate() {
        if let Some(result) = validate_revocation_ref(
            related_revocation,
            &format!("EventEnvelope related_revocations[{index}]"),
        ) {
            return result;
        }
    }
    if let Some(result) = validate_optional_object_ref(
        candidate.event_metadata.as_ref(),
        "EventEnvelope event_metadata",
    ) {
        return result;
    }

    let is_genesis = candidate.seq == 0;
    if is_genesis && event_type != edgerun_proto::edgerun::v0::stream::EventType::NodeGenesis {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("seq 0 event must be EVENT_TYPE_NODE_GENESIS".into()),
            empty_map(),
        );
    }
    if !is_genesis && event_type == edgerun_proto::edgerun::v0::stream::EventType::NodeGenesis {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("EVENT_TYPE_NODE_GENESIS is only valid at seq 0".into()),
            empty_map(),
        );
    }

    if is_genesis {
        if candidate.prev_event_hash.is_some() {
            return reject(
                ReasonCode::StructuralInvalid,
                Value::String("genesis event must not have prev_hash".into()),
                empty_map(),
            );
        }
        if current_head.is_some() {
            return duplicate(
                ReasonCode::ForkConflict,
                Value::String("genesis already exists".into()),
            );
        }
    } else {
        let Some(head) = current_head else {
            return defer(
                ReasonCode::MissingDependency,
                Value::String("no genesis event found".into()),
            );
        };

        if candidate.seq != head.seq + 1 {
            if candidate.seq <= head.seq {
                let head_hash = compute_event_hash(head);
                let cand_hash = compute_event_hash(candidate);
                if head_hash.value == cand_hash.value {
                    return duplicate(
                        ReasonCode::ReplayDetected,
                        Value::String("exact duplicate event".into()),
                    );
                }
                return reject(
                    ReasonCode::ForkConflict,
                    Value::String(format!(
                        "seq {} already exists with different event",
                        candidate.seq
                    )),
                    empty_map(),
                );
            }
            return defer(
                ReasonCode::MissingDependency,
                Value::String(format!(
                    "missing predecessor: expected seq {}, got {}",
                    head.seq + 1,
                    candidate.seq
                )),
            );
        }

        let expected_hash = compute_event_hash(head);
        if candidate.prev_event_hash.as_ref() != Some(&expected_hash) {
            return reject(
                ReasonCode::CryptoInvalid,
                Value::String("prev_hash mismatch".into()),
                empty_map(),
            );
        }
    }

    // Crypto: signature presence
    let Some(sig) = &candidate.signature else {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("missing signature".into()),
            empty_map(),
        );
    };
    if sig.algorithm != crate::crypto::SIGNATURE_ALGORITHM_ECDSA_P256 as i32 {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("unsupported event signature algorithm".into()),
            empty_map(),
        );
    }
    if sig.value.len() != crate::crypto::ECDSA_P256_SIGNATURE_LEN {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("event signature has invalid length".into()),
            empty_map(),
        );
    }

    // Signature verification (if key is available)
    if let Some(key_bytes) = writer_key_hint {
        if !verify_event_signature(candidate, key_bytes) {
            return reject(
                ReasonCode::CryptoInvalid,
                Value::String("signature verification failed".into()),
                empty_map(),
            );
        }
    }

    // Accept
    let mut derived = std::collections::BTreeMap::new();
    derived.insert("seq".into(), Value::Int(candidate.seq as i64));
    derived.insert(
        "stream_id".into(),
        Value::String(crate::util::bytes_to_hex(&candidate.stream_id)),
    );
    accept(Value::Map(derived), empty_map())
}

fn compute_event_hash(event: &EventEnvelope) -> crate::protocol::Digest {
    let record = ProtocolRecord::EventEnvelope(event.clone());
    let canonical = canonical_bytes(&record, true);
    let hash = crate::crypto::record_hash(crate::crypto::HASH_DOMAIN_EVENT_ENVELOPE, &canonical);
    crate::protocol::Digest {
        algorithm: 1,
        value: hash.to_vec(),
    }
}

fn verify_event_signature(event: &EventEnvelope, key: &[u8; 64]) -> bool {
    let Some(sig) = &event.signature else {
        return false;
    };

    if sig.algorithm != crate::crypto::SIGNATURE_ALGORITHM_ECDSA_P256 as i32 {
        return false;
    }

    if sig.value.len() != crate::crypto::ECDSA_P256_SIGNATURE_LEN {
        return false;
    }

    let vk = match crate::crypto::node_id_to_verifying_key(key) {
        Some(vk) => vk,
        None => return false,
    };

    let record = ProtocolRecord::EventEnvelope(event.clone());
    let canonical = canonical_bytes(&record, true);
    let record_hash =
        crate::crypto::record_hash(crate::crypto::HASH_DOMAIN_EVENT_ENVELOPE, &canonical);

    let sig_input =
        crate::crypto::signature_input(crate::crypto::SIG_DOMAIN_EVENT_ENVELOPE, &record_hash);

    use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashVerifier;
    let Ok(r): Result<[u8; 32], _> = sig.value[..32].try_into() else {
        return false;
    };
    let Ok(s): Result<[u8; 32], _> = sig.value[32..].try_into() else {
        return false;
    };
    let Ok(ecdsa_sig) = edgerun_crypto::p256::ecdsa::Signature::from_scalars(r, s) else {
        return false;
    };

    vk.verify_prehash(&sig_input, &ecdsa_sig).is_ok()
}

// ---------------------------------------------------------------------------
// Stream payload validation (protobuf-native)
// ---------------------------------------------------------------------------

/// Validates a node genesis payload before using it as bootstrap stream state.
pub fn validate_node_genesis_payload(payload: &NodeGenesisPayload) -> ValidationResult {
    if payload.payload_version != 1 {
        return reject(
            ReasonCode::VersionUnsupported,
            Value::String("unsupported NodeGenesisPayload version".into()),
            empty_map(),
        );
    }
    if payload.node_id.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("NodeGenesisPayload node_id is empty".into()),
            empty_map(),
        );
    }
    if let Some(result) = validate_identity_ref(
        payload.primary_node_identity.as_ref(),
        "NodeGenesisPayload primary_node_identity",
    ) {
        return result;
    }
    if payload.initial_controllers.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("NodeGenesisPayload initial_controllers is empty".into()),
            empty_map(),
        );
    }
    for controller in &payload.initial_controllers {
        if let Some(result) =
            validate_identity_ref(Some(controller), "NodeGenesisPayload initial_controller")
        {
            return result;
        }
    }
    if let Some(result) = validate_optional_object_ref(
        payload.initial_policy_object.as_ref(),
        "initial_policy_object",
    ) {
        return result;
    }
    for (index, bootstrap_record) in payload.bootstrap_records.iter().enumerate() {
        if let Some(result) = validate_required_object_ref(
            bootstrap_record,
            &format!("NodeGenesisPayload bootstrap_records[{index}]"),
        ) {
            return result;
        }
    }
    for (index, assurance_claim) in payload.assurance_claims.iter().enumerate() {
        if let Some(result) = validate_required_object_ref(
            assurance_claim,
            &format!("NodeGenesisPayload assurance_claims[{index}]"),
        ) {
            return result;
        }
    }
    if payload.node_roles.iter().any(|role| role.is_empty()) {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("NodeGenesisPayload node_role is empty".into()),
            empty_map(),
        );
    }
    if let Some(result) =
        validate_optional_object_ref(payload.genesis_metadata.as_ref(), "genesis_metadata")
    {
        return result;
    }

    let mut derived = std::collections::BTreeMap::new();
    derived.insert(
        "payload_family".into(),
        Value::String("node_genesis".into()),
    );
    accept(Value::Map(derived), empty_map())
}

/// Validates a command-sent payload before using it as durable send evidence.
pub fn validate_command_sent_payload(payload: &CommandSentPayload) -> ValidationResult {
    if payload.payload_version != 1 {
        return reject(
            ReasonCode::VersionUnsupported,
            Value::String("unsupported CommandSentPayload version".into()),
            empty_map(),
        );
    }
    if let Some(result) =
        validate_command_ref(payload.command.as_ref(), "CommandSentPayload command")
    {
        return result;
    }
    if let Some(result) = validate_node_ref(
        payload.target_node.as_ref(),
        "CommandSentPayload target_node",
    ) {
        return result;
    }
    if let Some(result) =
        validate_optional_object_ref(payload.send_metadata.as_ref(), "send_metadata")
    {
        return result;
    }

    let mut derived = std::collections::BTreeMap::new();
    derived.insert(
        "payload_family".into(),
        Value::String("command_sent".into()),
    );
    accept(Value::Map(derived), empty_map())
}

/// Validates the structural fields of a command result payload before indexing
/// or exposing it as command outcome evidence.
pub fn validate_command_result_payload(payload: &CommandResultPayload) -> ValidationResult {
    if payload.payload_version != 1 {
        return reject(
            ReasonCode::VersionUnsupported,
            Value::String("unsupported CommandResultPayload version".into()),
            empty_map(),
        );
    }
    if let Some(result) =
        validate_command_ref(payload.command.as_ref(), "CommandResultPayload command")
    {
        return result;
    }
    let Some(issuer) = &payload.issuer else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("CommandResultPayload missing issuer".into()),
            empty_map(),
        );
    };
    if let Some(result) = validate_identity_ref(Some(issuer), "CommandResultPayload issuer") {
        return result;
    }
    if edgerun_proto::edgerun::v0::stream::CommandDecision::from_i32(payload.decision).is_none_or(
        |decision| decision == edgerun_proto::edgerun::v0::stream::CommandDecision::Unspecified,
    ) {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("CommandResultPayload decision is invalid".into()),
            empty_map(),
        );
    }
    if let Some(result) =
        validate_optional_object_ref(payload.decision_basis.as_ref(), "decision_basis")
    {
        return result;
    }
    if let Some(result) = validate_optional_object_ref(
        payload.effect_summary_object.as_ref(),
        "effect_summary_object",
    ) {
        return result;
    }
    if let Some(result) =
        validate_optional_object_ref(payload.result_object.as_ref(), "result_object")
    {
        return result;
    }

    let mut derived = std::collections::BTreeMap::new();
    derived.insert(
        "payload_family".into(),
        Value::String("command_result".into()),
    );
    accept(Value::Map(derived), empty_map())
}

/// Validates the structural fields of an action lifecycle payload before it is
/// treated as command progress evidence.
pub fn validate_action_lifecycle_payload(payload: &ActionLifecyclePayload) -> ValidationResult {
    if payload.payload_version != 1 {
        return reject(
            ReasonCode::VersionUnsupported,
            Value::String("unsupported ActionLifecyclePayload version".into()),
            empty_map(),
        );
    }
    if let Some(result) = validate_command_ref(
        payload.origin_command.as_ref(),
        "ActionLifecyclePayload origin_command",
    ) {
        return result;
    }
    if payload.action_instance_id.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("ActionLifecyclePayload action_instance_id is empty".into()),
            empty_map(),
        );
    }
    let Some(status) = edgerun_proto::edgerun::v0::stream::ActionStatus::from_i32(payload.status)
    else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("ActionLifecyclePayload status is invalid".into()),
            empty_map(),
        );
    };
    if status == edgerun_proto::edgerun::v0::stream::ActionStatus::Unspecified {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("ActionLifecyclePayload status is invalid".into()),
            empty_map(),
        );
    }
    if let Some(result) =
        validate_optional_object_ref(payload.result_object.as_ref(), "result_object")
    {
        return result;
    }
    if let Some(result) =
        validate_optional_object_ref(payload.error_object.as_ref(), "error_object")
    {
        return result;
    }
    if let Some(result) =
        validate_optional_object_ref(payload.progress_object.as_ref(), "progress_object")
    {
        return result;
    }
    if let Some(result) =
        validate_optional_object_ref(payload.action_metadata.as_ref(), "action_metadata")
    {
        return result;
    }
    if status == edgerun_proto::edgerun::v0::stream::ActionStatus::Completed
        && payload.result_object.is_none()
    {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("ActionLifecyclePayload completed status requires result_object".into()),
            empty_map(),
        );
    }
    if status == edgerun_proto::edgerun::v0::stream::ActionStatus::Failed
        && payload.error_object.is_none()
    {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("ActionLifecyclePayload failed status requires error_object".into()),
            empty_map(),
        );
    }

    let mut derived = std::collections::BTreeMap::new();
    derived.insert(
        "payload_family".into(),
        Value::String("action_lifecycle".into()),
    );
    accept(Value::Map(derived), empty_map())
}

/// Validates metadata for a stored or rotated secret. The secret value itself is
/// intentionally outside this payload and remains in the encrypted blob store.
pub fn validate_secret_put_payload(payload: &SecretPutPayload) -> ValidationResult {
    if payload.payload_version != 1 {
        return reject(
            ReasonCode::VersionUnsupported,
            Value::String("unsupported SecretPutPayload version".into()),
            empty_map(),
        );
    }
    if let Some(result) = validate_required_string(&payload.namespace, "SecretPutPayload namespace")
    {
        return result;
    }
    if let Some(result) = validate_required_string(&payload.key, "SecretPutPayload key") {
        return result;
    }
    if let Some(result) = validate_required_string(&payload.label, "SecretPutPayload label") {
        return result;
    }
    if let Some(result) =
        validate_required_string(&payload.secret_blob_id, "SecretPutPayload secret_blob_id")
    {
        return result;
    }
    if !is_even_hex(&payload.secret_blob_id) {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("SecretPutPayload secret_blob_id must be even-length hex".into()),
            empty_map(),
        );
    }

    let mut derived = std::collections::BTreeMap::new();
    derived.insert("payload_family".into(), Value::String("secret_put".into()));
    accept(Value::Map(derived), empty_map())
}

/// Validates metadata for a deleted secret.
pub fn validate_secret_delete_payload(payload: &SecretDeletePayload) -> ValidationResult {
    if payload.payload_version != 1 {
        return reject(
            ReasonCode::VersionUnsupported,
            Value::String("unsupported SecretDeletePayload version".into()),
            empty_map(),
        );
    }
    if let Some(result) =
        validate_required_string(&payload.namespace, "SecretDeletePayload namespace")
    {
        return result;
    }
    if let Some(result) = validate_required_string(&payload.key, "SecretDeletePayload key") {
        return result;
    }
    if let Some(result) = validate_required_string(&payload.label, "SecretDeletePayload label") {
        return result;
    }

    let mut derived = std::collections::BTreeMap::new();
    derived.insert(
        "payload_family".into(),
        Value::String("secret_delete".into()),
    );
    accept(Value::Map(derived), empty_map())
}

/// Validates metadata for collection creation.
pub fn validate_collection_created_payload(payload: &CollectionCreatedPayload) -> ValidationResult {
    if payload.payload_version != 1 {
        return reject(
            ReasonCode::VersionUnsupported,
            Value::String("unsupported CollectionCreatedPayload version".into()),
            empty_map(),
        );
    }
    if let Some(result) = validate_required_string(
        &payload.collection_name,
        "CollectionCreatedPayload collection_name",
    ) {
        return result;
    }
    if let Some(result) = validate_required_string(&payload.label, "CollectionCreatedPayload label")
    {
        return result;
    }

    let mut derived = std::collections::BTreeMap::new();
    derived.insert(
        "payload_family".into(),
        Value::String("collection_created".into()),
    );
    accept(Value::Map(derived), empty_map())
}

/// Validates metadata for collection deletion.
pub fn validate_collection_deleted_payload(payload: &CollectionDeletedPayload) -> ValidationResult {
    if payload.payload_version != 1 {
        return reject(
            ReasonCode::VersionUnsupported,
            Value::String("unsupported CollectionDeletedPayload version".into()),
            empty_map(),
        );
    }
    if let Some(result) = validate_required_string(
        &payload.collection_name,
        "CollectionDeletedPayload collection_name",
    ) {
        return result;
    }

    let mut derived = std::collections::BTreeMap::new();
    derived.insert(
        "payload_family".into(),
        Value::String("collection_deleted".into()),
    );
    accept(Value::Map(derived), empty_map())
}

fn validate_required_string(value: &str, label: &str) -> Option<ValidationResult> {
    if value.is_empty() {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} is empty")),
            empty_map(),
        ));
    }
    None
}

fn is_even_hex(value: &str) -> bool {
    !value.is_empty() && value.len() % 2 == 0 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_command_ref(
    command: Option<&crate::protocol::CommandRef>,
    label: &str,
) -> Option<ValidationResult> {
    let Some(command) = command else {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} is missing")),
            empty_map(),
        ));
    };
    if command.command_id.is_empty() {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} command_id is empty")),
            empty_map(),
        ));
    }
    let Some(command_hash) = &command.command_hash else {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} command_hash is missing")),
            empty_map(),
        ));
    };
    if let Some(result) = validate_sha256_digest(command_hash, &format!("{label} command_hash")) {
        return Some(result);
    }
    None
}

fn validate_identity_ref(
    identity: Option<&crate::protocol::IdentityRef>,
    label: &str,
) -> Option<ValidationResult> {
    let Some(identity) = identity else {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} is missing")),
            empty_map(),
        ));
    };
    if identity.identity_id.is_empty() {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} identity_id is empty")),
            empty_map(),
        ));
    }
    if let Some(identity_kind) = identity.identity_kind {
        if edgerun_proto::edgerun::v0::common::IdentityKind::from_i32(identity_kind).is_none_or(
            |kind| kind == edgerun_proto::edgerun::v0::common::IdentityKind::Unspecified,
        ) {
            return Some(reject(
                ReasonCode::StructuralInvalid,
                Value::String(format!("{label} identity_kind is invalid")),
                empty_map(),
            ));
        }
    }
    None
}

fn validate_node_ref(
    node: Option<&crate::protocol::NodeRef>,
    label: &str,
) -> Option<ValidationResult> {
    let Some(node) = node else {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} is missing")),
            empty_map(),
        ));
    };
    if node.node_id.is_empty() {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} node_id is empty")),
            empty_map(),
        ));
    }
    None
}

fn validate_stream_ref(
    stream: &crate::protocol::StreamRef,
    label: &str,
) -> Option<ValidationResult> {
    if stream.stream_id.is_empty() {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} stream_id is empty")),
            empty_map(),
        ));
    }
    None
}

fn validate_optional_object_ref(
    object: Option<&crate::protocol::ObjectRef>,
    label: &str,
) -> Option<ValidationResult> {
    if let Some(object) = object {
        if let Some(result) = validate_object_ref_fields(object, label) {
            return Some(result);
        }
    }
    None
}

fn validate_required_object_ref(
    object: &crate::protocol::ObjectRef,
    label: &str,
) -> Option<ValidationResult> {
    validate_object_ref_fields(object, label)
}

fn validate_object_ref_fields(
    object: &crate::protocol::ObjectRef,
    label: &str,
) -> Option<ValidationResult> {
    if object.object_id.is_empty() {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} object_id is empty")),
            empty_map(),
        ));
    }
    if let Some(object_kind) = object.object_kind {
        if edgerun_proto::edgerun::v0::common::ObjectKind::from_i32(object_kind)
            .is_none_or(|kind| kind == edgerun_proto::edgerun::v0::common::ObjectKind::Unspecified)
        {
            return Some(reject(
                ReasonCode::StructuralInvalid,
                Value::String(format!("{label} object_kind is invalid")),
                empty_map(),
            ));
        }
    }
    None
}

fn validate_sha256_digest(
    digest: &crate::protocol::Digest,
    label: &str,
) -> Option<ValidationResult> {
    if digest.algorithm
        != edgerun_proto::edgerun::v0::common::digest::Algorithm::DigestAlgorithmSha256 as i32
    {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} algorithm is not SHA-256")),
            empty_map(),
        ));
    }
    if digest.value.len() != 32 {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} must be 32 bytes")),
            empty_map(),
        ));
    }
    None
}

fn validate_event_ref(event: &crate::protocol::EventRef, label: &str) -> Option<ValidationResult> {
    if event.stream_id.is_empty() {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} stream_id is empty")),
            empty_map(),
        ));
    }
    let Some(event_hash) = &event.event_hash else {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} event_hash is missing")),
            empty_map(),
        ));
    };
    if let Some(result) = validate_sha256_digest(event_hash, &format!("{label} event_hash")) {
        return Some(result);
    }
    None
}

fn validate_delegation_ref(
    delegation: &crate::protocol::DelegationRef,
    label: &str,
) -> Option<ValidationResult> {
    if delegation.delegation_id.is_empty() {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} delegation_id is empty")),
            empty_map(),
        ));
    }
    let Some(delegation_hash) = &delegation.delegation_hash else {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} delegation_hash is missing")),
            empty_map(),
        ));
    };
    if let Some(result) =
        validate_sha256_digest(delegation_hash, &format!("{label} delegation_hash"))
    {
        return Some(result);
    }
    None
}

fn validate_revocation_ref(
    revocation: &crate::protocol::RevocationRef,
    label: &str,
) -> Option<ValidationResult> {
    if revocation.revocation_id.is_empty() {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} revocation_id is empty")),
            empty_map(),
        ));
    }
    let Some(revocation_hash) = &revocation.revocation_hash else {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} revocation_hash is missing")),
            empty_map(),
        ));
    };
    if let Some(result) =
        validate_sha256_digest(revocation_hash, &format!("{label} revocation_hash"))
    {
        return Some(result);
    }
    None
}

fn validate_reachability_hint(
    hint: &edgerun_proto::edgerun::v0::network::ReachabilityHint,
    label: &str,
) -> Option<ValidationResult> {
    if hint.hint_version != 1 {
        return Some(reject(
            ReasonCode::VersionUnsupported,
            Value::String(format!("{label} has unsupported version")),
            empty_map(),
        ));
    }
    if let Some(result) =
        validate_node_ref(hint.subject_node.as_ref(), &format!("{label} subject_node"))
    {
        return Some(result);
    }
    if edgerun_proto::edgerun::v0::common::TransportClass::from_i32(hint.transport_class)
        .is_none_or(|class| {
            class == edgerun_proto::edgerun::v0::common::TransportClass::Unspecified
        })
    {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} has invalid transport_class")),
            empty_map(),
        ));
    }
    if hint.locator_payload.is_empty() {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} locator_payload is empty")),
            empty_map(),
        ));
    }
    if edgerun_proto::edgerun::v0::common::Directness::from_i32(hint.directness).is_none_or(
        |directness| directness == edgerun_proto::edgerun::v0::common::Directness::Unspecified,
    ) {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} has invalid directness")),
            empty_map(),
        ));
    }
    if let Some(valid_after) = &hint.valid_after {
        if let Some(result) = validate_timestamp_shape(valid_after, &format!("{label} valid_after"))
        {
            return Some(result);
        }
    }
    if let Some(valid_until) = &hint.valid_until {
        if let Some(result) = validate_timestamp_shape(valid_until, &format!("{label} valid_until"))
        {
            return Some(result);
        }
    }
    if let (Some(valid_after), Some(valid_until)) = (&hint.valid_after, &hint.valid_until) {
        if timestamp_ms(valid_after) > timestamp_ms(valid_until) {
            return Some(reject(
                ReasonCode::TimeInvalid,
                Value::String(format!("{label} validity window is inverted")),
                empty_map(),
            ));
        }
    }
    if let Some(issuer) = &hint.issuer {
        if let Some(result) = validate_identity_ref(Some(issuer), &format!("{label} issuer")) {
            return Some(result);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Delegation chain validation (protobuf-native)
// ---------------------------------------------------------------------------

/// Validates a delegation chain without an associated command.
///
/// This is the protobuf-native counterpart to
/// `validators.rs::validate_delegation_case`.
pub fn validate_delegation_chain(
    chain: &[DelegationRecord],
    now_ms: i64,
    revoked_ids: &std::collections::HashSet<Vec<u8>>,
) -> ValidationResult {
    if chain.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("empty delegation chain".into()),
            empty_map(),
        );
    }

    // Check continuity first: each delegation's recipient matches next issuer
    for i in 1..chain.len() {
        let prev_recipient = chain[i - 1]
            .recipient
            .as_ref()
            .map(|r| r.identity_id.clone());
        let curr_issuer = chain[i].issuer.as_ref().map(|r| r.identity_id.clone());
        if prev_recipient != curr_issuer {
            return reject(
                ReasonCode::AuthorityDenied,
                Value::String("delegation chain broken: recipient != next issuer".into()),
                empty_map(),
            );
        }
    }

    for (i, delegation) in chain.iter().enumerate() {
        if delegation.record_version != 1 {
            return reject(
                ReasonCode::VersionUnsupported,
                Value::String("unsupported DelegationRecord version".into()),
                empty_map(),
            );
        }
        if delegation.delegation_id.is_empty() {
            return reject(
                ReasonCode::StructuralInvalid,
                Value::String("delegation_id is empty".into()),
                empty_map(),
            );
        }
        let Some(issuer) = &delegation.issuer else {
            return reject(
                ReasonCode::StructuralInvalid,
                Value::String("delegation missing issuer".into()),
                empty_map(),
            );
        };
        if let Some(result) = validate_identity_ref(Some(issuer), "delegation issuer") {
            return result;
        }
        let Some(recipient) = &delegation.recipient else {
            return reject(
                ReasonCode::StructuralInvalid,
                Value::String("delegation missing recipient".into()),
                empty_map(),
            );
        };
        if let Some(result) = validate_identity_ref(Some(recipient), "delegation recipient") {
            return result;
        }
        let Some(issued_at) = &delegation.issued_at else {
            return reject(
                ReasonCode::StructuralInvalid,
                Value::String("delegation missing issued_at".into()),
                empty_map(),
            );
        };
        if let Some(result) = validate_timestamp_shape(issued_at, "delegation issued_at") {
            return result;
        }
        if now_ms < timestamp_ms(issued_at) {
            return defer(
                ReasonCode::TimeInvalid,
                Value::String("delegation issued_at is in the future".into()),
            );
        }
        let Some(capability) = &delegation.capability else {
            return reject(
                ReasonCode::StructuralInvalid,
                Value::String("delegation missing capability".into()),
                empty_map(),
            );
        };
        if let Some(result) = validate_capability_descriptor(capability) {
            return result;
        }
        if let Some(parent_delegation) = &delegation.parent_delegation {
            if let Some(result) =
                validate_delegation_ref(parent_delegation, "delegation parent_delegation")
            {
                return result;
            }
        }
        for (index, authority) in delegation.revocation_authorities.iter().enumerate() {
            if let Some(result) = validate_identity_ref(
                Some(authority),
                &format!("delegation revocation_authorities[{index}]"),
            ) {
                return result;
            }
        }
        if let Some(result) = validate_optional_object_ref(
            delegation.delegation_metadata.as_ref(),
            "delegation metadata",
        ) {
            return result;
        }

        // Check revocation
        if revoked_ids.contains(&delegation.delegation_id) {
            return reject(
                ReasonCode::RevocationActive,
                Value::String(format!(
                    "delegation {} is revoked",
                    crate::util::bytes_to_hex(&delegation.delegation_id)
                )),
                empty_map(),
            );
        }

        // Check timing: expires_at
        if let Some(ref expires_at) = delegation.expires_at {
            if let Some(result) = validate_timestamp_shape(expires_at, "delegation expires_at") {
                return result;
            }
            if now_ms > timestamp_ms(expires_at) {
                return reject(
                    ReasonCode::TimeInvalid,
                    Value::String("delegation has expired".into()),
                    empty_map(),
                );
            }
        }

        // Check timing: not_before
        if let Some(ref not_before) = delegation.not_before {
            if let Some(result) = validate_timestamp_shape(not_before, "delegation not_before") {
                return result;
            }
            if now_ms < timestamp_ms(not_before) {
                return defer(
                    ReasonCode::TimeInvalid,
                    Value::String("delegation not yet valid".into()),
                );
            }
        }

        if !verify_delegation_signature(delegation) {
            return reject(
                ReasonCode::CryptoInvalid,
                Value::String("delegation signature verification failed".into()),
                empty_map(),
            );
        }

        // Attenuation: child must not expand parent's actions
        if i > 0 {
            let parent = &chain[i - 1];
            let Some(parent_cap) = &parent.capability else {
                return reject(
                    ReasonCode::StructuralInvalid,
                    Value::String("parent has no capability".into()),
                    empty_map(),
                );
            };
            let Some(child_cap) = &delegation.capability else {
                return reject(
                    ReasonCode::StructuralInvalid,
                    Value::String("child has no capability".into()),
                    empty_map(),
                );
            };
            if parent_cap.delegation_policy
                == edgerun_proto::edgerun::v0::trust::DelegationPolicy::NonDelegable as i32
            {
                return reject(
                    ReasonCode::AuthorityDenied,
                    Value::String("parent delegation is non-delegable".into()),
                    empty_map(),
                );
            }
            let parent_actions: std::collections::HashSet<&String> =
                parent_cap.actions.iter().collect();
            for action in &child_cap.actions {
                if !parent_actions.contains(action) {
                    return reject(
                        ReasonCode::AuthorityDenied,
                        Value::String(format!(
                            "attenuation violated: child adds action '{}'",
                            action
                        )),
                        empty_map(),
                    );
                }
            }
            let Some(parent_scope) = &parent_cap.scope else {
                return reject(
                    ReasonCode::StructuralInvalid,
                    Value::String("parent has no scope".into()),
                    empty_map(),
                );
            };
            let Some(child_scope) = &child_cap.scope else {
                return reject(
                    ReasonCode::StructuralInvalid,
                    Value::String("child has no scope".into()),
                    empty_map(),
                );
            };
            if !scope_descriptor_allows(parent_scope, child_scope) {
                return reject(
                    ReasonCode::AuthorityDenied,
                    Value::String("attenuation violated: child expands scope".into()),
                    empty_map(),
                );
            }
        }
    }

    let mut derived = std::collections::BTreeMap::new();
    derived.insert("chain_length".into(), Value::Int(chain.len() as i64));
    derived.insert(
        "root_issuer".into(),
        Value::String(crate::util::bytes_to_hex(
            &chain[0]
                .issuer
                .as_ref()
                .map(|i| i.identity_id.clone())
                .unwrap_or_default(),
        )),
    );
    derived.insert(
        "leaf_recipient".into(),
        Value::String(crate::util::bytes_to_hex(
            &chain
                .last()
                .unwrap()
                .recipient
                .as_ref()
                .map(|r| r.identity_id.clone())
                .unwrap_or_default(),
        )),
    );
    accept(Value::Map(derived), empty_map())
}

fn validate_capability_descriptor(capability: &CapabilityDescriptor) -> Option<ValidationResult> {
    if capability.capability_version != 1 {
        return Some(reject(
            ReasonCode::VersionUnsupported,
            Value::String("unsupported CapabilityDescriptor version".into()),
            empty_map(),
        ));
    }
    if edgerun_proto::edgerun::v0::trust::CapabilityKind::from_i32(capability.capability_kind)
        .is_none_or(|kind| kind == edgerun_proto::edgerun::v0::trust::CapabilityKind::Unspecified)
    {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String("capability_kind is invalid".into()),
            empty_map(),
        ));
    }
    if capability.actions.is_empty() || capability.actions.iter().any(|action| action.is_empty()) {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String("capability actions are empty".into()),
            empty_map(),
        ));
    }
    let Some(scope) = &capability.scope else {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String("capability missing scope".into()),
            empty_map(),
        ));
    };
    if let Some(result) = validate_scope_descriptor(scope, "capability scope") {
        return Some(result);
    }
    if let Some(constraints) = &capability.constraints {
        if let Some(result) = validate_constraint_set(constraints) {
            return Some(result);
        }
    }
    if edgerun_proto::edgerun::v0::trust::DelegationPolicy::from_i32(capability.delegation_policy)
        .is_none_or(|policy| {
            policy == edgerun_proto::edgerun::v0::trust::DelegationPolicy::Unspecified
        })
    {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String("delegation_policy is invalid".into()),
            empty_map(),
        ));
    }
    if let Some(result) = validate_optional_object_ref(
        capability.capability_metadata.as_ref(),
        "capability metadata",
    ) {
        return Some(result);
    }
    None
}

fn validate_constraint_set(constraints: &ConstraintSet) -> Option<ValidationResult> {
    if constraints.constraint_version != 1 {
        return Some(reject(
            ReasonCode::VersionUnsupported,
            Value::String("unsupported ConstraintSet version".into()),
            empty_map(),
        ));
    }
    if let (Some(not_before), Some(expires_at)) = (&constraints.not_before, &constraints.expires_at)
    {
        if let Some(result) = validate_timestamp_shape(not_before, "ConstraintSet not_before") {
            return Some(result);
        }
        if let Some(result) = validate_timestamp_shape(expires_at, "ConstraintSet expires_at") {
            return Some(result);
        }
        if timestamp_ms(not_before) > timestamp_ms(expires_at) {
            return Some(reject(
                ReasonCode::TimeInvalid,
                Value::String("ConstraintSet time window is inverted".into()),
                empty_map(),
            ));
        }
    } else {
        if let Some(not_before) = &constraints.not_before {
            if let Some(result) = validate_timestamp_shape(not_before, "ConstraintSet not_before") {
                return Some(result);
            }
        }
        if let Some(expires_at) = &constraints.expires_at {
            if let Some(result) = validate_timestamp_shape(expires_at, "ConstraintSet expires_at") {
                return Some(result);
            }
        }
    }
    if constraints.max_uses == Some(0) {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String("ConstraintSet max_uses must be positive".into()),
            empty_map(),
        ));
    }
    if let Some(rate_limit) = &constraints.rate_limit {
        if rate_limit.max_operations == 0 {
            return Some(reject(
                ReasonCode::StructuralInvalid,
                Value::String("ConstraintSet rate_limit max_operations must be positive".into()),
                empty_map(),
            ));
        }
        let Some(per) = &rate_limit.per else {
            return Some(reject(
                ReasonCode::StructuralInvalid,
                Value::String("ConstraintSet rate_limit period is missing".into()),
                empty_map(),
            ));
        };
        if !is_valid_non_negative_duration(per) || (per.seconds == 0 && per.nanos == 0) {
            return Some(reject(
                ReasonCode::TimeInvalid,
                Value::String("ConstraintSet rate_limit period must be positive".into()),
                empty_map(),
            ));
        }
    }
    for transport_class in &constraints.requires_transport_classes {
        if edgerun_proto::edgerun::v0::common::TransportClass::from_i32(*transport_class)
            .is_none_or(|class| {
                class == edgerun_proto::edgerun::v0::common::TransportClass::Unspecified
            })
        {
            return Some(reject(
                ReasonCode::StructuralInvalid,
                Value::String("ConstraintSet has invalid transport class".into()),
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
            Value::String("ConstraintSet location class is empty".into()),
            empty_map(),
        ));
    }
    if constraints.export_policy != 0
        && edgerun_proto::edgerun::v0::trust::ExportPolicy::from_i32(constraints.export_policy)
            .is_none()
    {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String("ConstraintSet has invalid export policy".into()),
            empty_map(),
        ));
    }
    for execution_class in &constraints.execution_class_limits {
        if edgerun_proto::edgerun::v0::common::ExecutionClass::from_i32(*execution_class)
            .is_none_or(|class| {
                class == edgerun_proto::edgerun::v0::common::ExecutionClass::Unspecified
            })
        {
            return Some(reject(
                ReasonCode::StructuralInvalid,
                Value::String("ConstraintSet has invalid execution class".into()),
                empty_map(),
            ));
        }
    }
    for storage_class in &constraints.storage_class_limits {
        if edgerun_proto::edgerun::v0::common::StorageClass::from_i32(*storage_class).is_none_or(
            |class| class == edgerun_proto::edgerun::v0::common::StorageClass::Unspecified,
        ) {
            return Some(reject(
                ReasonCode::StructuralInvalid,
                Value::String("ConstraintSet has invalid storage class".into()),
                empty_map(),
            ));
        }
    }
    if let Some(result) = validate_optional_object_ref(
        constraints.constraint_metadata.as_ref(),
        "ConstraintSet metadata",
    ) {
        return Some(result);
    }
    None
}

fn validate_scope_descriptor(scope: &ScopeDescriptor, label: &str) -> Option<ValidationResult> {
    if scope.scope_version != 1 {
        return Some(reject(
            ReasonCode::VersionUnsupported,
            Value::String(format!("{label} has unsupported version")),
            empty_map(),
        ));
    }
    if edgerun_proto::edgerun::v0::trust::ScopeKind::from_i32(scope.scope_kind)
        .is_none_or(|kind| kind == edgerun_proto::edgerun::v0::trust::ScopeKind::Unspecified)
    {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} has invalid scope_kind")),
            empty_map(),
        ));
    }
    for (index, node) in scope.target_nodes.iter().enumerate() {
        if let Some(result) =
            validate_node_ref(Some(node), &format!("{label} target_nodes[{index}]"))
        {
            return Some(result);
        }
    }
    for (index, stream) in scope.target_streams.iter().enumerate() {
        if let Some(result) =
            validate_stream_ref(stream, &format!("{label} target_streams[{index}]"))
        {
            return Some(result);
        }
    }
    for object_kind in &scope.target_object_kinds {
        if edgerun_proto::edgerun::v0::common::ObjectKind::from_i32(*object_kind)
            .is_none_or(|kind| kind == edgerun_proto::edgerun::v0::common::ObjectKind::Unspecified)
        {
            return Some(reject(
                ReasonCode::StructuralInvalid,
                Value::String(format!("{label} has invalid target object kind")),
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
        if let (Some(not_before), Some(expires_at)) =
            (&time_bounds.not_before, &time_bounds.expires_at)
        {
            if let Some(result) =
                validate_timestamp_shape(not_before, &format!("{label} time_bounds not_before"))
            {
                return Some(result);
            }
            if let Some(result) =
                validate_timestamp_shape(expires_at, &format!("{label} time_bounds expires_at"))
            {
                return Some(result);
            }
            if timestamp_ms(not_before) > timestamp_ms(expires_at) {
                return Some(reject(
                    ReasonCode::TimeInvalid,
                    Value::String(format!("{label} time_bounds are inverted")),
                    empty_map(),
                ));
            }
        } else {
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
        }
    }
    if let Some(result) =
        validate_optional_object_ref(scope.scope_metadata.as_ref(), &format!("{label} metadata"))
    {
        return Some(result);
    }
    None
}

fn scope_descriptor_allows(parent: &ScopeDescriptor, child: &ScopeDescriptor) -> bool {
    let parent_kind = edgerun_proto::edgerun::v0::trust::ScopeKind::from_i32(parent.scope_kind);
    let child_kind = edgerun_proto::edgerun::v0::trust::ScopeKind::from_i32(child.scope_kind);
    if parent_kind == Some(edgerun_proto::edgerun::v0::trust::ScopeKind::GlobalWithConstraints) {
        return time_window_allows(parent.time_bounds.as_ref(), child.time_bounds.as_ref());
    }
    if parent_kind != child_kind {
        return false;
    }

    node_targets_allow(&parent.target_nodes, &child.target_nodes)
        && stream_targets_allow(&parent.target_streams, &child.target_streams)
        && scalar_targets_allow(&parent.target_object_kinds, &child.target_object_kinds)
        && scalar_targets_allow(&parent.target_view_types, &child.target_view_types)
        && scalar_targets_allow(&parent.target_domains, &child.target_domains)
        && time_window_allows(parent.time_bounds.as_ref(), child.time_bounds.as_ref())
}

fn node_targets_allow(
    parent: &[crate::protocol::NodeRef],
    child: &[crate::protocol::NodeRef],
) -> bool {
    parent.is_empty()
        || child.iter().all(|needle| {
            parent
                .iter()
                .any(|haystack| haystack.node_id == needle.node_id)
        })
}

fn stream_targets_allow(
    parent: &[crate::protocol::StreamRef],
    child: &[crate::protocol::StreamRef],
) -> bool {
    parent.is_empty()
        || child.iter().all(|needle| {
            parent
                .iter()
                .any(|haystack| haystack.stream_id == needle.stream_id)
        })
}

fn scalar_targets_allow<T: Eq>(parent: &[T], child: &[T]) -> bool {
    parent.is_empty() || child.iter().all(|needle| parent.contains(needle))
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
        if timestamp_ms(child_not_before) < timestamp_ms(parent_not_before) {
            return false;
        }
    }
    if let Some(parent_expires_at) = &parent.expires_at {
        let Some(child_expires_at) = &child.expires_at else {
            return false;
        };
        if timestamp_ms(child_expires_at) > timestamp_ms(parent_expires_at) {
            return false;
        }
    }
    true
}

fn verify_delegation_signature(delegation: &DelegationRecord) -> bool {
    let Some(sig) = &delegation.signature else {
        return false;
    };
    if sig.algorithm != crate::crypto::SIGNATURE_ALGORITHM_ECDSA_P256 as i32 {
        return false;
    }
    if sig.value.len() != crate::crypto::ECDSA_P256_SIGNATURE_LEN {
        return false;
    }

    let Some(issuer) = &delegation.issuer else {
        return false;
    };
    let Some(key_hint) = &issuer.key_hint else {
        return false;
    };
    if key_hint.len() != crate::crypto::ECDSA_P256_PUBLIC_KEY_LEN {
        return false;
    }
    let Ok(key_hint): Result<[u8; 64], _> = key_hint.as_slice().try_into() else {
        return false;
    };
    let Some(vk) = crate::crypto::node_id_to_verifying_key(&key_hint) else {
        return false;
    };

    let record = ProtocolRecord::DelegationRecord(delegation.clone());
    let canonical = canonical_bytes(&record, true);
    crate::crypto::verify_canonical_record(
        &vk,
        crate::crypto::SIG_DOMAIN_DELEGATION_RECORD,
        &canonical,
        &sig.value,
    )
}

// ---------------------------------------------------------------------------
// Snapshot validation (protobuf-native)
// ---------------------------------------------------------------------------

/// Validates a snapshot descriptor.
///
/// This is the protobuf-native counterpart to
/// `validators.rs::validate_snapshot_case`.
pub fn validate_snapshot(
    snapshot: &SnapshotDescriptor,
    trusted_producers: &[Vec<u8>],
) -> ValidationResult {
    // Structural: required fields
    if snapshot.descriptor_version != 1 {
        return reject(
            ReasonCode::VersionUnsupported,
            Value::String("unsupported SnapshotDescriptor version".into()),
            empty_map(),
        );
    }
    if snapshot.snapshot_id.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("empty snapshot_id".into()),
            empty_map(),
        );
    }
    if snapshot.view_type.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("snapshot view_type is empty".into()),
            empty_map(),
        );
    }
    if snapshot.view_version == 0 {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("snapshot view_version is zero".into()),
            empty_map(),
        );
    }
    let Some(producer) = &snapshot.producer else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("missing producer".into()),
            empty_map(),
        );
    };
    if let Some(result) = validate_identity_ref(Some(producer), "snapshot producer") {
        return result;
    }
    let Some(produced_at) = &snapshot.produced_at else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("missing produced_at".into()),
            empty_map(),
        );
    };
    if let Some(result) = validate_timestamp_shape(produced_at, "snapshot produced_at") {
        return result;
    }
    let Some(scope) = &snapshot.scope else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("missing snapshot scope".into()),
            empty_map(),
        );
    };
    if let Some(result) = validate_scope_descriptor(scope, "snapshot scope") {
        return result;
    }
    if edgerun_proto::edgerun::v0::access::SnapshotCompleteness::from_i32(snapshot.completeness)
        .is_none_or(|completeness| {
            completeness == edgerun_proto::edgerun::v0::access::SnapshotCompleteness::Unspecified
        })
    {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("snapshot completeness is invalid".into()),
            empty_map(),
        );
    }

    let Some(payload_object) = &snapshot.payload_object else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("snapshot missing payload_object".into()),
            empty_map(),
        );
    };
    if let Some(result) = validate_required_object_ref(payload_object, "snapshot payload") {
        return result;
    }

    // Producer trust
    if !trusted_producers.is_empty() {
        if !trusted_producers.contains(&producer.identity_id) {
            return reject(
                ReasonCode::AuthorityDenied,
                Value::String("snapshot producer not in trusted set".into()),
                empty_map(),
            );
        }
    }

    if !verify_snapshot_signature(snapshot) {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("snapshot signature verification failed".into()),
            empty_map(),
        );
    }

    // Base heads or checkpoints must be present (at least one)
    if snapshot.base_heads.is_empty() && snapshot.base_checkpoints.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("snapshot has no base heads or checkpoints".into()),
            empty_map(),
        );
    }
    for head in &snapshot.base_heads {
        if let Some(result) = validate_head_ref(head, "snapshot base_head") {
            return result;
        }
    }
    for checkpoint in &snapshot.base_checkpoints {
        if let Some(result) = validate_checkpoint_ref(checkpoint, "snapshot base_checkpoint") {
            return result;
        }
    }
    if let Some(supersedes) = &snapshot.supersedes {
        if let Some(result) = validate_snapshot_ref(supersedes, "snapshot supersedes") {
            return result;
        }
    }
    if let Some(result) =
        validate_optional_object_ref(snapshot.snapshot_metadata.as_ref(), "snapshot metadata")
    {
        return result;
    }

    let mut derived = std::collections::BTreeMap::new();
    derived.insert(
        "snapshot_id".into(),
        Value::String(crate::util::bytes_to_hex(&snapshot.snapshot_id)),
    );
    derived.insert(
        "producer".into(),
        Value::String(crate::util::bytes_to_hex(
            &snapshot
                .producer
                .as_ref()
                .map(|p| p.identity_id.clone())
                .unwrap_or_default(),
        )),
    );
    derived.insert(
        "view_type".into(),
        Value::String(snapshot.view_type.clone()),
    );
    accept(Value::Map(derived), empty_map())
}

fn validate_head_ref(head: &crate::protocol::HeadRef, label: &str) -> Option<ValidationResult> {
    if head.stream_id.is_empty() {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} stream_id is empty")),
            empty_map(),
        ));
    }
    let Some(event_hash) = &head.event_hash else {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} event_hash is missing")),
            empty_map(),
        ));
    };
    if let Some(result) = validate_sha256_digest(event_hash, &format!("{label} event_hash")) {
        return Some(result);
    }
    None
}

fn validate_checkpoint_ref(
    checkpoint: &crate::protocol::CheckpointRef,
    label: &str,
) -> Option<ValidationResult> {
    if checkpoint
        .checkpoint_id
        .as_ref()
        .is_none_or(|id| id.is_empty())
        && checkpoint.heads.is_empty()
    {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} has no checkpoint_id or heads")),
            empty_map(),
        ));
    }
    for head in &checkpoint.heads {
        if let Some(result) = validate_head_ref(head, label) {
            return Some(result);
        }
    }
    None
}

fn validate_snapshot_ref(
    snapshot: &crate::protocol::SnapshotRef,
    label: &str,
) -> Option<ValidationResult> {
    if snapshot.snapshot_id.is_empty() {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} snapshot_id is empty")),
            empty_map(),
        ));
    }
    if snapshot.object_id.as_ref().is_some_and(|id| id.is_empty()) {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} object_id is empty")),
            empty_map(),
        ));
    }
    None
}

fn verify_snapshot_signature(snapshot: &SnapshotDescriptor) -> bool {
    let Some(sig) = &snapshot.signature else {
        return false;
    };
    if sig.algorithm != crate::crypto::SIGNATURE_ALGORITHM_ECDSA_P256 as i32 {
        return false;
    }
    if sig.value.len() != crate::crypto::ECDSA_P256_SIGNATURE_LEN {
        return false;
    }

    let Some(producer) = &snapshot.producer else {
        return false;
    };
    let Some(key_hint) = &producer.key_hint else {
        return false;
    };
    if key_hint.len() != crate::crypto::ECDSA_P256_PUBLIC_KEY_LEN {
        return false;
    }
    let Ok(key_hint): Result<[u8; 64], _> = key_hint.as_slice().try_into() else {
        return false;
    };
    let Some(vk) = crate::crypto::node_id_to_verifying_key(&key_hint) else {
        return false;
    };

    let record = ProtocolRecord::SnapshotDescriptor(snapshot.clone());
    let canonical = canonical_bytes(&record, true);
    crate::crypto::verify_canonical_record(
        &vk,
        crate::crypto::SIG_DOMAIN_SNAPSHOT_DESCRIPTOR,
        &canonical,
        &sig.value,
    )
}

// ---------------------------------------------------------------------------
// Object retrieval validation (protobuf-native)
// ---------------------------------------------------------------------------

/// Validates the typed object retrieval surface for a descriptor, stored
/// representation header, optional chunk manifest, and available bytes.
pub fn validate_object_retrieval(
    descriptor: Option<&LogicalObjectDescriptor>,
    header: Option<&StoredRepresentationHeader>,
    manifest: Option<&ChunkManifest>,
    logical_bytes: Option<&[u8]>,
    stored_bytes: Option<&[u8]>,
) -> ValidationResult {
    if descriptor.is_none() && header.is_none() && logical_bytes.is_none() {
        return defer(
            ReasonCode::MissingDependency,
            Value::String("object descriptor or bytes not available".into()),
        );
    }

    if let Some(descriptor) = descriptor {
        if descriptor.descriptor_version != 1 {
            return reject(
                ReasonCode::VersionUnsupported,
                Value::String("unsupported descriptor version".into()),
                empty_map(),
            );
        }
        if descriptor.object_id.is_empty() {
            return reject(
                ReasonCode::StructuralInvalid,
                Value::String("descriptor object_id is empty".into()),
                empty_map(),
            );
        }
        if edgerun_proto::edgerun::v0::common::ObjectKind::from_i32(descriptor.object_kind)
            .is_none_or(|kind| kind == edgerun_proto::edgerun::v0::common::ObjectKind::Unspecified)
        {
            return reject(
                ReasonCode::StructuralInvalid,
                Value::String("descriptor object_kind is invalid".into()),
                empty_map(),
            );
        }
        if descriptor.object_schema_version == 0 {
            return reject(
                ReasonCode::StructuralInvalid,
                Value::String("descriptor object_schema_version is zero".into()),
                empty_map(),
            );
        }
        if descriptor.canonicalization_id.is_empty() {
            return reject(
                ReasonCode::StructuralInvalid,
                Value::String("descriptor canonicalization_id is empty".into()),
                empty_map(),
            );
        }
        let Some(canonical_digest) = &descriptor.canonical_digest else {
            return reject(
                ReasonCode::StructuralInvalid,
                Value::String("descriptor missing canonical_digest".into()),
                empty_map(),
            );
        };
        if let Some(result) =
            validate_sha256_digest(canonical_digest, "descriptor canonical_digest")
        {
            return result;
        }
        if let Some(created_at) = &descriptor.created_at {
            if let Some(result) = validate_timestamp_shape(created_at, "descriptor created_at") {
                return result;
            }
        }
        if let Some(producer) = &descriptor.producer {
            if let Some(result) = validate_identity_ref(Some(producer), "descriptor producer") {
                return result;
            }
        }
        if let Some(result) = validate_optional_object_ref(
            descriptor.describes_object.as_ref(),
            "descriptor describes_object",
        ) {
            return result;
        }
        if let Some(result) =
            validate_optional_object_ref(descriptor.object_metadata.as_ref(), "descriptor metadata")
        {
            return result;
        }

        if let Some(bytes) = logical_bytes {
            let derived =
                crate::crypto::derive_object_id(descriptor.canonicalization_id.as_bytes(), bytes);
            if descriptor.object_id != derived {
                return reject(
                    ReasonCode::ObjectIdMismatch,
                    Value::String("descriptor object_id does not match logical bytes".into()),
                    empty_map(),
                );
            }
            if let Some(canonical_digest) = &descriptor.canonical_digest {
                if canonical_digest.value != crate::crypto::sha256(bytes) {
                    return reject(
                        ReasonCode::ObjectIdMismatch,
                        Value::String(
                            "descriptor canonical_digest does not match logical bytes".into(),
                        ),
                        empty_map(),
                    );
                }
            }
            if descriptor.canonical_size != bytes.len() as u64 {
                return reject(
                    ReasonCode::ObjectIdMismatch,
                    Value::String("descriptor canonical_size does not match logical bytes".into()),
                    empty_map(),
                );
            }
        }
    }

    if let Some(header) = header {
        if header.header_version != 1 {
            return reject(
                ReasonCode::VersionUnsupported,
                Value::String("unsupported representation header version".into()),
                empty_map(),
            );
        }
        if header.representation_id.is_empty() {
            return reject(
                ReasonCode::RepresentationInvalid,
                Value::String("representation_id is empty".into()),
                empty_map(),
            );
        }
        let Some(object_ref) = &header.object else {
            return reject(
                ReasonCode::StructuralInvalid,
                Value::String("representation header missing object ref".into()),
                empty_map(),
            );
        };
        if let Some(result) =
            validate_required_object_ref(object_ref, "representation header object")
        {
            return result;
        }
        let Some(representation_digest) = &header.representation_digest else {
            return reject(
                ReasonCode::RepresentationInvalid,
                Value::String("representation header missing digest".into()),
                empty_map(),
            );
        };
        if representation_digest.algorithm
            != edgerun_proto::edgerun::v0::common::digest::Algorithm::DigestAlgorithmSha256 as i32
        {
            return reject(
                ReasonCode::RepresentationInvalid,
                Value::String("representation digest algorithm is not SHA-256".into()),
                empty_map(),
            );
        }
        if representation_digest.value.len() != 32 {
            return reject(
                ReasonCode::RepresentationInvalid,
                Value::String("representation digest must be 32 bytes".into()),
                empty_map(),
            );
        }
        if edgerun_proto::edgerun::v0::object::ChunkingMode::from_i32(header.chunking_mode)
            .is_none_or(|mode| {
                mode == edgerun_proto::edgerun::v0::object::ChunkingMode::Unspecified
            })
        {
            return reject(
                ReasonCode::RepresentationInvalid,
                Value::String("representation chunking_mode is invalid".into()),
                empty_map(),
            );
        }
        if let Some(descriptor) = descriptor {
            if object_ref.object_id != descriptor.object_id {
                return reject(
                    ReasonCode::ObjectIdMismatch,
                    Value::String("representation header object does not match descriptor".into()),
                    empty_map(),
                );
            }
        }
        if header.stored_size == 0 {
            return reject(
                ReasonCode::RepresentationInvalid,
                Value::String("representation stored_size is zero".into()),
                empty_map(),
            );
        }
        if let Some(created_at) = &header.created_at {
            if let Some(result) = validate_timestamp_shape(created_at, "representation created_at")
            {
                return result;
            }
        }
        if let Some(result) = validate_optional_object_ref(
            header.chunk_manifest_object.as_ref(),
            "representation chunk_manifest_object",
        ) {
            return result;
        }
        if let Some(result) = validate_optional_object_ref(
            header.access_package_object.as_ref(),
            "representation access_package_object",
        ) {
            return result;
        }
        if let Some(result) = validate_optional_object_ref(
            header.representation_metadata.as_ref(),
            "representation metadata",
        ) {
            return result;
        }

        if let Some(bytes) = stored_bytes {
            if header.stored_size != bytes.len() as u64 {
                return reject(
                    ReasonCode::RepresentationInvalid,
                    Value::String("representation stored_size does not match stored bytes".into()),
                    empty_map(),
                );
            }
            if let Some(digest) = &header.representation_digest {
                if digest.value != crate::crypto::derive_representation_digest(bytes) {
                    return reject(
                        ReasonCode::RepresentationInvalid,
                        Value::String("representation digest does not match stored bytes".into()),
                        empty_map(),
                    );
                }
            }
        }

        if header.chunking_mode == edgerun_proto::edgerun::v0::object::ChunkingMode::Manifest as i32
            && manifest.is_none()
        {
            return defer(
                ReasonCode::MissingDependency,
                Value::String("chunk manifest not available".into()),
            );
        }
    }

    if let Some(manifest) = manifest {
        if manifest.manifest_version != 1 {
            return reject(
                ReasonCode::VersionUnsupported,
                Value::String("unsupported chunk manifest version".into()),
                empty_map(),
            );
        }
        let Some(manifest_object) = &manifest.object else {
            return reject(
                ReasonCode::StructuralInvalid,
                Value::String("chunk manifest missing object ref".into()),
                empty_map(),
            );
        };
        if let Some(result) = validate_required_object_ref(manifest_object, "chunk manifest object")
        {
            return result;
        }
        if let Some(descriptor) = descriptor {
            if manifest_object.object_id != descriptor.object_id {
                return reject(
                    ReasonCode::ObjectIdMismatch,
                    Value::String("chunk manifest object does not match descriptor".into()),
                    empty_map(),
                );
            }
        }
        if let Some(representation) = &manifest.representation {
            if representation.representation_id.is_empty() {
                return reject(
                    ReasonCode::RepresentationInvalid,
                    Value::String("manifest representation_id is empty".into()),
                    empty_map(),
                );
            }
            if representation.object_id.is_empty() {
                return reject(
                    ReasonCode::RepresentationInvalid,
                    Value::String("manifest representation object_id is empty".into()),
                    empty_map(),
                );
            }
        }
        if let Some(result) =
            validate_optional_object_ref(manifest.manifest_metadata.as_ref(), "manifest metadata")
        {
            return result;
        }

        if manifest.chunk_count == 0 || manifest.chunk_entries.is_empty() {
            return reject(
                ReasonCode::RepresentationInvalid,
                Value::String("chunk manifest has no chunk entries".into()),
                empty_map(),
            );
        }
        if manifest.chunk_count != manifest.chunk_entries.len() as u32 {
            return reject(
                ReasonCode::RepresentationInvalid,
                Value::String("chunk_count does not match entries".into()),
                empty_map(),
            );
        }
        let mut total_size = 0u64;
        for entry in &manifest.chunk_entries {
            let Some(next_total) = total_size.checked_add(entry.length) else {
                return reject(
                    ReasonCode::RepresentationInvalid,
                    Value::String("manifest chunk sizes overflow".into()),
                    empty_map(),
                );
            };
            total_size = next_total;
        }
        if manifest.total_stored_size != total_size {
            return reject(
                ReasonCode::RepresentationInvalid,
                Value::String("manifest total_stored_size does not match entries".into()),
                empty_map(),
            );
        }
        let mut expected_offset = 0u64;
        for entry in &manifest.chunk_entries {
            let Some(digest) = &entry.chunk_digest else {
                return reject(
                    ReasonCode::RepresentationInvalid,
                    Value::String("chunk entry missing digest".into()),
                    empty_map(),
                );
            };
            if digest.algorithm
                != edgerun_proto::edgerun::v0::common::digest::Algorithm::DigestAlgorithmSha256
                    as i32
            {
                return reject(
                    ReasonCode::RepresentationInvalid,
                    Value::String("chunk entry digest algorithm is not SHA-256".into()),
                    empty_map(),
                );
            }
            if digest.value.len() != 32 {
                return reject(
                    ReasonCode::RepresentationInvalid,
                    Value::String("chunk entry digest must be 32 bytes".into()),
                    empty_map(),
                );
            }
            if entry.chunk_representation_id.is_empty() {
                return reject(
                    ReasonCode::RepresentationInvalid,
                    Value::String("chunk entry representation_id is empty".into()),
                    empty_map(),
                );
            }
            if entry.index as usize >= manifest.chunk_entries.len()
                || manifest.chunk_entries[entry.index as usize].index != entry.index
            {
                return reject(
                    ReasonCode::RepresentationInvalid,
                    Value::String("chunk entry indexes are not contiguous".into()),
                    empty_map(),
                );
            }
            if entry.length == 0 {
                return reject(
                    ReasonCode::RepresentationInvalid,
                    Value::String("chunk entry length is zero".into()),
                    empty_map(),
                );
            }
            if entry.offset != expected_offset {
                return reject(
                    ReasonCode::RepresentationInvalid,
                    Value::String(
                        "chunk entry offsets do not preserve reconstruction order".into(),
                    ),
                    empty_map(),
                );
            }
            expected_offset += entry.length;
        }

        if let Some(header) = header {
            if let Some(representation) = &manifest.representation {
                if representation.representation_id != header.representation_id {
                    return reject(
                        ReasonCode::RepresentationInvalid,
                        Value::String(
                            "manifest representation does not match representation header".into(),
                        ),
                        empty_map(),
                    );
                }
                if representation.object_id
                    != header
                        .object
                        .as_ref()
                        .map(|object| object.object_id.clone())
                        .unwrap_or_default()
                {
                    return reject(
                        ReasonCode::ObjectIdMismatch,
                        Value::String(
                            "manifest representation object does not match representation header"
                                .into(),
                        ),
                        empty_map(),
                    );
                }
            }
            if manifest.total_stored_size != header.stored_size {
                return reject(
                    ReasonCode::RepresentationInvalid,
                    Value::String(
                        "manifest total size does not match representation header".into(),
                    ),
                    empty_map(),
                );
            }
        }
    }

    let mut derived = std::collections::BTreeMap::new();
    if let Some(descriptor) = descriptor {
        derived.insert(
            "object_id".into(),
            Value::String(crate::util::bytes_to_hex(&descriptor.object_id)),
        );
    }
    derived.insert(
        "validation_level".into(),
        Value::String("proto_object_retrieval_valid".into()),
    );
    accept(Value::Map(derived), empty_map())
}

// ---------------------------------------------------------------------------
// Query request validation (protobuf-native)
// ---------------------------------------------------------------------------

/// Verifies a signed `QueryRequest` before a responder executes it.
pub fn validate_query_request_signature(query: &QueryRequest) -> ValidationResult {
    if query.request_version != 1 {
        return reject(
            ReasonCode::VersionUnsupported,
            Value::String("unsupported QueryRequest version".into()),
            empty_map(),
        );
    }
    if query.query_id.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("query_id is empty".into()),
            empty_map(),
        );
    }
    let Some(query_class) =
        edgerun_proto::edgerun::v0::access::QueryClass::from_i32(query.query_class)
    else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("QueryRequest has invalid query_class".into()),
            empty_map(),
        );
    };
    if query_class == edgerun_proto::edgerun::v0::access::QueryClass::Unspecified {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("QueryRequest query_class is unspecified".into()),
            empty_map(),
        );
    }
    let Some(scope) = &query.target_scope else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("QueryRequest missing target_scope".into()),
            empty_map(),
        );
    };
    if let Some(result) = validate_scope_descriptor(scope, "QueryRequest target_scope") {
        return result;
    }
    if query.result_limit == Some(0) {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("QueryRequest result_limit must be positive".into()),
            empty_map(),
        );
    }
    if let Some(time_window) = &query.time_window {
        if let (Some(not_before), Some(expires_at)) =
            (&time_window.not_before, &time_window.expires_at)
        {
            if let Some(result) = validate_timestamp_shape(not_before, "QueryRequest not_before") {
                return result;
            }
            if let Some(result) = validate_timestamp_shape(expires_at, "QueryRequest expires_at") {
                return result;
            }
            if timestamp_ms(not_before) > timestamp_ms(expires_at) {
                return reject(
                    ReasonCode::TimeInvalid,
                    Value::String("QueryRequest time_window is inverted".into()),
                    empty_map(),
                );
            }
        } else {
            if let Some(not_before) = &time_window.not_before {
                if let Some(result) =
                    validate_timestamp_shape(not_before, "QueryRequest not_before")
                {
                    return result;
                }
            }
            if let Some(expires_at) = &time_window.expires_at {
                if let Some(result) =
                    validate_timestamp_shape(expires_at, "QueryRequest expires_at")
                {
                    return result;
                }
            }
        }
    }
    if let Some(checkpoint_base) = &query.checkpoint_base {
        if let Some(result) =
            validate_checkpoint_ref(checkpoint_base, "QueryRequest checkpoint_base")
        {
            return result;
        }
    }
    if let Some(result) =
        validate_optional_object_ref(query.query_payload_object.as_ref(), "query_payload_object")
    {
        return result;
    }
    if let Some(cost_limit) = &query.cost_limit {
        if cost_limit.max_results == Some(0)
            || cost_limit.max_total_bytes == Some(0)
            || cost_limit.max_federated_responders == Some(0)
        {
            return reject(
                ReasonCode::StructuralInvalid,
                Value::String("QueryRequest cost_limit values must be positive".into()),
                empty_map(),
            );
        }
        if let Some(duration) = cost_limit.max_wall_time.as_ref() {
            if !is_valid_non_negative_duration(duration)
                || (duration.seconds == 0 && duration.nanos == 0)
            {
                return reject(
                    ReasonCode::TimeInvalid,
                    Value::String("QueryRequest max_wall_time must be positive".into()),
                    empty_map(),
                );
            }
        }
    }
    for proof_class in &query.required_proof_classes {
        if edgerun_proto::edgerun::v0::access::ProofClass::from_i32(*proof_class).is_none_or(
            |class| class == edgerun_proto::edgerun::v0::access::ProofClass::Unspecified,
        ) {
            return reject(
                ReasonCode::StructuralInvalid,
                Value::String("QueryRequest has invalid required_proof_class".into()),
                empty_map(),
            );
        }
    }

    let Some(sig) = &query.signature else {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("query missing signature".into()),
            empty_map(),
        );
    };
    if sig.algorithm != crate::crypto::SIGNATURE_ALGORITHM_ECDSA_P256 as i32 {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("unsupported query signature algorithm".into()),
            empty_map(),
        );
    }
    if sig.value.len() != crate::crypto::ECDSA_P256_SIGNATURE_LEN {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("query signature has invalid length".into()),
            empty_map(),
        );
    }

    let Some(requester) = &query.requester else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("query missing requester".into()),
            empty_map(),
        );
    };
    if let Some(result) = validate_identity_ref(Some(requester), "query requester") {
        return result;
    }
    let Some(key_hint) = &requester.key_hint else {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("query requester missing key_hint".into()),
            empty_map(),
        );
    };
    if key_hint.len() != crate::crypto::ECDSA_P256_PUBLIC_KEY_LEN {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("query requester key_hint has invalid length".into()),
            empty_map(),
        );
    }
    let Ok(key_hint): Result<[u8; 64], _> = key_hint.as_slice().try_into() else {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("query requester key_hint has invalid length".into()),
            empty_map(),
        );
    };
    let Some(vk) = crate::crypto::node_id_to_verifying_key(&key_hint) else {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("query requester key_hint is not a valid public key".into()),
            empty_map(),
        );
    };

    let canonical = canonical_bytes(&ProtocolRecord::QueryRequest(query.clone()), true);
    if !crate::crypto::verify_canonical_record(
        &vk,
        crate::crypto::SIG_DOMAIN_QUERY_REQUEST,
        &canonical,
        &sig.value,
    ) {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("query signature verification failed".into()),
            empty_map(),
        );
    }

    let mut derived = std::collections::BTreeMap::new();
    derived.insert(
        "query_id".into(),
        Value::String(crate::util::bytes_to_hex(&query.query_id)),
    );
    accept(Value::Map(derived), empty_map())
}

/// Validates a signed responder-local `QueryResultFragment`.
///
/// Accepted fragments remain advisory; callers must still verify referenced
/// objects, events, snapshots, and proof objects before treating their contents
/// as authoritative.
pub fn validate_query_result_fragment(
    fragment: &QueryResultFragment,
    expected_query_id: Option<&[u8]>,
    trusted_responders: &[Vec<u8>],
) -> ValidationResult {
    if fragment.fragment_version != 1 {
        return reject(
            ReasonCode::VersionUnsupported,
            Value::String("unsupported QueryResultFragment version".into()),
            empty_map(),
        );
    }
    if fragment.query_id.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("QueryResultFragment query_id is empty".into()),
            empty_map(),
        );
    }
    if let Some(expected_query_id) = expected_query_id {
        if fragment.query_id.as_slice() != expected_query_id {
            return reject(
                ReasonCode::TargetMismatch,
                Value::String("QueryResultFragment query_id mismatch".into()),
                empty_map(),
            );
        }
    }

    let Some(responder) = &fragment.responder else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("QueryResultFragment missing responder".into()),
            empty_map(),
        );
    };
    if let Some(result) = validate_identity_ref(Some(responder), "QueryResultFragment responder") {
        return result;
    }
    if !trusted_responders.is_empty()
        && !trusted_responders
            .iter()
            .any(|id| id.as_slice() == responder.identity_id.as_slice())
    {
        return reject(
            ReasonCode::AuthorityDenied,
            Value::String("QueryResultFragment responder is not trusted".into()),
            empty_map(),
        );
    }
    let Some(answered_at) = &fragment.answered_at else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("QueryResultFragment missing answered_at".into()),
            empty_map(),
        );
    };
    if let Some(result) = validate_timestamp_shape(answered_at, "QueryResultFragment answered_at") {
        return result;
    }

    let Some(completeness) =
        edgerun_proto::edgerun::v0::access::ResultCompleteness::from_i32(fragment.completeness)
    else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("QueryResultFragment has invalid completeness".into()),
            empty_map(),
        );
    };
    if completeness == edgerun_proto::edgerun::v0::access::ResultCompleteness::Unspecified {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("QueryResultFragment completeness is unspecified".into()),
            empty_map(),
        );
    }

    let has_backing = !fragment.snapshot_refs.is_empty()
        || !fragment.event_refs.is_empty()
        || !fragment.object_refs.is_empty()
        || !fragment.proof_objects.is_empty()
        || fragment.bundled_result_object.is_some();
    if completeness == edgerun_proto::edgerun::v0::access::ResultCompleteness::Denied {
        if fragment.omission_reason.is_empty() {
            return reject(
                ReasonCode::StructuralInvalid,
                Value::String("denied QueryResultFragment missing omission_reason".into()),
                empty_map(),
            );
        }
    } else if completeness == edgerun_proto::edgerun::v0::access::ResultCompleteness::MetadataOnly {
        if fragment.result_metadata.is_none() {
            return reject(
                ReasonCode::StructuralInvalid,
                Value::String("metadata-only QueryResultFragment missing result_metadata".into()),
                empty_map(),
            );
        }
    } else if !has_backing {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("QueryResultFragment has no backing references".into()),
            empty_map(),
        );
    }
    for snapshot_ref in &fragment.snapshot_refs {
        if let Some(result) =
            validate_snapshot_ref(snapshot_ref, "QueryResultFragment snapshot_ref")
        {
            return result;
        }
    }
    for event_ref in &fragment.event_refs {
        if let Some(result) = validate_event_ref(event_ref, "QueryResultFragment event_ref") {
            return result;
        }
    }
    for object_ref in &fragment.object_refs {
        if let Some(result) =
            validate_required_object_ref(object_ref, "QueryResultFragment object_ref")
        {
            return result;
        }
    }
    for proof_object in &fragment.proof_objects {
        if let Some(result) =
            validate_required_object_ref(proof_object, "QueryResultFragment proof_object")
        {
            return result;
        }
    }
    if let Some(result) = validate_optional_object_ref(
        fragment.bundled_result_object.as_ref(),
        "bundled_result_object",
    ) {
        return result;
    }
    if let Some(result) =
        validate_optional_object_ref(fragment.result_metadata.as_ref(), "result_metadata")
    {
        return result;
    }

    if !verify_record_signature(
        &ProtocolRecord::QueryResultFragment(fragment.clone()),
        &fragment.signature,
        responder,
        crate::crypto::SIG_DOMAIN_QUERY_RESULT_FRAGMENT,
    ) {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("QueryResultFragment signature verification failed".into()),
            empty_map(),
        );
    }

    let mut derived = std::collections::BTreeMap::new();
    derived.insert(
        "query_id".into(),
        Value::String(crate::util::bytes_to_hex(&fragment.query_id)),
    );
    derived.insert("advisory_only".into(), Value::Bool(true));
    accept(Value::Map(derived), empty_map())
}

// ---------------------------------------------------------------------------
// Control change validation (protobuf-native)
// ---------------------------------------------------------------------------

/// Validates a controller-set mutation command against current state.
///
/// The v0 `CommandEnvelope` does not define a typed control-change payload.
/// The runtime convention is that control commands carry the target controller
/// identity in `command_id`, falling back to the issuer identity for legacy
/// callers. This validator mirrors that convention and simulates post-state
/// before the node commits the control change to its stream.
pub fn validate_control_change_command(
    command: &CommandEnvelope,
    current_controllers: &[Vec<u8>],
    minimum_controllers: usize,
) -> ValidationResult {
    let Some(command_type) = CommandType::from_i32(command.command_type) else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("unknown command_type".into()),
            empty_map(),
        );
    };

    if !matches!(
        command_type,
        CommandType::AddController | CommandType::RemoveController | CommandType::TransferControl
    ) {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("not a control-change command".into()),
            empty_map(),
        );
    }

    let Some(issuer) = &command.issuer else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("control command missing issuer".into()),
            empty_map(),
        );
    };
    if let Some(result) = validate_identity_ref(Some(issuer), "control command issuer") {
        return result;
    }

    if !current_controllers.is_empty()
        && !current_controllers
            .iter()
            .any(|id| id.as_slice() == issuer.identity_id.as_slice())
    {
        return reject(
            ReasonCode::AuthorityDenied,
            Value::String("issuer is not a current controller".into()),
            empty_map(),
        );
    }

    let target_controller = control_target_identity(command);
    if target_controller.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("control command missing target controller identity".into()),
            empty_map(),
        );
    }

    let mut post_controllers = current_controllers.to_vec();
    match command_type {
        CommandType::AddController | CommandType::TransferControl => {
            if !post_controllers
                .iter()
                .any(|id| id.as_slice() == target_controller.as_slice())
            {
                post_controllers.push(target_controller.clone());
            }
        }
        CommandType::RemoveController => {
            let Some(index) = post_controllers
                .iter()
                .position(|id| id.as_slice() == target_controller.as_slice())
            else {
                return reject(
                    ReasonCode::ControlInvariantFailed,
                    Value::String("target controller is not installed".into()),
                    empty_map(),
                );
            };
            post_controllers.remove(index);
            if post_controllers.len() < minimum_controllers {
                return reject(
                    ReasonCode::ControlInvariantFailed,
                    Value::String("controller set would fall below minimum".into()),
                    empty_map(),
                );
            }
        }
        _ => unreachable!("control command types matched above"),
    }

    let mut derived = std::collections::BTreeMap::new();
    derived.insert(
        "target_controller".into(),
        Value::String(crate::util::bytes_to_hex(&target_controller)),
    );
    derived.insert(
        "command_type".into(),
        Value::String(command_type.as_str_name().into()),
    );

    let controller_values = post_controllers
        .iter()
        .map(|id| Value::String(crate::util::bytes_to_hex(id)))
        .collect();
    let mut post_state = std::collections::BTreeMap::new();
    post_state.insert("controller_set".into(), Value::Seq(controller_values));

    accept(Value::Map(derived), Value::Map(post_state))
}

fn control_target_identity(command: &CommandEnvelope) -> Vec<u8> {
    if !command.command_id.is_empty() {
        return command.command_id.clone();
    }
    command
        .issuer
        .as_ref()
        .map(|issuer| issuer.identity_id.clone())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Session handshake validation (protobuf-native)
// ---------------------------------------------------------------------------

/// Validates a signed `SessionHello` before accepting a peer handshake.
pub fn validate_session_hello(
    hello: &SessionHello,
    local_node_id: Option<&[u8]>,
) -> ValidationResult {
    if hello.message_version != 1 {
        return reject(
            ReasonCode::VersionUnsupported,
            Value::String("unsupported SessionHello version".into()),
            empty_map(),
        );
    }
    let Some(initiator) = &hello.initiator else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("SessionHello missing initiator".into()),
            empty_map(),
        );
    };
    if let Some(result) = validate_identity_ref(Some(initiator), "SessionHello initiator") {
        return result;
    }
    if hello.supported_protocol_versions.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("SessionHello missing supported protocol versions".into()),
            empty_map(),
        );
    }
    if hello
        .supported_protocol_versions
        .iter()
        .any(|version| *version == 0)
    {
        return reject(
            ReasonCode::VersionUnsupported,
            Value::String("SessionHello contains unsupported protocol version 0".into()),
            empty_map(),
        );
    }
    if hello
        .supported_transport_features
        .iter()
        .any(|feature| feature.is_empty())
    {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("SessionHello transport feature is empty".into()),
            empty_map(),
        );
    }
    if hello.session_nonce.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("SessionHello missing session nonce".into()),
            empty_map(),
        );
    }
    if let Some(target) = &hello.target_node {
        if let Some(result) = validate_node_ref(Some(target), "SessionHello target_node") {
            return result;
        }
    }
    if let (Some(target), Some(local_node_id)) = (&hello.target_node, local_node_id) {
        if target.node_id.as_slice() != local_node_id {
            return reject(
                ReasonCode::TargetMismatch,
                Value::String("SessionHello targets a different node".into()),
                empty_map(),
            );
        }
    }
    for (index, locator) in hello.initiator_locators.iter().enumerate() {
        if let Some(result) = validate_reachability_hint(
            locator,
            &format!("SessionHello initiator_locators[{index}]"),
        ) {
            return result;
        }
    }
    if let Some(result) =
        validate_optional_object_ref(hello.hello_metadata.as_ref(), "SessionHello metadata")
    {
        return result;
    }

    if !verify_session_signature(
        &ProtocolRecord::SessionHello(hello.clone()),
        &hello.signature,
        initiator,
        crate::crypto::SIG_DOMAIN_SESSION_HELLO,
    ) {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("SessionHello signature verification failed".into()),
            empty_map(),
        );
    }

    let mut derived = std::collections::BTreeMap::new();
    derived.insert(
        "initiator".into(),
        Value::String(crate::util::bytes_to_hex(&initiator.identity_id)),
    );
    accept(Value::Map(derived), empty_map())
}

/// Validates a signed `SessionAccept` for a previously sent hello nonce.
pub fn validate_session_accept(
    accept_msg: &SessionAccept,
    expected_nonce: &[u8],
    supported_protocol_versions: &[u32],
) -> ValidationResult {
    if accept_msg.message_version != 1 {
        return reject(
            ReasonCode::VersionUnsupported,
            Value::String("unsupported SessionAccept version".into()),
            empty_map(),
        );
    }
    let Some(responder) = &accept_msg.responder else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("SessionAccept missing responder".into()),
            empty_map(),
        );
    };
    if let Some(result) = validate_identity_ref(Some(responder), "SessionAccept responder") {
        return result;
    }
    if accept_msg.selected_protocol_version == 0 {
        return reject(
            ReasonCode::VersionUnsupported,
            Value::String("SessionAccept selected protocol version is 0".into()),
            empty_map(),
        );
    }
    if accept_msg.echoed_session_nonce.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("SessionAccept missing echoed session nonce".into()),
            empty_map(),
        );
    }
    if accept_msg.echoed_session_nonce != expected_nonce {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("SessionAccept nonce mismatch".into()),
            empty_map(),
        );
    }
    if !supported_protocol_versions.is_empty()
        && !supported_protocol_versions.contains(&accept_msg.selected_protocol_version)
    {
        return reject(
            ReasonCode::VersionUnsupported,
            Value::String("SessionAccept selected unsupported protocol version".into()),
            empty_map(),
        );
    }
    if accept_msg
        .selected_transport_features
        .iter()
        .any(|feature| feature.is_empty())
    {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("SessionAccept transport feature is empty".into()),
            empty_map(),
        );
    }
    if let Some(result) = validate_optional_object_ref(
        accept_msg.accept_metadata.as_ref(),
        "SessionAccept metadata",
    ) {
        return result;
    }
    for (index, locator) in accept_msg.responder_locators.iter().enumerate() {
        if let Some(result) = validate_reachability_hint(
            locator,
            &format!("SessionAccept responder_locators[{index}]"),
        ) {
            return result;
        }
    }

    if !verify_session_signature(
        &ProtocolRecord::SessionAccept(accept_msg.clone()),
        &accept_msg.signature,
        responder,
        crate::crypto::SIG_DOMAIN_SESSION_ACCEPT,
    ) {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("SessionAccept signature verification failed".into()),
            empty_map(),
        );
    }

    let mut derived = std::collections::BTreeMap::new();
    derived.insert(
        "responder".into(),
        Value::String(crate::util::bytes_to_hex(&responder.identity_id)),
    );
    derived.insert(
        "selected_protocol_version".into(),
        Value::Int(accept_msg.selected_protocol_version as i64),
    );
    accept(Value::Map(derived), empty_map())
}

/// Validates a relay envelope before caching or forwarding it.
///
/// Relay envelopes are transport artifacts. A structurally valid relay envelope
/// remains advisory until its payload family is decoded and validated.
pub fn validate_relay_envelope(
    envelope: &RelayEnvelope,
    local_node_id: Option<&[u8]>,
    now_ms: i64,
) -> ValidationResult {
    if envelope.envelope_version != 1 {
        return reject(
            ReasonCode::VersionUnsupported,
            Value::String("unsupported RelayEnvelope version".into()),
            empty_map(),
        );
    }
    if envelope.relay_message_id.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("RelayEnvelope relay_message_id is empty".into()),
            empty_map(),
        );
    }
    let Some(sender) = &envelope.original_sender else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("RelayEnvelope missing original_sender".into()),
            empty_map(),
        );
    };
    if let Some(result) = validate_identity_ref(Some(sender), "RelayEnvelope original_sender") {
        return result;
    }
    let Some(recipient) = &envelope.intended_recipient_node else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("RelayEnvelope missing intended_recipient_node".into()),
            empty_map(),
        );
    };
    if recipient.node_id.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("RelayEnvelope intended recipient node_id is empty".into()),
            empty_map(),
        );
    }
    if let Some(local_node_id) = local_node_id {
        if recipient.node_id.as_slice() != local_node_id {
            return reject(
                ReasonCode::TargetMismatch,
                Value::String("RelayEnvelope targets a different node".into()),
                empty_map(),
            );
        }
    }

    if edgerun_proto::edgerun::v0::network::PayloadKind::from_i32(envelope.payload_kind)
        .is_none_or(|kind| kind == edgerun_proto::edgerun::v0::network::PayloadKind::Unspecified)
    {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("RelayEnvelope payload_kind is invalid".into()),
            empty_map(),
        );
    }

    let Some(payload) = &envelope.payload else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("RelayEnvelope missing payload".into()),
            empty_map(),
        );
    };
    match payload {
        edgerun_proto::edgerun::v0::network::relay_envelope::Payload::PayloadObject(object) => {
            if let Some(result) =
                validate_required_object_ref(object, "RelayEnvelope payload_object")
            {
                return result;
            }
        }
        edgerun_proto::edgerun::v0::network::relay_envelope::Payload::InlinePayload(bytes) => {
            if bytes.is_empty() {
                return reject(
                    ReasonCode::StructuralInvalid,
                    Value::String("RelayEnvelope inline_payload is empty".into()),
                    empty_map(),
                );
            }
        }
    }

    for (index, relay) in envelope.relay_chain.iter().enumerate() {
        if let Some(result) =
            validate_identity_ref(Some(relay), &format!("RelayEnvelope relay_chain[{index}]"))
        {
            return result;
        }
    }

    if let Some(store_until) = &envelope.store_until {
        if let Some(result) = validate_timestamp_shape(store_until, "RelayEnvelope store_until") {
            return result;
        }
        if now_ms > timestamp_ms(store_until) {
            return reject(
                ReasonCode::TimeInvalid,
                Value::String("RelayEnvelope store_until has expired".into()),
                empty_map(),
            );
        }
    }
    if let Some(result) =
        validate_optional_object_ref(envelope.relay_metadata.as_ref(), "RelayEnvelope metadata")
    {
        return result;
    }

    if envelope.signature.is_some()
        && !verify_record_signature(
            &ProtocolRecord::RelayEnvelope(envelope.clone()),
            &envelope.signature,
            sender,
            crate::crypto::SIG_DOMAIN_RELAY_ENVELOPE,
        )
    {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("RelayEnvelope signature verification failed".into()),
            empty_map(),
        );
    }

    let mut derived = std::collections::BTreeMap::new();
    derived.insert(
        "relay_message_id".into(),
        Value::String(crate::util::bytes_to_hex(&envelope.relay_message_id)),
    );
    derived.insert("advisory_only".into(), Value::Bool(true));
    accept(Value::Map(derived), empty_map())
}

// ---------------------------------------------------------------------------
// Trust record validation (protobuf-native)
// ---------------------------------------------------------------------------

/// Validates a signed `AssuranceClaim` before using it as trust evidence.
pub fn validate_assurance_claim(
    claim: &AssuranceClaim,
    now_ms: i64,
    trusted_attesters: &[Vec<u8>],
) -> ValidationResult {
    if claim.claim_version != 1 {
        return reject(
            ReasonCode::VersionUnsupported,
            Value::String("unsupported AssuranceClaim version".into()),
            empty_map(),
        );
    }
    let Some(subject) = &claim.subject else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("AssuranceClaim missing subject".into()),
            empty_map(),
        );
    };
    match subject {
        edgerun_proto::edgerun::v0::trust::assurance_claim::Subject::SubjectIdentity(identity) => {
            if let Some(result) =
                validate_identity_ref(Some(identity), "AssuranceClaim subject_identity")
            {
                return result;
            }
        }
        edgerun_proto::edgerun::v0::trust::assurance_claim::Subject::SubjectNode(node) => {
            if let Some(result) = validate_node_ref(Some(node), "AssuranceClaim subject_node") {
                return result;
            }
        }
    }
    if edgerun_proto::edgerun::v0::common::AssuranceClass::from_i32(claim.assurance_class)
        .is_none_or(|class| {
            class == edgerun_proto::edgerun::v0::common::AssuranceClass::Unspecified
        })
    {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("AssuranceClaim has invalid assurance class".into()),
            empty_map(),
        );
    }

    let Some(attester) = &claim.attester else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("AssuranceClaim missing attester".into()),
            empty_map(),
        );
    };
    if let Some(result) = validate_identity_ref(Some(attester), "AssuranceClaim attester") {
        return result;
    }
    if !trusted_attesters.is_empty()
        && !trusted_attesters
            .iter()
            .any(|id| id.as_slice() == attester.identity_id.as_slice())
    {
        return reject(
            ReasonCode::AuthorityDenied,
            Value::String("AssuranceClaim attester is not trusted".into()),
            empty_map(),
        );
    }

    let Some(issued_at) = &claim.issued_at else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("AssuranceClaim missing issued_at".into()),
            empty_map(),
        );
    };
    if let Some(result) = validate_timestamp_shape(issued_at, "AssuranceClaim issued_at") {
        return result;
    }
    let issued_ms = timestamp_ms(issued_at);
    if now_ms < issued_ms {
        return defer(
            ReasonCode::TimeInvalid,
            Value::String("AssuranceClaim issued_at is in the future".into()),
        );
    }
    if let Some(expires_at) = &claim.expires_at {
        if let Some(result) = validate_timestamp_shape(expires_at, "AssuranceClaim expires_at") {
            return result;
        }
        if now_ms > timestamp_ms(expires_at) {
            return reject(
                ReasonCode::TimeInvalid,
                Value::String("AssuranceClaim has expired".into()),
                empty_map(),
            );
        }
    }
    if let Some(result) = validate_optional_object_ref(
        claim.evidence_object.as_ref(),
        "AssuranceClaim evidence_object",
    ) {
        return result;
    }

    if !verify_record_signature(
        &ProtocolRecord::AssuranceClaim(claim.clone()),
        &claim.signature,
        attester,
        crate::crypto::SIG_DOMAIN_ASSURANCE_CLAIM,
    ) {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("AssuranceClaim signature verification failed".into()),
            empty_map(),
        );
    }

    let mut derived = std::collections::BTreeMap::new();
    derived.insert(
        "attester".into(),
        Value::String(crate::util::bytes_to_hex(&attester.identity_id)),
    );
    derived.insert(
        "assurance_class".into(),
        Value::Int(claim.assurance_class as i64),
    );
    accept(Value::Map(derived), empty_map())
}

/// Validates that a signed `AssuranceClaim` satisfies an `AssuranceRequirement`.
pub fn validate_assurance_claim_satisfies_requirement(
    claim: &AssuranceClaim,
    requirement: &AssuranceRequirement,
    now_ms: i64,
) -> ValidationResult {
    if requirement.assurance_version != 1 {
        return reject(
            ReasonCode::VersionUnsupported,
            Value::String("unsupported AssuranceRequirement version".into()),
            empty_map(),
        );
    }

    let Some(required_class) =
        edgerun_proto::edgerun::v0::common::AssuranceClass::from_i32(requirement.required_class)
    else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("AssuranceRequirement has invalid required_class".into()),
            empty_map(),
        );
    };
    if required_class == edgerun_proto::edgerun::v0::common::AssuranceClass::Unspecified {
        return accept(empty_map(), empty_map());
    }

    for (index, attester) in requirement.acceptable_attesters.iter().enumerate() {
        if let Some(result) = validate_identity_ref(
            Some(attester),
            &format!("AssuranceRequirement acceptable_attesters[{index}]"),
        ) {
            return result;
        }
    }
    if let Some(result) = validate_optional_object_ref(
        requirement.assurance_metadata.as_ref(),
        "AssuranceRequirement metadata",
    ) {
        return result;
    }

    let acceptable_attesters = requirement
        .acceptable_attesters
        .iter()
        .map(|attester| attester.identity_id.clone())
        .collect::<Vec<_>>();
    let claim_result = validate_assurance_claim(claim, now_ms, &acceptable_attesters);
    if claim_result.verdict != crate::result::Verdict::Accept {
        return claim_result;
    }

    if claim.expires_at.is_none() {
        return reject(
            ReasonCode::TimeInvalid,
            Value::String(
                "AssuranceClaim without expires_at cannot satisfy positive requirement".into(),
            ),
            empty_map(),
        );
    }

    if claim.assurance_class < requirement.required_class {
        let mut derived = std::collections::BTreeMap::new();
        derived.insert(
            "required_class".into(),
            Value::Int(requirement.required_class as i64),
        );
        derived.insert(
            "claim_class".into(),
            Value::Int(claim.assurance_class as i64),
        );
        return reject(
            ReasonCode::AssuranceInsufficient,
            Value::Map(derived),
            empty_map(),
        );
    }

    let Some(issued_at) = &claim.issued_at else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("AssuranceClaim missing issued_at".into()),
            empty_map(),
        );
    };
    if let Some(max_age) = &requirement.max_evidence_age {
        if !is_valid_non_negative_duration(max_age) {
            return reject(
                ReasonCode::StructuralInvalid,
                Value::String("AssuranceRequirement max_evidence_age is invalid".into()),
                empty_map(),
            );
        }
        let max_age_ms = max_age.seconds.saturating_mul(1000) + (max_age.nanos as i64) / 1_000_000;
        let age_ms = now_ms.saturating_sub(timestamp_ms(issued_at));
        if age_ms > max_age_ms {
            let mut derived = std::collections::BTreeMap::new();
            derived.insert("age_ms".into(), Value::Int(age_ms));
            derived.insert("max_age_ms".into(), Value::Int(max_age_ms));
            return reject(ReasonCode::TimeInvalid, Value::Map(derived), empty_map());
        }
    }

    let mut derived = std::collections::BTreeMap::new();
    derived.insert(
        "assurance_class".into(),
        Value::Int(claim.assurance_class as i64),
    );
    accept(Value::Map(derived), empty_map())
}

/// Validates a signed `RevocationRecord` before indexing or applying it.
pub fn validate_revocation_record(
    revocation: &RevocationRecord,
    now_ms: i64,
    trusted_issuers: &[Vec<u8>],
) -> ValidationResult {
    if revocation.record_version != 1 {
        return reject(
            ReasonCode::VersionUnsupported,
            Value::String("unsupported RevocationRecord version".into()),
            empty_map(),
        );
    }
    if revocation.revocation_id.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("RevocationRecord revocation_id is empty".into()),
            empty_map(),
        );
    }

    let Some(issuer) = &revocation.issuer else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("RevocationRecord missing issuer".into()),
            empty_map(),
        );
    };
    if let Some(result) = validate_identity_ref(Some(issuer), "RevocationRecord issuer") {
        return result;
    }
    if !trusted_issuers.is_empty()
        && !trusted_issuers
            .iter()
            .any(|id| id.as_slice() == issuer.identity_id.as_slice())
    {
        return reject(
            ReasonCode::AuthorityDenied,
            Value::String("RevocationRecord issuer is not trusted".into()),
            empty_map(),
        );
    }

    let Some(issued_at) = &revocation.issued_at else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("RevocationRecord missing issued_at".into()),
            empty_map(),
        );
    };
    if let Some(result) = validate_timestamp_shape(issued_at, "RevocationRecord issued_at") {
        return result;
    }
    if now_ms < timestamp_ms(issued_at) {
        return defer(
            ReasonCode::TimeInvalid,
            Value::String("RevocationRecord issued_at is in the future".into()),
        );
    }
    if let Some(effective_at) = &revocation.effective_at {
        if let Some(result) =
            validate_timestamp_shape(effective_at, "RevocationRecord effective_at")
        {
            return result;
        }
        if now_ms < timestamp_ms(effective_at) {
            return defer(
                ReasonCode::TimeInvalid,
                Value::String("RevocationRecord is not yet effective".into()),
            );
        }
    }

    let Some(revocation_kind) =
        edgerun_proto::edgerun::v0::trust::RevocationKind::from_i32(revocation.revocation_kind)
    else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("RevocationRecord has invalid revocation kind".into()),
            empty_map(),
        );
    };
    if revocation_kind == edgerun_proto::edgerun::v0::trust::RevocationKind::Unspecified {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("RevocationRecord has invalid revocation kind".into()),
            empty_map(),
        );
    }
    let Some(target) = &revocation.target else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("RevocationRecord missing target".into()),
            empty_map(),
        );
    };
    match target {
        edgerun_proto::edgerun::v0::trust::revocation_record::Target::TargetDelegation(
            delegation,
        ) => {
            if let Some(result) =
                validate_delegation_ref(delegation, "RevocationRecord target_delegation")
            {
                return result;
            }
        }
        edgerun_proto::edgerun::v0::trust::revocation_record::Target::TargetIdentity(identity) => {
            if let Some(result) =
                validate_identity_ref(Some(identity), "RevocationRecord target_identity")
            {
                return result;
            }
        }
        edgerun_proto::edgerun::v0::trust::revocation_record::Target::TargetNode(node) => {
            if let Some(result) = validate_node_ref(Some(node), "RevocationRecord target_node") {
                return result;
            }
        }
        edgerun_proto::edgerun::v0::trust::revocation_record::Target::TargetObject(object) => {
            if let Some(result) =
                validate_required_object_ref(object, "RevocationRecord target_object")
            {
                return result;
            }
        }
    }
    if !revocation_kind_matches_target(revocation_kind, target) {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("RevocationRecord revocation_kind does not match target family".into()),
            empty_map(),
        );
    }
    if revocation.scope_override.is_some()
        && !matches!(
            target,
            edgerun_proto::edgerun::v0::trust::revocation_record::Target::TargetDelegation(_)
        )
    {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("RevocationRecord scope_override requires delegation target".into()),
            empty_map(),
        );
    }
    if let Some(scope_override) = &revocation.scope_override {
        if let Some(result) =
            validate_scope_descriptor(scope_override, "RevocationRecord scope_override")
        {
            return result;
        }
    }
    if let Some(result) = validate_optional_object_ref(
        revocation.revocation_metadata.as_ref(),
        "RevocationRecord revocation_metadata",
    ) {
        return result;
    }

    if !verify_record_signature(
        &ProtocolRecord::RevocationRecord(revocation.clone()),
        &revocation.signature,
        issuer,
        crate::crypto::SIG_DOMAIN_REVOCATION_RECORD,
    ) {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("RevocationRecord signature verification failed".into()),
            empty_map(),
        );
    }

    let mut derived = std::collections::BTreeMap::new();
    derived.insert(
        "revocation_id".into(),
        Value::String(crate::util::bytes_to_hex(&revocation.revocation_id)),
    );
    derived.insert(
        "issuer".into(),
        Value::String(crate::util::bytes_to_hex(&issuer.identity_id)),
    );
    accept(Value::Map(derived), empty_map())
}

fn revocation_kind_matches_target(
    kind: edgerun_proto::edgerun::v0::trust::RevocationKind,
    target: &edgerun_proto::edgerun::v0::trust::revocation_record::Target,
) -> bool {
    match kind {
        edgerun_proto::edgerun::v0::trust::RevocationKind::Delegation => matches!(
            target,
            edgerun_proto::edgerun::v0::trust::revocation_record::Target::TargetDelegation(_)
        ),
        edgerun_proto::edgerun::v0::trust::RevocationKind::ControllerInstallation => matches!(
            target,
            edgerun_proto::edgerun::v0::trust::revocation_record::Target::TargetNode(_)
        ),
        edgerun_proto::edgerun::v0::trust::RevocationKind::IdentityTrust => matches!(
            target,
            edgerun_proto::edgerun::v0::trust::revocation_record::Target::TargetIdentity(_)
        ),
        edgerun_proto::edgerun::v0::trust::RevocationKind::AssuranceClaim
        | edgerun_proto::edgerun::v0::trust::RevocationKind::SnapshotTrust
        | edgerun_proto::edgerun::v0::trust::RevocationKind::RepresentationAccess => matches!(
            target,
            edgerun_proto::edgerun::v0::trust::revocation_record::Target::TargetObject(_)
        ),
        edgerun_proto::edgerun::v0::trust::RevocationKind::Unspecified => false,
    }
}

fn timestamp_ms(timestamp: &prost_types::Timestamp) -> i64 {
    timestamp.seconds * 1000 + (timestamp.nanos as i64) / 1_000_000
}

fn validate_timestamp_shape(
    timestamp: &prost_types::Timestamp,
    label: &str,
) -> Option<ValidationResult> {
    if !(0..1_000_000_000).contains(&timestamp.nanos) {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            Value::String(format!("{label} nanos is out of range")),
            empty_map(),
        ));
    }
    None
}

fn is_valid_non_negative_duration(duration: &prost_types::Duration) -> bool {
    duration.seconds >= 0 && (0..1_000_000_000).contains(&duration.nanos)
}

fn verify_session_signature(
    record: &ProtocolRecord,
    signature: &Option<crate::protocol::Signature>,
    identity: &crate::protocol::IdentityRef,
    signature_domain: &str,
) -> bool {
    verify_record_signature(record, signature, identity, signature_domain)
}

fn verify_record_signature(
    record: &ProtocolRecord,
    signature: &Option<crate::protocol::Signature>,
    identity: &crate::protocol::IdentityRef,
    signature_domain: &str,
) -> bool {
    let Some(sig) = signature else {
        return false;
    };
    if sig.algorithm != crate::crypto::SIGNATURE_ALGORITHM_ECDSA_P256 as i32 {
        return false;
    }
    if sig.value.len() != crate::crypto::ECDSA_P256_SIGNATURE_LEN {
        return false;
    }

    let key_bytes = identity
        .key_hint
        .as_deref()
        .filter(|bytes| bytes.len() == crate::crypto::ECDSA_P256_PUBLIC_KEY_LEN)
        .unwrap_or(&identity.identity_id);
    if key_bytes.len() != crate::crypto::ECDSA_P256_PUBLIC_KEY_LEN {
        return false;
    }
    let Ok(key_hint): Result<[u8; 64], _> = key_bytes.try_into() else {
        return false;
    };
    let Some(vk) = crate::crypto::node_id_to_verifying_key(&key_hint) else {
        return false;
    };

    let canonical = canonical_bytes(record, true);
    crate::crypto::verify_canonical_record(&vk, signature_domain, &canonical, &sig.value)
        || crate::crypto::verify_canonical_record_hw(&vk, signature_domain, &canonical, &sig.value)
}

// ---------------------------------------------------------------------------
// Re-exports for convenience
// ---------------------------------------------------------------------------

pub use crate::command::{
    command_hash, validate_command, validate_command_signature, CommandValidationContext,
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{
        ActionLifecyclePayload, CheckpointRef, ChunkEntry, ChunkManifest, CommandRef,
        CommandResultPayload, CommandSentPayload, CostLimit, Digest, EventEnvelope, EventRef,
        EventType, HeadRef, IdentityRef, NodeGenesisPayload, NodeRef, ObjectRef, RepresentationRef,
        SecretDeletePayload, SecretPutPayload, SnapshotRef, TimeWindow,
    };
    use edgerun_proto::edgerun::v0::stream::{CollectionCreatedPayload, CollectionDeletedPayload};

    fn make_event(seq: u64, prev_hash: Option<crate::protocol::Digest>) -> EventEnvelope {
        EventEnvelope {
            envelope_version: 1,
            stream_id: b"test-stream".to_vec(),
            seq,
            prev_event_hash: prev_hash,
            event_type: if seq == 0 {
                EventType::NodeGenesis as i32
            } else {
                EventType::CommandSent as i32
            },
            event_version: 1,
            recorded_at: Some(prost_types::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            effective_at: None,
            payload_object: None,
            related_events: vec![],
            related_commands: vec![],
            related_objects: vec![],
            related_delegations: vec![],
            related_revocations: vec![],
            event_metadata: None,
            signature: Some(crate::protocol::Signature {
                algorithm: 1,
                value: vec![0; 64],
            }),
        }
    }

    fn make_genesis_event() -> EventEnvelope {
        make_event(0, None)
    }

    fn test_node_id(key: &edgerun_crypto::p256::ecdsa::SigningKey) -> Vec<u8> {
        crate::crypto::verifying_key_to_node_id(key.verifying_key()).to_vec()
    }

    fn sign_delegation(
        delegation: &mut DelegationRecord,
        key: &edgerun_crypto::p256::ecdsa::SigningKey,
    ) {
        delegation.signature = None;
        if let Some(issuer) = &mut delegation.issuer {
            issuer.key_hint = Some(test_node_id(key));
        }
        let canonical =
            canonical_bytes(&ProtocolRecord::DelegationRecord(delegation.clone()), true);
        let sig = crate::crypto::sign_canonical_record(
            key,
            crate::crypto::SIG_DOMAIN_DELEGATION_RECORD,
            &canonical,
        )
        .unwrap();
        delegation.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: sig,
        });
    }

    fn valid_scope_descriptor() -> ScopeDescriptor {
        ScopeDescriptor {
            scope_version: 1,
            scope_kind: edgerun_proto::edgerun::v0::trust::ScopeKind::Node as i32,
            target_nodes: vec![NodeRef {
                node_id: b"node-a".to_vec(),
            }],
            target_streams: vec![],
            target_object_kinds: vec![],
            target_view_types: vec![],
            target_domains: vec![],
            time_bounds: None,
            scope_metadata: None,
        }
    }

    fn valid_capability_descriptor(actions: &[&str]) -> CapabilityDescriptor {
        CapabilityDescriptor {
            capability_version: 1,
            capability_kind: edgerun_proto::edgerun::v0::trust::CapabilityKind::Query as i32,
            actions: actions.iter().map(|action| (*action).into()).collect(),
            scope: Some(valid_scope_descriptor()),
            constraints: None,
            delegation_policy:
                edgerun_proto::edgerun::v0::trust::DelegationPolicy::DelegableWithAttenuation as i32,
            minimum_assurance: None,
            capability_metadata: None,
        }
    }

    fn valid_delegation(issuer: Vec<u8>, recipient: Vec<u8>, actions: &[&str]) -> DelegationRecord {
        DelegationRecord {
            record_version: 1,
            delegation_id: format!(
                "deleg-{}-{}",
                crate::util::bytes_to_hex(&issuer),
                crate::util::bytes_to_hex(&recipient)
            )
            .into_bytes(),
            issuer: Some(IdentityRef {
                identity_id: issuer,
                identity_kind: None,
                key_hint: None,
            }),
            recipient: Some(IdentityRef {
                identity_id: recipient,
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: Some(prost_types::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            not_before: None,
            expires_at: None,
            capability: Some(valid_capability_descriptor(actions)),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        }
    }

    fn sign_snapshot(
        snapshot: &mut SnapshotDescriptor,
        key: &edgerun_crypto::p256::ecdsa::SigningKey,
    ) {
        snapshot.signature = None;
        if let Some(producer) = &mut snapshot.producer {
            producer.key_hint = Some(test_node_id(key));
        }
        let canonical =
            canonical_bytes(&ProtocolRecord::SnapshotDescriptor(snapshot.clone()), true);
        let sig = crate::crypto::sign_canonical_record(
            key,
            crate::crypto::SIG_DOMAIN_SNAPSHOT_DESCRIPTOR,
            &canonical,
        )
        .unwrap();
        snapshot.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: sig,
        });
    }

    fn valid_snapshot_for_tests(producer_id: Vec<u8>) -> SnapshotDescriptor {
        SnapshotDescriptor {
            descriptor_version: 1,
            snapshot_id: b"snap-1".to_vec(),
            view_type: "timeline".into(),
            view_version: 1,
            producer: Some(IdentityRef {
                identity_id: producer_id,
                identity_kind: None,
                key_hint: None,
            }),
            produced_at: Some(prost_types::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            base_heads: vec![crate::protocol::HeadRef {
                stream_id: b"stream-1".to_vec(),
                seq: 10,
                event_hash: Some(Digest {
                    algorithm: 1,
                    value: vec![0xAA; 32],
                }),
            }],
            base_checkpoints: vec![],
            scope: Some(crate::protocol::ScopeDescriptor {
                scope_version: 1,
                scope_kind: edgerun_proto::edgerun::v0::trust::ScopeKind::Stream as i32,
                target_nodes: vec![],
                target_streams: vec![crate::protocol::StreamRef {
                    stream_id: b"stream-1".to_vec(),
                }],
                target_object_kinds: vec![],
                target_view_types: vec!["timeline".into()],
                target_domains: vec![],
                time_bounds: None,
                scope_metadata: None,
            }),
            completeness: edgerun_proto::edgerun::v0::access::SnapshotCompleteness::Full as i32,
            payload_object: Some(crate::protocol::ObjectRef {
                object_id: b"obj-1".to_vec(),
                object_kind: Some(edgerun_proto::edgerun::v0::common::ObjectKind::Snapshot as i32),
            }),
            supersedes: None,
            snapshot_metadata: None,
            signature: None,
        }
    }

    #[test]
    fn stream_append_accepts_genesis() {
        let genesis = make_genesis_event();
        let result = validate_stream_append(&genesis, None, None);
        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn stream_append_rejects_invalid_recorded_at_timestamp() {
        let mut genesis = make_genesis_event();
        genesis.recorded_at = Some(prost_types::Timestamp {
            seconds: 1_700_000_000,
            nanos: 1_000_000_000,
        });

        let result = validate_stream_append(&genesis, None, None);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn stream_append_rejects_invalid_effective_at_timestamp() {
        let mut genesis = make_genesis_event();
        genesis.effective_at = Some(prost_types::Timestamp {
            seconds: 1_700_000_000,
            nanos: -1,
        });

        let result = validate_stream_append(&genesis, None, None);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn stream_append_rejects_genesis_with_prev_hash() {
        let mut genesis = make_genesis_event();
        genesis.prev_event_hash = Some(crate::protocol::Digest {
            algorithm: 1,
            value: vec![1; 32],
        });
        let result = validate_stream_append(&genesis, None, None);
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn stream_append_rejects_non_genesis_type_at_seq_zero() {
        let mut genesis = make_genesis_event();
        genesis.event_type = EventType::CommandSent as i32;

        let result = validate_stream_append(&genesis, None, None);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn stream_append_rejects_node_genesis_after_seq_zero() {
        let genesis = make_genesis_event();
        let mut event = make_event(1, Some(compute_event_hash(&genesis)));
        event.event_type = EventType::NodeGenesis as i32;

        let result = validate_stream_append(&event, Some(&genesis), None);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn stream_append_defers_without_genesis() {
        let event = make_event(
            1,
            Some(crate::protocol::Digest {
                algorithm: 1,
                value: vec![1; 32],
            }),
        );
        let result = validate_stream_append(&event, None, None);
        assert_eq!(result.verdict, crate::result::Verdict::Defer);
    }

    #[test]
    fn stream_append_defers_missing_predecessor() {
        let genesis = make_genesis_event();
        let event = make_event(
            5,
            Some(crate::protocol::Digest {
                algorithm: 1,
                value: vec![1; 32],
            }),
        );
        let result = validate_stream_append(&event, Some(&genesis), None);
        assert_eq!(result.verdict, crate::result::Verdict::Defer);
    }

    #[test]
    fn stream_append_rejects_prev_hash_mismatch() {
        let genesis = make_genesis_event();
        let event = make_event(
            1,
            Some(crate::protocol::Digest {
                algorithm: 1,
                value: vec![0xFF; 32],
            }),
        );
        let result = validate_stream_append(&event, Some(&genesis), None);
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
    }

    #[test]
    fn stream_append_detects_fork() {
        let genesis = make_genesis_event();
        let duplicate_genesis = make_genesis_event();
        let result = validate_stream_append(&duplicate_genesis, Some(&genesis), None);
        assert_eq!(result.verdict, crate::result::Verdict::Duplicate);
        assert_eq!(result.reason_code, Some(ReasonCode::ForkConflict));
    }

    #[test]
    fn stream_append_rejects_empty_related_object_ref() {
        let mut genesis = make_genesis_event();
        genesis.related_objects.push(ObjectRef {
            object_id: vec![],
            object_kind: None,
        });

        let result = validate_stream_append(&genesis, None, None);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn stream_append_rejects_related_delegation_without_hash() {
        let mut genesis = make_genesis_event();
        genesis
            .related_delegations
            .push(crate::protocol::DelegationRef {
                delegation_id: b"delegation-1".to_vec(),
                delegation_hash: None,
            });

        let result = validate_stream_append(&genesis, None, None);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn stream_append_rejects_related_revocation_without_hash() {
        let mut genesis = make_genesis_event();
        genesis
            .related_revocations
            .push(crate::protocol::RevocationRef {
                revocation_id: b"revocation-1".to_vec(),
                revocation_hash: None,
            });

        let result = validate_stream_append(&genesis, None, None);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn stream_append_rejects_event_metadata_empty_object_id() {
        let mut genesis = make_genesis_event();
        genesis.event_metadata = Some(ObjectRef {
            object_id: vec![],
            object_kind: None,
        });

        let result = validate_stream_append(&genesis, None, None);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    fn valid_command_ref() -> CommandRef {
        CommandRef {
            command_id: b"command-1".to_vec(),
            command_hash: Some(Digest {
                algorithm: 1,
                value: vec![7; 32],
            }),
        }
    }

    fn valid_identity_ref(id: &[u8]) -> IdentityRef {
        IdentityRef {
            identity_id: id.to_vec(),
            identity_kind: Some(1),
            key_hint: None,
        }
    }

    fn valid_node_genesis_payload() -> NodeGenesisPayload {
        NodeGenesisPayload {
            payload_version: 1,
            node_id: b"node-1".to_vec(),
            primary_node_identity: Some(valid_identity_ref(b"node-identity")),
            initial_controllers: vec![valid_identity_ref(b"controller-1")],
            initial_policy_object: None,
            bootstrap_records: vec![],
            assurance_claims: vec![],
            node_roles: vec![],
            genesis_metadata: None,
        }
    }

    fn valid_command_sent_payload() -> CommandSentPayload {
        CommandSentPayload {
            payload_version: 1,
            command: Some(valid_command_ref()),
            target_node: Some(NodeRef {
                node_id: b"node-1".to_vec(),
            }),
            send_metadata: None,
        }
    }

    fn valid_command_result_payload() -> CommandResultPayload {
        CommandResultPayload {
            payload_version: 1,
            command: Some(valid_command_ref()),
            issuer: Some(valid_identity_ref(b"issuer-1")),
            decision: edgerun_proto::edgerun::v0::stream::CommandDecision::Committed as i32,
            decision_basis: None,
            reason_code: String::new(),
            effect_summary_object: None,
            result_object: None,
        }
    }

    fn valid_action_lifecycle_payload() -> ActionLifecyclePayload {
        ActionLifecyclePayload {
            payload_version: 1,
            origin_command: Some(valid_command_ref()),
            action_instance_id: b"action-1".to_vec(),
            status: edgerun_proto::edgerun::v0::stream::ActionStatus::Started as i32,
            result_object: None,
            error_object: None,
            progress_object: None,
            action_metadata: None,
        }
    }

    fn valid_secret_put_payload() -> SecretPutPayload {
        SecretPutPayload {
            payload_version: 1,
            namespace: "default".into(),
            key: "api-token".into(),
            label: "API token".into(),
            attributes: std::collections::BTreeMap::new(),
            secret_blob_id: "0123456789abcdef".into(),
        }
    }

    fn valid_secret_delete_payload() -> SecretDeletePayload {
        SecretDeletePayload {
            payload_version: 1,
            namespace: "default".into(),
            key: "api-token".into(),
            label: "API token".into(),
            reason: String::new(),
        }
    }

    fn valid_collection_created_payload() -> CollectionCreatedPayload {
        CollectionCreatedPayload {
            payload_version: 1,
            collection_name: "default".into(),
            label: "Default".into(),
        }
    }

    fn valid_collection_deleted_payload() -> CollectionDeletedPayload {
        CollectionDeletedPayload {
            payload_version: 1,
            collection_name: "default".into(),
            items_removed: 0,
        }
    }

    #[test]
    fn node_genesis_payload_valid_is_accepted() {
        let result = validate_node_genesis_payload(&valid_node_genesis_payload());
        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn node_genesis_payload_missing_primary_identity_is_rejected() {
        let mut payload = valid_node_genesis_payload();
        payload.primary_node_identity = None;

        let result = validate_node_genesis_payload(&payload);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn node_genesis_payload_empty_initial_controllers_is_rejected() {
        let mut payload = valid_node_genesis_payload();
        payload.initial_controllers.clear();

        let result = validate_node_genesis_payload(&payload);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn node_genesis_payload_empty_bootstrap_record_is_rejected() {
        let mut payload = valid_node_genesis_payload();
        payload.bootstrap_records.push(ObjectRef {
            object_id: vec![],
            object_kind: None,
        });

        let result = validate_node_genesis_payload(&payload);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn node_genesis_payload_empty_assurance_claim_is_rejected() {
        let mut payload = valid_node_genesis_payload();
        payload.assurance_claims.push(ObjectRef {
            object_id: vec![],
            object_kind: None,
        });

        let result = validate_node_genesis_payload(&payload);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn node_genesis_payload_empty_role_is_rejected() {
        let mut payload = valid_node_genesis_payload();
        payload.node_roles.push(String::new());

        let result = validate_node_genesis_payload(&payload);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn command_sent_payload_valid_is_accepted() {
        let result = validate_command_sent_payload(&valid_command_sent_payload());
        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn command_sent_payload_missing_command_hash_is_rejected() {
        let mut payload = valid_command_sent_payload();
        payload.command.as_mut().unwrap().command_hash = None;

        let result = validate_command_sent_payload(&payload);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn command_sent_payload_missing_target_node_is_rejected() {
        let mut payload = valid_command_sent_payload();
        payload.target_node = None;

        let result = validate_command_sent_payload(&payload);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn secret_put_payload_valid_is_accepted() {
        let result = validate_secret_put_payload(&valid_secret_put_payload());
        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn secret_put_payload_missing_blob_id_is_rejected() {
        let mut payload = valid_secret_put_payload();
        payload.secret_blob_id.clear();

        let result = validate_secret_put_payload(&payload);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn secret_put_payload_non_hex_blob_id_is_rejected() {
        let mut payload = valid_secret_put_payload();
        payload.secret_blob_id = "blob-1".into();

        let result = validate_secret_put_payload(&payload);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn secret_put_payload_odd_length_blob_id_is_rejected() {
        let mut payload = valid_secret_put_payload();
        payload.secret_blob_id = "abc".into();

        let result = validate_secret_put_payload(&payload);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn secret_delete_payload_empty_key_is_rejected() {
        let mut payload = valid_secret_delete_payload();
        payload.key.clear();

        let result = validate_secret_delete_payload(&payload);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn collection_created_payload_valid_is_accepted() {
        let result = validate_collection_created_payload(&valid_collection_created_payload());
        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn collection_created_payload_empty_name_is_rejected() {
        let mut payload = valid_collection_created_payload();
        payload.collection_name.clear();

        let result = validate_collection_created_payload(&payload);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn collection_deleted_payload_valid_with_zero_items_is_accepted() {
        let result = validate_collection_deleted_payload(&valid_collection_deleted_payload());
        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn collection_deleted_payload_empty_name_is_rejected() {
        let mut payload = valid_collection_deleted_payload();
        payload.collection_name.clear();

        let result = validate_collection_deleted_payload(&payload);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn command_result_payload_valid_is_accepted() {
        let result = validate_command_result_payload(&valid_command_result_payload());
        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn command_result_payload_missing_command_hash_is_rejected() {
        let mut payload = valid_command_result_payload();
        payload.command.as_mut().unwrap().command_hash = None;

        let result = validate_command_result_payload(&payload);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn command_result_payload_invalid_decision_is_rejected() {
        let mut payload = valid_command_result_payload();
        payload.decision = edgerun_proto::edgerun::v0::stream::CommandDecision::Unspecified as i32;

        let result = validate_command_result_payload(&payload);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn action_lifecycle_payload_valid_is_accepted() {
        let result = validate_action_lifecycle_payload(&valid_action_lifecycle_payload());
        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn action_lifecycle_payload_empty_action_id_is_rejected() {
        let mut payload = valid_action_lifecycle_payload();
        payload.action_instance_id.clear();

        let result = validate_action_lifecycle_payload(&payload);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn action_lifecycle_payload_invalid_status_is_rejected() {
        let mut payload = valid_action_lifecycle_payload();
        payload.status = 999_999;

        let result = validate_action_lifecycle_payload(&payload);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn action_lifecycle_payload_completed_requires_result_object() {
        let mut payload = valid_action_lifecycle_payload();
        payload.status = edgerun_proto::edgerun::v0::stream::ActionStatus::Completed as i32;

        let result = validate_action_lifecycle_payload(&payload);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn action_lifecycle_payload_completed_with_result_is_accepted() {
        let mut payload = valid_action_lifecycle_payload();
        payload.status = edgerun_proto::edgerun::v0::stream::ActionStatus::Completed as i32;
        payload.result_object = Some(ObjectRef {
            object_id: b"result-object".to_vec(),
            object_kind: None,
        });

        let result = validate_action_lifecycle_payload(&payload);

        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn action_lifecycle_payload_failed_requires_error_object() {
        let mut payload = valid_action_lifecycle_payload();
        payload.status = edgerun_proto::edgerun::v0::stream::ActionStatus::Failed as i32;

        let result = validate_action_lifecycle_payload(&payload);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn action_lifecycle_payload_failed_with_error_is_accepted() {
        let mut payload = valid_action_lifecycle_payload();
        payload.status = edgerun_proto::edgerun::v0::stream::ActionStatus::Failed as i32;
        payload.error_object = Some(ObjectRef {
            object_id: b"error-object".to_vec(),
            object_kind: None,
        });

        let result = validate_action_lifecycle_payload(&payload);

        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn delegation_chain_valid_single() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut deleg = valid_delegation(b"root".to_vec(), b"user".to_vec(), &["query"]);
        sign_delegation(&mut deleg, &signing_key);

        let result = validate_delegation_chain(
            &[deleg],
            1_700_000_000_000,
            &std::collections::HashSet::new(),
        );
        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn delegation_chain_bad_signature_is_rejected() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut deleg = valid_delegation(b"root".to_vec(), b"user".to_vec(), &["query"]);
        sign_delegation(&mut deleg, &signing_key);
        if let Some(sig) = &mut deleg.signature {
            sig.value[0] ^= 0xFF;
        }

        let result = validate_delegation_chain(
            &[deleg],
            1_700_000_000_000,
            &std::collections::HashSet::new(),
        );
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
    }

    #[test]
    fn delegation_chain_revoked() {
        let deleg = valid_delegation(b"root".to_vec(), b"user".to_vec(), &["query"]);

        let mut revoked = std::collections::HashSet::new();
        revoked.insert(deleg.delegation_id.clone());

        let result = validate_delegation_chain(&[deleg], 1_700_000_000_000, &revoked);
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::RevocationActive));
    }

    #[test]
    fn delegation_chain_broken_continuity() {
        let parent = valid_delegation(b"root".to_vec(), b"mid".to_vec(), &["query"]);

        let child = valid_delegation(b"OTHER".to_vec(), b"user".to_vec(), &["query"]);

        let result = validate_delegation_chain(
            &[parent, child],
            1_700_000_000_000,
            &std::collections::HashSet::new(),
        );
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn delegation_chain_non_delegable_parent_rejects_child() {
        let root_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mid_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[43u8; 32].into()).unwrap();
        let mut parent = valid_delegation(b"root".to_vec(), b"mid".to_vec(), &["query"]);
        parent.capability.as_mut().unwrap().delegation_policy =
            edgerun_proto::edgerun::v0::trust::DelegationPolicy::NonDelegable as i32;
        sign_delegation(&mut parent, &root_key);
        let mut child = valid_delegation(b"mid".to_vec(), b"user".to_vec(), &["query"]);
        sign_delegation(&mut child, &mid_key);

        let result = validate_delegation_chain(
            &[parent, child],
            1_700_000_000_000,
            &std::collections::HashSet::new(),
        );

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn delegation_chain_scope_expansion_is_rejected() {
        let root_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mid_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[43u8; 32].into()).unwrap();
        let mut parent = valid_delegation(b"root".to_vec(), b"mid".to_vec(), &["query"]);
        sign_delegation(&mut parent, &root_key);
        let mut child = valid_delegation(b"mid".to_vec(), b"user".to_vec(), &["query"]);
        child
            .capability
            .as_mut()
            .unwrap()
            .scope
            .as_mut()
            .unwrap()
            .target_nodes = vec![NodeRef {
            node_id: b"node-b".to_vec(),
        }];
        sign_delegation(&mut child, &mid_key);

        let result = validate_delegation_chain(
            &[parent, child],
            1_700_000_000_000,
            &std::collections::HashSet::new(),
        );

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn delegation_missing_capability_is_rejected() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut deleg = valid_delegation(b"root".to_vec(), b"user".to_vec(), &["query"]);
        deleg.capability = None;
        sign_delegation(&mut deleg, &signing_key);

        let result = validate_delegation_chain(
            &[deleg],
            1_700_000_000_000,
            &std::collections::HashSet::new(),
        );

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn delegation_missing_issued_at_is_rejected() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut deleg = valid_delegation(b"root".to_vec(), b"user".to_vec(), &["query"]);
        deleg.issued_at = None;
        sign_delegation(&mut deleg, &signing_key);

        let result = validate_delegation_chain(
            &[deleg],
            1_700_000_000_000,
            &std::collections::HashSet::new(),
        );

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn delegation_invalid_issued_at_timestamp_is_rejected() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut deleg = valid_delegation(b"root".to_vec(), b"user".to_vec(), &["query"]);
        deleg.issued_at = Some(prost_types::Timestamp {
            seconds: 1_700_000_000,
            nanos: 1_000_000_000,
        });
        sign_delegation(&mut deleg, &signing_key);

        let result = validate_delegation_chain(
            &[deleg],
            1_700_000_000_000,
            &std::collections::HashSet::new(),
        );

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn delegation_capability_invalid_policy_is_rejected() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut deleg = valid_delegation(b"root".to_vec(), b"user".to_vec(), &["query"]);
        deleg.capability.as_mut().unwrap().delegation_policy =
            edgerun_proto::edgerun::v0::trust::DelegationPolicy::Unspecified as i32;
        sign_delegation(&mut deleg, &signing_key);

        let result = validate_delegation_chain(
            &[deleg],
            1_700_000_000_000,
            &std::collections::HashSet::new(),
        );

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn delegation_parent_ref_requires_hash() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut deleg = valid_delegation(b"root".to_vec(), b"user".to_vec(), &["query"]);
        deleg.parent_delegation = Some(crate::protocol::DelegationRef {
            delegation_id: b"parent-delegation".to_vec(),
            delegation_hash: None,
        });
        sign_delegation(&mut deleg, &signing_key);

        let result = validate_delegation_chain(
            &[deleg],
            1_700_000_000_000,
            &std::collections::HashSet::new(),
        );

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn delegation_revocation_authority_requires_identity_id() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut deleg = valid_delegation(b"root".to_vec(), b"user".to_vec(), &["query"]);
        deleg.revocation_authorities.push(IdentityRef {
            identity_id: vec![],
            identity_kind: None,
            key_hint: None,
        });
        sign_delegation(&mut deleg, &signing_key);

        let result = validate_delegation_chain(
            &[deleg],
            1_700_000_000_000,
            &std::collections::HashSet::new(),
        );

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn delegation_invalid_identity_kind_is_rejected() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut deleg = valid_delegation(b"root".to_vec(), b"user".to_vec(), &["query"]);
        deleg.issuer.as_mut().unwrap().identity_kind = Some(999_999);
        sign_delegation(&mut deleg, &signing_key);

        let result = validate_delegation_chain(
            &[deleg],
            1_700_000_000_000,
            &std::collections::HashSet::new(),
        );

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn delegation_capability_metadata_requires_object_id() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut deleg = valid_delegation(b"root".to_vec(), b"user".to_vec(), &["query"]);
        deleg.capability.as_mut().unwrap().capability_metadata = Some(ObjectRef {
            object_id: vec![],
            object_kind: None,
        });
        sign_delegation(&mut deleg, &signing_key);

        let result = validate_delegation_chain(
            &[deleg],
            1_700_000_000_000,
            &std::collections::HashSet::new(),
        );

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn delegation_constraint_invalid_version_is_rejected() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut deleg = valid_delegation(b"root".to_vec(), b"user".to_vec(), &["query"]);
        deleg.capability.as_mut().unwrap().constraints = Some(ConstraintSet {
            constraint_version: 0,
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
        });
        sign_delegation(&mut deleg, &signing_key);

        let result = validate_delegation_chain(
            &[deleg],
            1_700_000_000_000,
            &std::collections::HashSet::new(),
        );

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::VersionUnsupported));
    }

    #[test]
    fn delegation_constraint_inverted_time_is_rejected() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut deleg = valid_delegation(b"root".to_vec(), b"user".to_vec(), &["query"]);
        deleg.capability.as_mut().unwrap().constraints = Some(ConstraintSet {
            constraint_version: 1,
            not_before: Some(prost_types::Timestamp {
                seconds: 20,
                nanos: 0,
            }),
            expires_at: Some(prost_types::Timestamp {
                seconds: 10,
                nanos: 0,
            }),
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
        });
        sign_delegation(&mut deleg, &signing_key);

        let result = validate_delegation_chain(
            &[deleg],
            1_700_000_000_000,
            &std::collections::HashSet::new(),
        );

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::TimeInvalid));
    }

    #[test]
    fn delegation_constraint_invalid_time_shape_is_rejected() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut deleg = valid_delegation(b"root".to_vec(), b"user".to_vec(), &["query"]);
        deleg.capability.as_mut().unwrap().constraints = Some(ConstraintSet {
            constraint_version: 1,
            not_before: Some(prost_types::Timestamp {
                seconds: 10,
                nanos: -1,
            }),
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
        });
        sign_delegation(&mut deleg, &signing_key);

        let result = validate_delegation_chain(
            &[deleg],
            1_700_000_000_000,
            &std::collections::HashSet::new(),
        );

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn delegation_constraint_zero_rate_limit_is_rejected() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut deleg = valid_delegation(b"root".to_vec(), b"user".to_vec(), &["query"]);
        deleg.capability.as_mut().unwrap().constraints = Some(ConstraintSet {
            constraint_version: 1,
            not_before: None,
            expires_at: None,
            max_uses: None,
            rate_limit: Some(crate::protocol::RateLimit {
                max_operations: 0,
                per: Some(prost_types::Duration {
                    seconds: 1,
                    nanos: 0,
                }),
            }),
            requires_local_session: None,
            requires_user_presence: None,
            requires_transport_classes: vec![],
            requires_location_classes: vec![],
            export_policy: 0,
            execution_class_limits: vec![],
            storage_class_limits: vec![],
            constraint_metadata: None,
        });
        sign_delegation(&mut deleg, &signing_key);

        let result = validate_delegation_chain(
            &[deleg],
            1_700_000_000_000,
            &std::collections::HashSet::new(),
        );

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn delegation_constraint_invalid_rate_limit_duration_is_rejected() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut deleg = valid_delegation(b"root".to_vec(), b"user".to_vec(), &["query"]);
        deleg.capability.as_mut().unwrap().constraints = Some(ConstraintSet {
            constraint_version: 1,
            not_before: None,
            expires_at: None,
            max_uses: None,
            rate_limit: Some(crate::protocol::RateLimit {
                max_operations: 1,
                per: Some(prost_types::Duration {
                    seconds: 1,
                    nanos: 1_000_000_000,
                }),
            }),
            requires_local_session: None,
            requires_user_presence: None,
            requires_transport_classes: vec![],
            requires_location_classes: vec![],
            export_policy: 0,
            execution_class_limits: vec![],
            storage_class_limits: vec![],
            constraint_metadata: None,
        });
        sign_delegation(&mut deleg, &signing_key);

        let result = validate_delegation_chain(
            &[deleg],
            1_700_000_000_000,
            &std::collections::HashSet::new(),
        );

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::TimeInvalid));
    }

    #[test]
    fn delegation_constraint_invalid_transport_is_rejected() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut deleg = valid_delegation(b"root".to_vec(), b"user".to_vec(), &["query"]);
        deleg.capability.as_mut().unwrap().constraints = Some(ConstraintSet {
            constraint_version: 1,
            not_before: None,
            expires_at: None,
            max_uses: None,
            rate_limit: None,
            requires_local_session: None,
            requires_user_presence: None,
            requires_transport_classes: vec![
                edgerun_proto::edgerun::v0::common::TransportClass::Unspecified as i32,
            ],
            requires_location_classes: vec![],
            export_policy: 0,
            execution_class_limits: vec![],
            storage_class_limits: vec![],
            constraint_metadata: None,
        });
        sign_delegation(&mut deleg, &signing_key);

        let result = validate_delegation_chain(
            &[deleg],
            1_700_000_000_000,
            &std::collections::HashSet::new(),
        );

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn delegation_constraint_metadata_requires_object_id() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut deleg = valid_delegation(b"root".to_vec(), b"user".to_vec(), &["query"]);
        deleg.capability.as_mut().unwrap().constraints = Some(ConstraintSet {
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
            constraint_metadata: Some(ObjectRef {
                object_id: vec![],
                object_kind: None,
            }),
        });
        sign_delegation(&mut deleg, &signing_key);

        let result = validate_delegation_chain(
            &[deleg],
            1_700_000_000_000,
            &std::collections::HashSet::new(),
        );

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn delegation_metadata_requires_object_id() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut deleg = valid_delegation(b"root".to_vec(), b"user".to_vec(), &["query"]);
        deleg.delegation_metadata = Some(ObjectRef {
            object_id: vec![],
            object_kind: None,
        });
        sign_delegation(&mut deleg, &signing_key);

        let result = validate_delegation_chain(
            &[deleg],
            1_700_000_000_000,
            &std::collections::HashSet::new(),
        );

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn snapshot_valid_with_trusted_producer() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut snapshot = valid_snapshot_for_tests(b"trusted".to_vec());
        sign_snapshot(&mut snapshot, &signing_key);

        let result = validate_snapshot(&snapshot, &[b"trusted".to_vec()]);
        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn snapshot_bad_signature_rejected() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut snapshot = valid_snapshot_for_tests(b"trusted".to_vec());
        sign_snapshot(&mut snapshot, &signing_key);
        if let Some(sig) = &mut snapshot.signature {
            sig.value[0] ^= 0xFF;
        }

        let result = validate_snapshot(&snapshot, &[b"trusted".to_vec()]);
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
    }

    #[test]
    fn snapshot_untrusted_producer_rejected() {
        let snapshot = valid_snapshot_for_tests(b"untrusted".to_vec());

        let result = validate_snapshot(&snapshot, &[b"trusted".to_vec()]);
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn snapshot_missing_scope_rejected() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut snapshot = valid_snapshot_for_tests(b"trusted".to_vec());
        snapshot.scope = None;
        sign_snapshot(&mut snapshot, &signing_key);

        let result = validate_snapshot(&snapshot, &[b"trusted".to_vec()]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn snapshot_base_head_missing_event_hash_rejected() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut snapshot = valid_snapshot_for_tests(b"trusted".to_vec());
        snapshot.base_heads[0].event_hash = None;
        sign_snapshot(&mut snapshot, &signing_key);

        let result = validate_snapshot(&snapshot, &[b"trusted".to_vec()]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn snapshot_base_checkpoint_without_id_or_heads_rejected() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut snapshot = valid_snapshot_for_tests(b"trusted".to_vec());
        snapshot.base_heads.clear();
        snapshot
            .base_checkpoints
            .push(crate::protocol::CheckpointRef {
                checkpoint_id: None,
                heads: vec![],
            });
        sign_snapshot(&mut snapshot, &signing_key);

        let result = validate_snapshot(&snapshot, &[b"trusted".to_vec()]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn snapshot_missing_payload_object_rejected() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut snapshot = valid_snapshot_for_tests(b"trusted".to_vec());
        snapshot.payload_object = None;
        sign_snapshot(&mut snapshot, &signing_key);

        let result = validate_snapshot(&snapshot, &[b"trusted".to_vec()]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn snapshot_empty_payload_object_rejected() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut snapshot = valid_snapshot_for_tests(b"trusted".to_vec());
        snapshot.payload_object = Some(ObjectRef {
            object_id: vec![],
            object_kind: None,
        });
        sign_snapshot(&mut snapshot, &signing_key);

        let result = validate_snapshot(&snapshot, &[b"trusted".to_vec()]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn snapshot_invalid_payload_object_kind_rejected() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut snapshot = valid_snapshot_for_tests(b"trusted".to_vec());
        snapshot.payload_object = Some(ObjectRef {
            object_id: vec![0x44; 32],
            object_kind: Some(edgerun_proto::edgerun::v0::common::ObjectKind::Unspecified as i32),
        });
        sign_snapshot(&mut snapshot, &signing_key);

        let result = validate_snapshot(&snapshot, &[b"trusted".to_vec()]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn snapshot_supersedes_missing_snapshot_id_rejected() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut snapshot = valid_snapshot_for_tests(b"trusted".to_vec());
        snapshot.supersedes = Some(crate::protocol::SnapshotRef {
            snapshot_id: vec![],
            object_id: None,
        });
        sign_snapshot(&mut snapshot, &signing_key);

        let result = validate_snapshot(&snapshot, &[b"trusted".to_vec()]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn snapshot_supersedes_empty_object_id_rejected() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut snapshot = valid_snapshot_for_tests(b"trusted".to_vec());
        snapshot.supersedes = Some(crate::protocol::SnapshotRef {
            snapshot_id: b"previous-snapshot".to_vec(),
            object_id: Some(vec![]),
        });
        sign_snapshot(&mut snapshot, &signing_key);

        let result = validate_snapshot(&snapshot, &[b"trusted".to_vec()]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn snapshot_metadata_empty_object_id_rejected() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut snapshot = valid_snapshot_for_tests(b"trusted".to_vec());
        snapshot.snapshot_metadata = Some(ObjectRef {
            object_id: vec![],
            object_kind: None,
        });
        sign_snapshot(&mut snapshot, &signing_key);

        let result = validate_snapshot(&snapshot, &[b"trusted".to_vec()]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn object_retrieval_validates_descriptor_header_and_bytes() {
        let logical = b"hello object";
        let object_id = crate::crypto::derive_object_id(b"raw-bytes-v0", logical);
        let representation_id = crate::crypto::sha256(b"representation-id");
        let descriptor = LogicalObjectDescriptor {
            descriptor_version: 1,
            object_id: object_id.clone(),
            object_kind: 1,
            object_schema_version: 1,
            canonicalization_id: "raw-bytes-v0".into(),
            canonical_digest: Some(Digest {
                algorithm: 1,
                value: crate::crypto::sha256(logical),
            }),
            canonical_size: logical.len() as u64,
            created_at: None,
            producer: None,
            describes_object: None,
            object_metadata: None,
        };
        let header = StoredRepresentationHeader {
            header_version: 1,
            representation_id,
            object: Some(ObjectRef {
                object_id,
                object_kind: Some(1),
            }),
            representation_digest: Some(Digest {
                algorithm: 1,
                value: crate::crypto::derive_representation_digest(logical),
            }),
            plaintext_size: Some(logical.len() as u64),
            stored_size: logical.len() as u64,
            encryption_scheme: String::new(),
            compression_scheme: String::new(),
            chunking_mode: edgerun_proto::edgerun::v0::object::ChunkingMode::None as i32,
            chunk_manifest_object: None,
            access_package_object: None,
            created_at: None,
            representation_metadata: None,
        };

        let result = validate_object_retrieval(
            Some(&descriptor),
            Some(&header),
            None,
            Some(logical),
            Some(logical),
        );

        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn object_retrieval_rejects_representation_digest_mismatch() {
        let logical = b"hello object";
        let object_id = crate::crypto::derive_object_id(b"raw-bytes-v0", logical);
        let descriptor = LogicalObjectDescriptor {
            descriptor_version: 1,
            object_id: object_id.clone(),
            object_kind: 1,
            object_schema_version: 1,
            canonicalization_id: "raw-bytes-v0".into(),
            canonical_digest: Some(Digest {
                algorithm: 1,
                value: crate::crypto::sha256(logical),
            }),
            canonical_size: logical.len() as u64,
            created_at: None,
            producer: None,
            describes_object: None,
            object_metadata: None,
        };
        let header = StoredRepresentationHeader {
            header_version: 1,
            representation_id: vec![0x11; 32],
            object: Some(ObjectRef {
                object_id,
                object_kind: Some(1),
            }),
            representation_digest: Some(Digest {
                algorithm: 1,
                value: vec![0xFF; 32],
            }),
            plaintext_size: Some(logical.len() as u64),
            stored_size: logical.len() as u64,
            encryption_scheme: String::new(),
            compression_scheme: String::new(),
            chunking_mode: edgerun_proto::edgerun::v0::object::ChunkingMode::None as i32,
            chunk_manifest_object: None,
            access_package_object: None,
            created_at: None,
            representation_metadata: None,
        };

        let result = validate_object_retrieval(
            Some(&descriptor),
            Some(&header),
            None,
            Some(logical),
            Some(logical),
        );

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::RepresentationInvalid));
    }

    #[test]
    fn object_retrieval_defers_missing_manifest() {
        let header = StoredRepresentationHeader {
            header_version: 1,
            representation_id: vec![0x11; 32],
            object: Some(ObjectRef {
                object_id: vec![0x22; 32],
                object_kind: Some(1),
            }),
            representation_digest: Some(Digest {
                algorithm: 1,
                value: vec![0x33; 32],
            }),
            plaintext_size: None,
            stored_size: 10,
            encryption_scheme: String::new(),
            compression_scheme: String::new(),
            chunking_mode: edgerun_proto::edgerun::v0::object::ChunkingMode::Manifest as i32,
            chunk_manifest_object: None,
            access_package_object: None,
            created_at: None,
            representation_metadata: None,
        };

        let result = validate_object_retrieval(None, Some(&header), None, None, None);

        assert_eq!(result.verdict, crate::result::Verdict::Defer);
        assert_eq!(result.reason_code, Some(ReasonCode::MissingDependency));
    }

    #[test]
    fn object_retrieval_rejects_descriptor_missing_digest() {
        let descriptor = LogicalObjectDescriptor {
            descriptor_version: 1,
            object_id: vec![0x22; 32],
            object_kind: edgerun_proto::edgerun::v0::common::ObjectKind::Payload as i32,
            object_schema_version: 1,
            canonicalization_id: "raw-bytes-v0".into(),
            canonical_digest: None,
            canonical_size: 10,
            created_at: None,
            producer: None,
            describes_object: None,
            object_metadata: None,
        };

        let result = validate_object_retrieval(Some(&descriptor), None, None, None, None);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn object_retrieval_rejects_descriptor_digest_without_sha256_algorithm() {
        let descriptor = LogicalObjectDescriptor {
            descriptor_version: 1,
            object_id: vec![0x22; 32],
            object_kind: edgerun_proto::edgerun::v0::common::ObjectKind::Payload as i32,
            object_schema_version: 1,
            canonicalization_id: "raw-bytes-v0".into(),
            canonical_digest: Some(Digest {
                algorithm: 0,
                value: vec![0x33; 32],
            }),
            canonical_size: 10,
            created_at: None,
            producer: None,
            describes_object: None,
            object_metadata: None,
        };

        let result = validate_object_retrieval(Some(&descriptor), None, None, None, None);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn object_retrieval_rejects_invalid_descriptor_created_at_timestamp() {
        let descriptor = LogicalObjectDescriptor {
            descriptor_version: 1,
            object_id: vec![0x22; 32],
            object_kind: edgerun_proto::edgerun::v0::common::ObjectKind::Payload as i32,
            object_schema_version: 1,
            canonicalization_id: "raw-bytes-v0".into(),
            canonical_digest: Some(Digest {
                algorithm: 1,
                value: vec![0x33; 32],
            }),
            canonical_size: 10,
            created_at: Some(prost_types::Timestamp {
                seconds: 1,
                nanos: 1_000_000_000,
            }),
            producer: None,
            describes_object: None,
            object_metadata: None,
        };

        let result = validate_object_retrieval(Some(&descriptor), None, None, None, None);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn object_retrieval_rejects_descriptor_empty_metadata_ref() {
        let descriptor = LogicalObjectDescriptor {
            descriptor_version: 1,
            object_id: vec![0x22; 32],
            object_kind: edgerun_proto::edgerun::v0::common::ObjectKind::Payload as i32,
            object_schema_version: 1,
            canonicalization_id: "raw-bytes-v0".into(),
            canonical_digest: Some(Digest {
                algorithm: 1,
                value: vec![0x33; 32],
            }),
            canonical_size: 10,
            created_at: None,
            producer: None,
            describes_object: None,
            object_metadata: Some(ObjectRef {
                object_id: vec![],
                object_kind: None,
            }),
        };

        let result = validate_object_retrieval(Some(&descriptor), None, None, None, None);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn object_retrieval_rejects_unspecified_chunking_mode() {
        let header = StoredRepresentationHeader {
            header_version: 1,
            representation_id: vec![0x11; 32],
            object: Some(ObjectRef {
                object_id: vec![0x22; 32],
                object_kind: Some(1),
            }),
            representation_digest: Some(Digest {
                algorithm: 1,
                value: vec![0x33; 32],
            }),
            plaintext_size: None,
            stored_size: 10,
            encryption_scheme: String::new(),
            compression_scheme: String::new(),
            chunking_mode: edgerun_proto::edgerun::v0::object::ChunkingMode::Unspecified as i32,
            chunk_manifest_object: None,
            access_package_object: None,
            created_at: None,
            representation_metadata: None,
        };

        let result = validate_object_retrieval(None, Some(&header), None, None, None);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::RepresentationInvalid));
    }

    #[test]
    fn object_retrieval_rejects_invalid_header_object_kind() {
        let header = StoredRepresentationHeader {
            header_version: 1,
            representation_id: vec![0x11; 32],
            object: Some(ObjectRef {
                object_id: vec![0x22; 32],
                object_kind: Some(999_999),
            }),
            representation_digest: Some(Digest {
                algorithm: 1,
                value: vec![0x33; 32],
            }),
            plaintext_size: None,
            stored_size: 10,
            encryption_scheme: String::new(),
            compression_scheme: String::new(),
            chunking_mode: edgerun_proto::edgerun::v0::object::ChunkingMode::None as i32,
            chunk_manifest_object: None,
            access_package_object: None,
            created_at: None,
            representation_metadata: None,
        };

        let result = validate_object_retrieval(None, Some(&header), None, None, None);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn object_retrieval_rejects_representation_digest_without_sha256_algorithm() {
        let mut header = StoredRepresentationHeader {
            header_version: 1,
            representation_id: vec![0x11; 32],
            object: Some(ObjectRef {
                object_id: vec![0x22; 32],
                object_kind: Some(1),
            }),
            representation_digest: Some(Digest {
                algorithm: 0,
                value: vec![0x33; 32],
            }),
            plaintext_size: None,
            stored_size: 10,
            encryption_scheme: String::new(),
            compression_scheme: String::new(),
            chunking_mode: edgerun_proto::edgerun::v0::object::ChunkingMode::None as i32,
            chunk_manifest_object: None,
            access_package_object: None,
            created_at: None,
            representation_metadata: None,
        };

        let result = validate_object_retrieval(None, Some(&header), None, None, None);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::RepresentationInvalid));

        header.representation_digest.as_mut().unwrap().algorithm = 99;
        let result = validate_object_retrieval(None, Some(&header), None, None, None);
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::RepresentationInvalid));
    }

    #[test]
    fn object_retrieval_rejects_invalid_header_created_at_timestamp() {
        let header = StoredRepresentationHeader {
            header_version: 1,
            representation_id: vec![0x11; 32],
            object: Some(ObjectRef {
                object_id: vec![0x22; 32],
                object_kind: Some(1),
            }),
            representation_digest: Some(Digest {
                algorithm: 1,
                value: vec![0x33; 32],
            }),
            plaintext_size: None,
            stored_size: 10,
            encryption_scheme: String::new(),
            compression_scheme: String::new(),
            chunking_mode: edgerun_proto::edgerun::v0::object::ChunkingMode::None as i32,
            chunk_manifest_object: None,
            access_package_object: None,
            created_at: Some(prost_types::Timestamp {
                seconds: 1,
                nanos: -1,
            }),
            representation_metadata: None,
        };

        let result = validate_object_retrieval(None, Some(&header), None, None, None);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn object_retrieval_rejects_empty_header_access_package_ref() {
        let mut header = StoredRepresentationHeader {
            header_version: 1,
            representation_id: vec![0x11; 32],
            object: Some(ObjectRef {
                object_id: vec![0x22; 32],
                object_kind: Some(1),
            }),
            representation_digest: Some(Digest {
                algorithm: 1,
                value: vec![0x33; 32],
            }),
            plaintext_size: None,
            stored_size: 10,
            encryption_scheme: String::new(),
            compression_scheme: String::new(),
            chunking_mode: edgerun_proto::edgerun::v0::object::ChunkingMode::None as i32,
            chunk_manifest_object: None,
            access_package_object: None,
            created_at: None,
            representation_metadata: None,
        };
        header.access_package_object = Some(ObjectRef {
            object_id: vec![],
            object_kind: None,
        });

        let result = validate_object_retrieval(None, Some(&header), None, None, None);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    fn valid_manifest_header_and_manifest() -> (StoredRepresentationHeader, ChunkManifest) {
        let object_id = vec![0x22; 32];
        let representation_id = vec![0x11; 32];
        let header = StoredRepresentationHeader {
            header_version: 1,
            representation_id: representation_id.clone(),
            object: Some(ObjectRef {
                object_id: object_id.clone(),
                object_kind: Some(1),
            }),
            representation_digest: Some(Digest {
                algorithm: 1,
                value: vec![0x33; 32],
            }),
            plaintext_size: None,
            stored_size: 10,
            encryption_scheme: String::new(),
            compression_scheme: String::new(),
            chunking_mode: edgerun_proto::edgerun::v0::object::ChunkingMode::Manifest as i32,
            chunk_manifest_object: None,
            access_package_object: None,
            created_at: None,
            representation_metadata: None,
        };
        let manifest = ChunkManifest {
            manifest_version: 1,
            object: Some(ObjectRef {
                object_id: object_id.clone(),
                object_kind: Some(1),
            }),
            representation: Some(RepresentationRef {
                representation_id,
                object_id,
            }),
            chunk_count: 1,
            total_stored_size: 10,
            chunk_entries: vec![ChunkEntry {
                index: 0,
                chunk_representation_id: vec![0x44; 32],
                chunk_digest: Some(Digest {
                    algorithm: 1,
                    value: vec![0x55; 32],
                }),
                offset: 0,
                length: 10,
            }],
            manifest_metadata: None,
        };
        (header, manifest)
    }

    #[test]
    fn object_retrieval_rejects_empty_chunk_manifest() {
        let (header, mut manifest) = valid_manifest_header_and_manifest();
        manifest.chunk_count = 0;
        manifest.chunk_entries.clear();
        manifest.total_stored_size = 0;

        let result = validate_object_retrieval(None, Some(&header), Some(&manifest), None, None);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::RepresentationInvalid));
    }

    #[test]
    fn object_retrieval_rejects_empty_chunk_representation_id() {
        let (header, mut manifest) = valid_manifest_header_and_manifest();
        manifest.chunk_entries[0].chunk_representation_id.clear();

        let result = validate_object_retrieval(None, Some(&header), Some(&manifest), None, None);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::RepresentationInvalid));
    }

    #[test]
    fn object_retrieval_rejects_chunk_digest_without_sha256_algorithm() {
        let (header, mut manifest) = valid_manifest_header_and_manifest();
        manifest.chunk_entries[0]
            .chunk_digest
            .as_mut()
            .unwrap()
            .algorithm = 0;

        let result = validate_object_retrieval(None, Some(&header), Some(&manifest), None, None);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::RepresentationInvalid));
    }

    #[test]
    fn object_retrieval_rejects_noncontiguous_chunk_indexes() {
        let (header, mut manifest) = valid_manifest_header_and_manifest();
        manifest.chunk_entries[0].index = 1;

        let result = validate_object_retrieval(None, Some(&header), Some(&manifest), None, None);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::RepresentationInvalid));
    }

    #[test]
    fn object_retrieval_rejects_gapped_chunk_offsets() {
        let (mut header, mut manifest) = valid_manifest_header_and_manifest();
        header.stored_size = 15;
        manifest.chunk_count = 2;
        manifest.total_stored_size = 15;
        manifest.chunk_entries[0].length = 10;
        let mut second = manifest.chunk_entries[0].clone();
        second.index = 1;
        second.offset = 11;
        second.length = 5;
        manifest.chunk_entries.push(second);

        let result = validate_object_retrieval(None, Some(&header), Some(&manifest), None, None);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::RepresentationInvalid));
    }

    #[test]
    fn object_retrieval_rejects_overlapping_chunk_offsets() {
        let (mut header, mut manifest) = valid_manifest_header_and_manifest();
        header.stored_size = 15;
        manifest.chunk_count = 2;
        manifest.total_stored_size = 15;
        manifest.chunk_entries[0].length = 10;
        let mut second = manifest.chunk_entries[0].clone();
        second.index = 1;
        second.offset = 9;
        second.length = 5;
        manifest.chunk_entries.push(second);

        let result = validate_object_retrieval(None, Some(&header), Some(&manifest), None, None);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::RepresentationInvalid));
    }

    #[test]
    fn object_retrieval_rejects_manifest_representation_object_mismatch() {
        let (header, mut manifest) = valid_manifest_header_and_manifest();
        manifest.representation.as_mut().unwrap().object_id = vec![0x99; 32];

        let result = validate_object_retrieval(None, Some(&header), Some(&manifest), None, None);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::ObjectIdMismatch));
    }

    #[test]
    fn object_retrieval_rejects_invalid_manifest_object_kind() {
        let (header, mut manifest) = valid_manifest_header_and_manifest();
        manifest.object.as_mut().unwrap().object_kind =
            Some(edgerun_proto::edgerun::v0::common::ObjectKind::Unspecified as i32);

        let result = validate_object_retrieval(None, Some(&header), Some(&manifest), None, None);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn object_retrieval_rejects_empty_manifest_metadata_ref() {
        let (header, mut manifest) = valid_manifest_header_and_manifest();
        manifest.manifest_metadata = Some(ObjectRef {
            object_id: vec![],
            object_kind: None,
        });

        let result = validate_object_retrieval(None, Some(&header), Some(&manifest), None, None);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    fn control_command(
        command_type: CommandType,
        target_controller: Vec<u8>,
        issuer: Vec<u8>,
    ) -> CommandEnvelope {
        CommandEnvelope {
            envelope_version: 1,
            command_id: target_controller,
            target_node: Some(NodeRef {
                node_id: b"node".to_vec(),
            }),
            issuer: Some(IdentityRef {
                identity_id: issuer,
                identity_kind: Some(1),
                key_hint: None,
            }),
            command_type: command_type as i32,
            command_version: 1,
            issued_at: None,
            not_before: None,
            expires_at: None,
            idempotency_key: vec![],
            payload: None,
            delegation_chain: vec![],
            requested_assurance: None,
            command_metadata: None,
            signature: None,
        }
    }

    #[test]
    fn control_add_controller_accepts_and_simulates_post_state() {
        let issuer = b"controller-a".to_vec();
        let new_controller = b"controller-b".to_vec();
        let command = control_command(
            CommandType::AddController,
            new_controller.clone(),
            issuer.clone(),
        );

        let result = validate_control_change_command(&command, &[issuer], 1);

        assert_eq!(result.verdict, crate::result::Verdict::Accept);
        let post_state = result.post_state.as_map().unwrap();
        let controller_set = post_state.get("controller_set").unwrap().as_seq().unwrap();
        assert_eq!(controller_set.len(), 2);
        assert!(controller_set
            .iter()
            .any(|v| { v.as_str() == Some(crate::util::bytes_to_hex(&new_controller).as_str()) }));
    }

    #[test]
    fn control_remove_controller_rejects_below_minimum() {
        let issuer = b"controller-a".to_vec();
        let command = control_command(
            CommandType::RemoveController,
            issuer.clone(),
            issuer.clone(),
        );

        let result = validate_control_change_command(&command, &[issuer], 1);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::ControlInvariantFailed));
    }

    #[test]
    fn control_change_rejects_non_controller_issuer() {
        let command = control_command(
            CommandType::AddController,
            b"controller-b".to_vec(),
            b"intruder".to_vec(),
        );

        let result = validate_control_change_command(&command, &[b"controller-a".to_vec()], 1);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn control_transfer_adds_new_controller_for_safe_transfer_step() {
        let issuer = b"controller-a".to_vec();
        let new_controller = b"controller-b".to_vec();
        let command = control_command(CommandType::TransferControl, new_controller, issuer.clone());

        let result = validate_control_change_command(&command, &[issuer], 1);

        assert_eq!(result.verdict, crate::result::Verdict::Accept);
        let post_state = result.post_state.as_map().unwrap();
        let controller_set = post_state.get("controller_set").unwrap().as_seq().unwrap();
        assert_eq!(controller_set.len(), 2);
    }

    fn signed_query_request(key: &edgerun_crypto::p256::ecdsa::SigningKey) -> QueryRequest {
        let key_hint = test_node_id(key);
        let mut query = QueryRequest {
            request_version: 1,
            query_id: b"query-1".to_vec(),
            requester: Some(IdentityRef {
                identity_id: b"requester".to_vec(),
                identity_kind: Some(1),
                key_hint: Some(key_hint),
            }),
            target_scope: Some(crate::protocol::ScopeDescriptor {
                scope_version: 1,
                scope_kind: edgerun_proto::edgerun::v0::trust::ScopeKind::Node as i32,
                target_nodes: vec![NodeRef {
                    node_id: b"target-node".to_vec(),
                }],
                target_streams: vec![],
                target_object_kinds: vec![],
                target_view_types: vec![],
                target_domains: vec![],
                time_bounds: None,
                scope_metadata: None,
            }),
            query_class: edgerun_proto::edgerun::v0::access::QueryClass::Head as i32,
            time_window: None,
            checkpoint_base: None,
            result_limit: Some(10),
            cost_limit: None,
            required_proof_classes: vec![],
            query_payload_object: None,
            signature: None,
        };
        let canonical = canonical_bytes(&ProtocolRecord::QueryRequest(query.clone()), true);
        let sig = crate::crypto::sign_canonical_record(
            key,
            crate::crypto::SIG_DOMAIN_QUERY_REQUEST,
            &canonical,
        )
        .unwrap();
        query.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: sig,
        });
        query
    }

    fn sign_query_request(query: &mut QueryRequest, key: &edgerun_crypto::p256::ecdsa::SigningKey) {
        query.signature = None;
        let canonical = canonical_bytes(&ProtocolRecord::QueryRequest(query.clone()), true);
        let sig = crate::crypto::sign_canonical_record(
            key,
            crate::crypto::SIG_DOMAIN_QUERY_REQUEST,
            &canonical,
        )
        .unwrap();
        query.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: sig,
        });
    }

    #[test]
    fn query_request_signature_valid_is_accepted() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let query = signed_query_request(&key);

        let result = validate_query_request_signature(&query);

        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn query_request_missing_signature_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut query = signed_query_request(&key);
        query.signature = None;

        let result = validate_query_request_signature(&query);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
    }

    #[test]
    fn query_request_bad_signature_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut query = signed_query_request(&key);
        if let Some(sig) = &mut query.signature {
            sig.value[0] ^= 0xFF;
        }

        let result = validate_query_request_signature(&query);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
    }

    #[test]
    fn query_request_missing_target_scope_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut query = signed_query_request(&key);
        query.target_scope = None;
        sign_query_request(&mut query, &key);

        let result = validate_query_request_signature(&query);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn query_request_invalid_query_class_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut query = signed_query_request(&key);
        query.query_class = 999_999;
        sign_query_request(&mut query, &key);

        let result = validate_query_request_signature(&query);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn query_request_invalid_required_proof_class_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut query = signed_query_request(&key);
        query.required_proof_classes = vec![999_999];
        sign_query_request(&mut query, &key);

        let result = validate_query_request_signature(&query);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn query_request_zero_result_limit_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut query = signed_query_request(&key);
        query.result_limit = Some(0);
        sign_query_request(&mut query, &key);

        let result = validate_query_request_signature(&query);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn query_request_inverted_time_window_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut query = signed_query_request(&key);
        query.time_window = Some(TimeWindow {
            not_before: Some(prost_types::Timestamp {
                seconds: 20,
                nanos: 0,
            }),
            expires_at: Some(prost_types::Timestamp {
                seconds: 10,
                nanos: 0,
            }),
        });
        sign_query_request(&mut query, &key);

        let result = validate_query_request_signature(&query);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::TimeInvalid));
    }

    #[test]
    fn query_request_invalid_time_window_shape_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut query = signed_query_request(&key);
        query.time_window = Some(TimeWindow {
            not_before: Some(prost_types::Timestamp {
                seconds: 10,
                nanos: -1,
            }),
            expires_at: None,
        });
        sign_query_request(&mut query, &key);

        let result = validate_query_request_signature(&query);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn query_request_empty_checkpoint_base_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut query = signed_query_request(&key);
        query.checkpoint_base = Some(CheckpointRef {
            checkpoint_id: None,
            heads: vec![],
        });
        sign_query_request(&mut query, &key);

        let result = validate_query_request_signature(&query);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn query_request_checkpoint_base_head_missing_hash_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut query = signed_query_request(&key);
        query.checkpoint_base = Some(CheckpointRef {
            checkpoint_id: None,
            heads: vec![HeadRef {
                stream_id: b"stream-1".to_vec(),
                seq: 1,
                event_hash: None,
            }],
        });
        sign_query_request(&mut query, &key);

        let result = validate_query_request_signature(&query);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn query_request_empty_payload_object_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut query = signed_query_request(&key);
        query.query_payload_object = Some(ObjectRef {
            object_id: vec![],
            object_kind: Some(1),
        });
        sign_query_request(&mut query, &key);

        let result = validate_query_request_signature(&query);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn query_request_invalid_payload_object_kind_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut query = signed_query_request(&key);
        query.query_payload_object = Some(ObjectRef {
            object_id: b"payload-object".to_vec(),
            object_kind: Some(0),
        });
        sign_query_request(&mut query, &key);

        let result = validate_query_request_signature(&query);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn query_request_empty_target_stream_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut query = signed_query_request(&key);
        query
            .target_scope
            .as_mut()
            .unwrap()
            .target_streams
            .push(crate::protocol::StreamRef { stream_id: vec![] });
        sign_query_request(&mut query, &key);

        let result = validate_query_request_signature(&query);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn query_request_scope_metadata_empty_object_id_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut query = signed_query_request(&key);
        query.target_scope.as_mut().unwrap().scope_metadata = Some(ObjectRef {
            object_id: vec![],
            object_kind: None,
        });
        sign_query_request(&mut query, &key);

        let result = validate_query_request_signature(&query);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn query_request_zero_cost_limit_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut query = signed_query_request(&key);
        query.cost_limit = Some(CostLimit {
            max_results: Some(0),
            max_total_bytes: None,
            max_wall_time: None,
            max_federated_responders: None,
        });
        sign_query_request(&mut query, &key);

        let result = validate_query_request_signature(&query);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn query_request_zero_wall_time_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut query = signed_query_request(&key);
        query.cost_limit = Some(CostLimit {
            max_results: None,
            max_total_bytes: None,
            max_wall_time: Some(prost_types::Duration {
                seconds: 0,
                nanos: 0,
            }),
            max_federated_responders: None,
        });
        sign_query_request(&mut query, &key);

        let result = validate_query_request_signature(&query);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::TimeInvalid));
    }

    #[test]
    fn query_request_invalid_wall_time_duration_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut query = signed_query_request(&key);
        query.cost_limit = Some(CostLimit {
            max_results: None,
            max_total_bytes: None,
            max_wall_time: Some(prost_types::Duration {
                seconds: 1,
                nanos: 1_000_000_000,
            }),
            max_federated_responders: None,
        });
        sign_query_request(&mut query, &key);

        let result = validate_query_request_signature(&query);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::TimeInvalid));
    }

    fn signed_query_result_fragment(
        key: &edgerun_crypto::p256::ecdsa::SigningKey,
    ) -> QueryResultFragment {
        let key_hint = test_node_id(key);
        let mut fragment = QueryResultFragment {
            fragment_version: 1,
            query_id: b"query-1".to_vec(),
            responder: Some(IdentityRef {
                identity_id: b"responder".to_vec(),
                identity_kind: Some(1),
                key_hint: Some(key_hint),
            }),
            answered_at: Some(prost_types::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            completeness:
                edgerun_proto::edgerun::v0::access::ResultCompleteness::CompleteForLocalKnowledge
                    as i32,
            snapshot_refs: vec![],
            event_refs: vec![],
            object_refs: vec![ObjectRef {
                object_id: b"object-1".to_vec(),
                object_kind: Some(1),
            }],
            proof_objects: vec![],
            omission_reason: String::new(),
            bundled_result_object: None,
            result_metadata: None,
            signature: None,
        };
        let canonical =
            canonical_bytes(&ProtocolRecord::QueryResultFragment(fragment.clone()), true);
        let sig = crate::crypto::sign_canonical_record(
            key,
            crate::crypto::SIG_DOMAIN_QUERY_RESULT_FRAGMENT,
            &canonical,
        )
        .unwrap();
        fragment.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: sig,
        });
        fragment
    }

    #[test]
    fn query_result_fragment_valid_is_accepted_as_advisory() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[44u8; 32].into()).unwrap();
        let fragment = signed_query_result_fragment(&key);

        let result = validate_query_result_fragment(&fragment, Some(b"query-1"), &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Accept);
        assert_eq!(
            result.derived.as_map().unwrap().get("advisory_only"),
            Some(&Value::Bool(true))
        );
    }

    #[test]
    fn query_result_fragment_invalid_answered_at_timestamp_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[44u8; 32].into()).unwrap();
        let mut fragment = signed_query_result_fragment(&key);
        fragment.answered_at = Some(prost_types::Timestamp {
            seconds: 1_700_000_000,
            nanos: -1,
        });
        let canonical =
            canonical_bytes(&ProtocolRecord::QueryResultFragment(fragment.clone()), true);
        fragment.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: crate::crypto::sign_canonical_record(
                &key,
                crate::crypto::SIG_DOMAIN_QUERY_RESULT_FRAGMENT,
                &canonical,
            )
            .unwrap(),
        });

        let result = validate_query_result_fragment(&fragment, Some(b"query-1"), &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn query_result_fragment_without_backing_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[44u8; 32].into()).unwrap();
        let mut fragment = signed_query_result_fragment(&key);
        fragment.object_refs.clear();
        let canonical =
            canonical_bytes(&ProtocolRecord::QueryResultFragment(fragment.clone()), true);
        fragment.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: crate::crypto::sign_canonical_record(
                &key,
                crate::crypto::SIG_DOMAIN_QUERY_RESULT_FRAGMENT,
                &canonical,
            )
            .unwrap(),
        });

        let result = validate_query_result_fragment(&fragment, Some(b"query-1"), &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn query_result_fragment_metadata_only_is_not_backing() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[44u8; 32].into()).unwrap();
        let mut fragment = signed_query_result_fragment(&key);
        fragment.object_refs.clear();
        fragment.result_metadata = Some(ObjectRef {
            object_id: b"metadata-object".to_vec(),
            object_kind: None,
        });
        let canonical =
            canonical_bytes(&ProtocolRecord::QueryResultFragment(fragment.clone()), true);
        fragment.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: crate::crypto::sign_canonical_record(
                &key,
                crate::crypto::SIG_DOMAIN_QUERY_RESULT_FRAGMENT,
                &canonical,
            )
            .unwrap(),
        });

        let result = validate_query_result_fragment(&fragment, Some(b"query-1"), &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn query_result_fragment_metadata_only_with_metadata_is_accepted() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[44u8; 32].into()).unwrap();
        let mut fragment = signed_query_result_fragment(&key);
        fragment.object_refs.clear();
        fragment.completeness =
            edgerun_proto::edgerun::v0::access::ResultCompleteness::MetadataOnly as i32;
        fragment.result_metadata = Some(ObjectRef {
            object_id: b"metadata-object".to_vec(),
            object_kind: None,
        });
        let canonical =
            canonical_bytes(&ProtocolRecord::QueryResultFragment(fragment.clone()), true);
        fragment.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: crate::crypto::sign_canonical_record(
                &key,
                crate::crypto::SIG_DOMAIN_QUERY_RESULT_FRAGMENT,
                &canonical,
            )
            .unwrap(),
        });

        let result = validate_query_result_fragment(&fragment, Some(b"query-1"), &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Accept);
        assert_eq!(
            result.derived.as_map().unwrap().get("advisory_only"),
            Some(&Value::Bool(true))
        );
    }

    #[test]
    fn query_result_fragment_metadata_only_requires_metadata() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[44u8; 32].into()).unwrap();
        let mut fragment = signed_query_result_fragment(&key);
        fragment.object_refs.clear();
        fragment.completeness =
            edgerun_proto::edgerun::v0::access::ResultCompleteness::MetadataOnly as i32;
        let canonical =
            canonical_bytes(&ProtocolRecord::QueryResultFragment(fragment.clone()), true);
        fragment.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: crate::crypto::sign_canonical_record(
                &key,
                crate::crypto::SIG_DOMAIN_QUERY_RESULT_FRAGMENT,
                &canonical,
            )
            .unwrap(),
        });

        let result = validate_query_result_fragment(&fragment, Some(b"query-1"), &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn query_result_fragment_bad_signature_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[44u8; 32].into()).unwrap();
        let mut fragment = signed_query_result_fragment(&key);
        if let Some(sig) = &mut fragment.signature {
            sig.value[0] ^= 0xFF;
        }

        let result = validate_query_result_fragment(&fragment, Some(b"query-1"), &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
    }

    #[test]
    fn query_result_fragment_empty_object_ref_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[44u8; 32].into()).unwrap();
        let mut fragment = signed_query_result_fragment(&key);
        fragment.object_refs[0].object_id.clear();

        let result = validate_query_result_fragment(&fragment, Some(b"query-1"), &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn query_result_fragment_event_ref_missing_hash_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[44u8; 32].into()).unwrap();
        let mut fragment = signed_query_result_fragment(&key);
        fragment.object_refs.clear();
        fragment.event_refs.push(EventRef {
            stream_id: b"stream-1".to_vec(),
            seq: 7,
            event_hash: None,
        });

        let result = validate_query_result_fragment(&fragment, Some(b"query-1"), &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn query_result_fragment_snapshot_ref_empty_id_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[44u8; 32].into()).unwrap();
        let mut fragment = signed_query_result_fragment(&key);
        fragment.object_refs.clear();
        fragment.snapshot_refs.push(SnapshotRef {
            snapshot_id: vec![],
            object_id: None,
        });

        let result = validate_query_result_fragment(&fragment, Some(b"query-1"), &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn query_result_fragment_snapshot_ref_empty_object_id_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[44u8; 32].into()).unwrap();
        let mut fragment = signed_query_result_fragment(&key);
        fragment.object_refs.clear();
        fragment.snapshot_refs.push(SnapshotRef {
            snapshot_id: b"snapshot-1".to_vec(),
            object_id: Some(vec![]),
        });

        let result = validate_query_result_fragment(&fragment, Some(b"query-1"), &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn query_result_fragment_empty_proof_object_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[44u8; 32].into()).unwrap();
        let mut fragment = signed_query_result_fragment(&key);
        fragment.object_refs.clear();
        fragment.proof_objects.push(ObjectRef {
            object_id: vec![],
            object_kind: Some(1),
        });

        let result = validate_query_result_fragment(&fragment, Some(b"query-1"), &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn query_result_denial_without_backing_is_accepted() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[44u8; 32].into()).unwrap();
        let mut fragment = signed_query_result_fragment(&key);
        fragment.object_refs.clear();
        fragment.completeness =
            edgerun_proto::edgerun::v0::access::ResultCompleteness::Denied as i32;
        fragment.omission_reason = "policy_denied".into();
        let canonical =
            canonical_bytes(&ProtocolRecord::QueryResultFragment(fragment.clone()), true);
        fragment.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: crate::crypto::sign_canonical_record(
                &key,
                crate::crypto::SIG_DOMAIN_QUERY_RESULT_FRAGMENT,
                &canonical,
            )
            .unwrap(),
        });

        let result = validate_query_result_fragment(&fragment, Some(b"query-1"), &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    fn signed_session_hello(key: &edgerun_crypto::p256::ecdsa::SigningKey) -> SessionHello {
        let node_id = test_node_id(key);
        let mut hello = SessionHello {
            message_version: 1,
            initiator: Some(IdentityRef {
                identity_id: node_id.clone(),
                identity_kind: Some(1),
                key_hint: Some(node_id),
            }),
            target_node: Some(NodeRef {
                node_id: b"target-node".to_vec(),
            }),
            supported_transport_features: vec!["tcp".into()],
            supported_protocol_versions: vec![1],
            session_nonce: b"nonce-1".to_vec(),
            initiator_locators: vec![],
            hello_metadata: None,
            signature: None,
        };
        let canonical = canonical_bytes(&ProtocolRecord::SessionHello(hello.clone()), true);
        let sig = crate::crypto::sign_canonical_record(
            key,
            crate::crypto::SIG_DOMAIN_SESSION_HELLO,
            &canonical,
        )
        .unwrap();
        hello.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: sig,
        });
        hello
    }

    fn signed_session_accept(key: &edgerun_crypto::p256::ecdsa::SigningKey) -> SessionAccept {
        let node_id = test_node_id(key);
        let mut accept_msg = SessionAccept {
            message_version: 1,
            responder: Some(IdentityRef {
                identity_id: node_id.clone(),
                identity_kind: Some(1),
                key_hint: Some(node_id),
            }),
            echoed_session_nonce: b"nonce-1".to_vec(),
            selected_protocol_version: 1,
            selected_transport_features: vec!["tcp".into()],
            responder_locators: vec![],
            accept_metadata: None,
            signature: None,
        };
        let canonical = canonical_bytes(&ProtocolRecord::SessionAccept(accept_msg.clone()), true);
        let sig = crate::crypto::sign_canonical_record(
            key,
            crate::crypto::SIG_DOMAIN_SESSION_ACCEPT,
            &canonical,
        )
        .unwrap();
        accept_msg.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: sig,
        });
        accept_msg
    }

    #[test]
    fn session_hello_valid_is_accepted() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let hello = signed_session_hello(&key);

        let result = validate_session_hello(&hello, Some(b"target-node"));

        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn session_hello_target_mismatch_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let hello = signed_session_hello(&key);

        let result = validate_session_hello(&hello, Some(b"other-node"));

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::TargetMismatch));
    }

    #[test]
    fn session_hello_invalid_initiator_kind_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut hello = signed_session_hello(&key);
        hello.initiator.as_mut().unwrap().identity_kind = Some(0);

        let result = validate_session_hello(&hello, Some(b"target-node"));

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn session_hello_bad_signature_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut hello = signed_session_hello(&key);
        hello.session_nonce.push(0xFF);

        let result = validate_session_hello(&hello, Some(b"target-node"));

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
    }

    #[test]
    fn session_hello_zero_protocol_version_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut hello = signed_session_hello(&key);
        hello.supported_protocol_versions = vec![0];

        let result = validate_session_hello(&hello, Some(b"target-node"));

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::VersionUnsupported));
    }

    #[test]
    fn session_hello_empty_target_node_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut hello = signed_session_hello(&key);
        hello.target_node = Some(NodeRef { node_id: vec![] });

        let result = validate_session_hello(&hello, None);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn session_hello_empty_transport_feature_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut hello = signed_session_hello(&key);
        hello.supported_transport_features.push(String::new());

        let result = validate_session_hello(&hello, Some(b"target-node"));

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn session_hello_empty_metadata_object_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut hello = signed_session_hello(&key);
        hello.hello_metadata = Some(ObjectRef {
            object_id: vec![],
            object_kind: None,
        });

        let result = validate_session_hello(&hello, Some(b"target-node"));

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn session_hello_invalid_locator_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut hello = signed_session_hello(&key);
        hello
            .initiator_locators
            .push(edgerun_proto::edgerun::v0::network::ReachabilityHint {
                hint_version: 1,
                subject_node: Some(NodeRef {
                    node_id: b"node-a".to_vec(),
                }),
                transport_class: edgerun_proto::edgerun::v0::common::TransportClass::Unspecified
                    as i32,
                locator_payload: b"addr".to_vec(),
                directness: edgerun_proto::edgerun::v0::common::Directness::Direct as i32,
                valid_after: None,
                valid_until: None,
                cost_hint: None,
                quality_hint: None,
                issuer: None,
                signature: None,
            });

        let result = validate_session_hello(&hello, Some(b"target-node"));

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn session_hello_invalid_locator_timestamp_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut hello = signed_session_hello(&key);
        hello
            .initiator_locators
            .push(edgerun_proto::edgerun::v0::network::ReachabilityHint {
                hint_version: 1,
                subject_node: Some(NodeRef {
                    node_id: b"node-a".to_vec(),
                }),
                transport_class: edgerun_proto::edgerun::v0::common::TransportClass::Quic as i32,
                locator_payload: b"addr".to_vec(),
                directness: edgerun_proto::edgerun::v0::common::Directness::Direct as i32,
                valid_after: Some(prost_types::Timestamp {
                    seconds: 10,
                    nanos: 1_000_000_000,
                }),
                valid_until: None,
                cost_hint: None,
                quality_hint: None,
                issuer: None,
                signature: None,
            });

        let result = validate_session_hello(&hello, Some(b"target-node"));

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn session_accept_valid_is_accepted() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let accept_msg = signed_session_accept(&key);

        let result = validate_session_accept(&accept_msg, b"nonce-1", &[1]);

        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn session_accept_nonce_mismatch_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let accept_msg = signed_session_accept(&key);

        let result = validate_session_accept(&accept_msg, b"other-nonce", &[1]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
    }

    #[test]
    fn session_accept_empty_nonce_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut accept_msg = signed_session_accept(&key);
        accept_msg.echoed_session_nonce.clear();
        accept_msg.signature = None;

        let result = validate_session_accept(&accept_msg, b"", &[1]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn session_accept_unsupported_version_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let accept_msg = signed_session_accept(&key);

        let result = validate_session_accept(&accept_msg, b"nonce-1", &[2]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::VersionUnsupported));
    }

    #[test]
    fn session_accept_zero_protocol_version_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut accept_msg = signed_session_accept(&key);
        accept_msg.selected_protocol_version = 0;

        let result = validate_session_accept(&accept_msg, b"nonce-1", &[0, 1]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::VersionUnsupported));
    }

    #[test]
    fn session_accept_empty_transport_feature_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut accept_msg = signed_session_accept(&key);
        accept_msg.selected_transport_features.push(String::new());

        let result = validate_session_accept(&accept_msg, b"nonce-1", &[1]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn session_accept_empty_metadata_object_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut accept_msg = signed_session_accept(&key);
        accept_msg.accept_metadata = Some(ObjectRef {
            object_id: vec![],
            object_kind: None,
        });

        let result = validate_session_accept(&accept_msg, b"nonce-1", &[1]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn session_accept_inverted_locator_window_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut accept_msg = signed_session_accept(&key);
        accept_msg
            .responder_locators
            .push(edgerun_proto::edgerun::v0::network::ReachabilityHint {
                hint_version: 1,
                subject_node: Some(NodeRef {
                    node_id: b"node-a".to_vec(),
                }),
                transport_class: edgerun_proto::edgerun::v0::common::TransportClass::Quic as i32,
                locator_payload: b"addr".to_vec(),
                directness: edgerun_proto::edgerun::v0::common::Directness::Direct as i32,
                valid_after: Some(prost_types::Timestamp {
                    seconds: 20,
                    nanos: 0,
                }),
                valid_until: Some(prost_types::Timestamp {
                    seconds: 10,
                    nanos: 0,
                }),
                cost_hint: None,
                quality_hint: None,
                issuer: None,
                signature: None,
            });

        let result = validate_session_accept(&accept_msg, b"nonce-1", &[1]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::TimeInvalid));
    }

    fn signed_relay_envelope(key: &edgerun_crypto::p256::ecdsa::SigningKey) -> RelayEnvelope {
        let node_id = test_node_id(key);
        let mut envelope = RelayEnvelope {
            envelope_version: 1,
            relay_message_id: b"relay-1".to_vec(),
            original_sender: Some(IdentityRef {
                identity_id: node_id.clone(),
                identity_kind: Some(1),
                key_hint: Some(node_id),
            }),
            intended_recipient_node: Some(NodeRef {
                node_id: b"target-node".to_vec(),
            }),
            relay_chain: vec![IdentityRef {
                identity_id: b"relay-hop".to_vec(),
                identity_kind: Some(1),
                key_hint: None,
            }],
            payload_kind: edgerun_proto::edgerun::v0::network::PayloadKind::Command as i32,
            payload: Some(
                edgerun_proto::edgerun::v0::network::relay_envelope::Payload::InlinePayload(
                    b"payload".to_vec(),
                ),
            ),
            store_until: Some(prost_types::Timestamp {
                seconds: 1_800_000_000,
                nanos: 0,
            }),
            relay_metadata: None,
            signature: None,
        };
        let canonical = canonical_bytes(&ProtocolRecord::RelayEnvelope(envelope.clone()), true);
        let sig = crate::crypto::sign_canonical_record(
            key,
            crate::crypto::SIG_DOMAIN_RELAY_ENVELOPE,
            &canonical,
        )
        .unwrap();
        envelope.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: sig,
        });
        envelope
    }

    #[test]
    fn relay_envelope_valid_is_advisory_accept() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let envelope = signed_relay_envelope(&key);

        let result = validate_relay_envelope(&envelope, Some(b"target-node"), 1_710_000_000_000);

        assert_eq!(result.verdict, crate::result::Verdict::Accept);
        assert_eq!(
            result.derived.as_map().unwrap().get("advisory_only"),
            Some(&Value::Bool(true))
        );
    }

    #[test]
    fn relay_envelope_missing_payload_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut envelope = signed_relay_envelope(&key);
        envelope.payload = None;
        envelope.signature = None;

        let result = validate_relay_envelope(&envelope, Some(b"target-node"), 1_710_000_000_000);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn relay_envelope_invalid_relay_identity_kind_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut envelope = signed_relay_envelope(&key);
        envelope.relay_chain[0].identity_kind = Some(999_999);

        let result = validate_relay_envelope(&envelope, Some(b"target-node"), 1_710_000_000_000);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn relay_envelope_invalid_payload_kind_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut envelope = signed_relay_envelope(&key);
        envelope.payload_kind =
            edgerun_proto::edgerun::v0::network::PayloadKind::Unspecified as i32;
        envelope.signature = None;

        let result = validate_relay_envelope(&envelope, Some(b"target-node"), 1_710_000_000_000);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn relay_envelope_target_mismatch_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let envelope = signed_relay_envelope(&key);

        let result = validate_relay_envelope(&envelope, Some(b"other-node"), 1_710_000_000_000);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::TargetMismatch));
    }

    #[test]
    fn relay_envelope_expired_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let envelope = signed_relay_envelope(&key);

        let result = validate_relay_envelope(&envelope, Some(b"target-node"), 1_900_000_000_000);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::TimeInvalid));
    }

    #[test]
    fn relay_envelope_bad_signature_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut envelope = signed_relay_envelope(&key);
        if let Some(sig) = &mut envelope.signature {
            sig.value[0] ^= 0xFF;
        }

        let result = validate_relay_envelope(&envelope, Some(b"target-node"), 1_710_000_000_000);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
    }

    #[test]
    fn relay_envelope_empty_metadata_object_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut envelope = signed_relay_envelope(&key);
        envelope.relay_metadata = Some(ObjectRef {
            object_id: vec![],
            object_kind: None,
        });
        envelope.signature = None;

        let result = validate_relay_envelope(&envelope, Some(b"target-node"), 1_710_000_000_000);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    fn sign_assurance_claim(
        claim: &mut AssuranceClaim,
        key: &edgerun_crypto::p256::ecdsa::SigningKey,
    ) {
        claim.signature = None;
        if let Some(attester) = &mut claim.attester {
            attester.key_hint = Some(test_node_id(key));
        }
        let canonical = canonical_bytes(&ProtocolRecord::AssuranceClaim(claim.clone()), true);
        let sig = crate::crypto::sign_canonical_record(
            key,
            crate::crypto::SIG_DOMAIN_ASSURANCE_CLAIM,
            &canonical,
        )
        .unwrap();
        claim.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: sig,
        });
    }

    fn signed_assurance_claim(key: &edgerun_crypto::p256::ecdsa::SigningKey) -> AssuranceClaim {
        let node_id = test_node_id(key);
        let mut claim = AssuranceClaim {
            claim_version: 1,
            subject: Some(
                edgerun_proto::edgerun::v0::trust::assurance_claim::Subject::SubjectNode(NodeRef {
                    node_id: b"subject-node".to_vec(),
                }),
            ),
            assurance_class: edgerun_proto::edgerun::v0::common::AssuranceClass::HardwareBacked
                as i32,
            attester: Some(IdentityRef {
                identity_id: node_id.clone(),
                identity_kind: Some(1),
                key_hint: Some(node_id),
            }),
            issued_at: Some(prost_types::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            expires_at: Some(prost_types::Timestamp {
                seconds: 1_800_000_000,
                nanos: 0,
            }),
            evidence_object: None,
            claim_note: "test".into(),
            signature: None,
        };
        sign_assurance_claim(&mut claim, key);
        claim
    }

    fn sign_revocation_record(
        revocation: &mut RevocationRecord,
        key: &edgerun_crypto::p256::ecdsa::SigningKey,
    ) {
        revocation.signature = None;
        if let Some(issuer) = &mut revocation.issuer {
            issuer.key_hint = Some(test_node_id(key));
        }
        let canonical =
            canonical_bytes(&ProtocolRecord::RevocationRecord(revocation.clone()), true);
        let sig = crate::crypto::sign_canonical_record(
            key,
            crate::crypto::SIG_DOMAIN_REVOCATION_RECORD,
            &canonical,
        )
        .unwrap();
        revocation.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: sig,
        });
    }

    fn signed_revocation_record(key: &edgerun_crypto::p256::ecdsa::SigningKey) -> RevocationRecord {
        let node_id = test_node_id(key);
        let mut revocation = RevocationRecord {
            record_version: 1,
            revocation_id: b"revocation-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: node_id.clone(),
                identity_kind: Some(1),
                key_hint: Some(node_id),
            }),
            issued_at: Some(prost_types::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            effective_at: Some(prost_types::Timestamp {
                seconds: 1_700_000_001,
                nanos: 0,
            }),
            revocation_kind: edgerun_proto::edgerun::v0::trust::RevocationKind::Delegation as i32,
            target: Some(
                edgerun_proto::edgerun::v0::trust::revocation_record::Target::TargetDelegation(
                    crate::protocol::DelegationRef {
                        delegation_id: b"delegation-1".to_vec(),
                        delegation_hash: Some(crate::protocol::Digest {
                            algorithm: 1,
                            value: [7u8; 32].to_vec(),
                        }),
                    },
                ),
            ),
            scope_override: None,
            reason_code: "compromised".into(),
            replacement_id: vec![],
            revocation_metadata: None,
            signature: None,
        };
        sign_revocation_record(&mut revocation, key);
        revocation
    }

    #[test]
    fn assurance_claim_valid_is_accepted() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let claim = signed_assurance_claim(&key);
        let trusted = vec![claim.attester.as_ref().unwrap().identity_id.clone()];

        let result = validate_assurance_claim(&claim, 1_710_000_000_000, &trusted);

        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn assurance_claim_invalid_issued_at_timestamp_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut claim = signed_assurance_claim(&key);
        claim.issued_at = Some(prost_types::Timestamp {
            seconds: 1_700_000_000,
            nanos: 1_000_000_000,
        });
        sign_assurance_claim(&mut claim, &key);

        let result = validate_assurance_claim(&claim, 1_710_000_000_000, &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn assurance_claim_bad_signature_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut claim = signed_assurance_claim(&key);
        claim.claim_note.push_str("-tampered");

        let result = validate_assurance_claim(&claim, 1_710_000_000_000, &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
    }

    #[test]
    fn assurance_claim_untrusted_attester_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let claim = signed_assurance_claim(&key);

        let result = validate_assurance_claim(&claim, 1_710_000_000_000, &[b"other".to_vec()]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn assurance_claim_empty_subject_identity_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut claim = signed_assurance_claim(&key);
        claim.subject = Some(
            edgerun_proto::edgerun::v0::trust::assurance_claim::Subject::SubjectIdentity(
                IdentityRef {
                    identity_id: vec![],
                    identity_kind: Some(1),
                    key_hint: None,
                },
            ),
        );
        sign_assurance_claim(&mut claim, &key);

        let result = validate_assurance_claim(&claim, 1_710_000_000_000, &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn assurance_claim_empty_evidence_object_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut claim = signed_assurance_claim(&key);
        claim.evidence_object = Some(ObjectRef {
            object_id: vec![],
            object_kind: None,
        });
        sign_assurance_claim(&mut claim, &key);

        let result = validate_assurance_claim(&claim, 1_710_000_000_000, &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    fn assurance_requirement_for_claim(
        claim: &AssuranceClaim,
        required_class: edgerun_proto::edgerun::v0::common::AssuranceClass,
        max_evidence_age: Option<prost_types::Duration>,
    ) -> AssuranceRequirement {
        AssuranceRequirement {
            assurance_version: 1,
            required_class: required_class as i32,
            acceptable_attesters: vec![claim.attester.as_ref().unwrap().clone()],
            max_evidence_age,
            assurance_metadata: None,
        }
    }

    #[test]
    fn assurance_claim_satisfies_requirement() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let claim = signed_assurance_claim(&key);
        let requirement = assurance_requirement_for_claim(
            &claim,
            edgerun_proto::edgerun::v0::common::AssuranceClass::HardwareBacked,
            Some(prost_types::Duration {
                seconds: 20_000_000,
                nanos: 0,
            }),
        );

        let result =
            validate_assurance_claim_satisfies_requirement(&claim, &requirement, 1_710_000_000_000);

        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn assurance_claim_requirement_rejects_missing_expires_at() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut claim = signed_assurance_claim(&key);
        claim.expires_at = None;
        sign_assurance_claim(&mut claim, &key);
        let requirement = assurance_requirement_for_claim(
            &claim,
            edgerun_proto::edgerun::v0::common::AssuranceClass::HardwareBacked,
            None,
        );

        let result =
            validate_assurance_claim_satisfies_requirement(&claim, &requirement, 1_710_000_000_000);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::TimeInvalid));
    }

    #[test]
    fn assurance_claim_requirement_rejects_stale_evidence() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let claim = signed_assurance_claim(&key);
        let requirement = assurance_requirement_for_claim(
            &claim,
            edgerun_proto::edgerun::v0::common::AssuranceClass::HardwareBacked,
            Some(prost_types::Duration {
                seconds: 1,
                nanos: 0,
            }),
        );

        let result =
            validate_assurance_claim_satisfies_requirement(&claim, &requirement, 1_710_000_000_000);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::TimeInvalid));
    }

    #[test]
    fn assurance_claim_requirement_rejects_negative_nanos_max_age() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let claim = signed_assurance_claim(&key);
        let requirement = assurance_requirement_for_claim(
            &claim,
            edgerun_proto::edgerun::v0::common::AssuranceClass::HardwareBacked,
            Some(prost_types::Duration {
                seconds: 0,
                nanos: -1,
            }),
        );

        let result =
            validate_assurance_claim_satisfies_requirement(&claim, &requirement, 1_700_000_000_000);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn assurance_claim_requirement_rejects_out_of_range_max_age_nanos() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let claim = signed_assurance_claim(&key);
        let requirement = assurance_requirement_for_claim(
            &claim,
            edgerun_proto::edgerun::v0::common::AssuranceClass::HardwareBacked,
            Some(prost_types::Duration {
                seconds: 1,
                nanos: 1_000_000_000,
            }),
        );

        let result =
            validate_assurance_claim_satisfies_requirement(&claim, &requirement, 1_700_000_000_000);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn assurance_claim_requirement_rejects_insufficient_class() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let claim = signed_assurance_claim(&key);
        let requirement = assurance_requirement_for_claim(
            &claim,
            edgerun_proto::edgerun::v0::common::AssuranceClass::AttestedRuntime,
            None,
        );

        let result =
            validate_assurance_claim_satisfies_requirement(&claim, &requirement, 1_710_000_000_000);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AssuranceInsufficient));
    }

    #[test]
    fn assurance_claim_requirement_rejects_unacceptable_attester() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let claim = signed_assurance_claim(&key);
        let mut requirement = assurance_requirement_for_claim(
            &claim,
            edgerun_proto::edgerun::v0::common::AssuranceClass::HardwareBacked,
            None,
        );
        requirement.acceptable_attesters = vec![IdentityRef {
            identity_id: b"other".to_vec(),
            identity_kind: Some(1),
            key_hint: None,
        }];

        let result =
            validate_assurance_claim_satisfies_requirement(&claim, &requirement, 1_710_000_000_000);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn assurance_requirement_rejects_empty_acceptable_attester() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let claim = signed_assurance_claim(&key);
        let mut requirement = assurance_requirement_for_claim(
            &claim,
            edgerun_proto::edgerun::v0::common::AssuranceClass::HardwareBacked,
            None,
        );
        requirement.acceptable_attesters = vec![IdentityRef {
            identity_id: vec![],
            identity_kind: Some(1),
            key_hint: None,
        }];

        let result =
            validate_assurance_claim_satisfies_requirement(&claim, &requirement, 1_710_000_000_000);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn assurance_requirement_rejects_empty_metadata_object() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let claim = signed_assurance_claim(&key);
        let mut requirement = assurance_requirement_for_claim(
            &claim,
            edgerun_proto::edgerun::v0::common::AssuranceClass::HardwareBacked,
            None,
        );
        requirement.assurance_metadata = Some(ObjectRef {
            object_id: vec![],
            object_kind: None,
        });

        let result =
            validate_assurance_claim_satisfies_requirement(&claim, &requirement, 1_710_000_000_000);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn revocation_record_valid_is_accepted() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[43u8; 32].into()).unwrap();
        let revocation = signed_revocation_record(&key);
        let trusted = vec![revocation.issuer.as_ref().unwrap().identity_id.clone()];

        let result = validate_revocation_record(&revocation, 1_710_000_000_000, &trusted);

        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn revocation_record_bad_signature_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[43u8; 32].into()).unwrap();
        let mut revocation = signed_revocation_record(&key);
        if let Some(sig) = &mut revocation.signature {
            sig.value[0] ^= 0xFF;
        }

        let result = validate_revocation_record(&revocation, 1_710_000_000_000, &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
    }

    #[test]
    fn revocation_record_not_yet_effective_defers() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[43u8; 32].into()).unwrap();
        let revocation = signed_revocation_record(&key);

        let result = validate_revocation_record(&revocation, 1_700_000_000_500, &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Defer);
        assert_eq!(result.reason_code, Some(ReasonCode::TimeInvalid));
    }

    #[test]
    fn revocation_scope_override_requires_delegation_target() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[43u8; 32].into()).unwrap();
        let mut revocation = signed_revocation_record(&key);
        revocation.target = Some(
            edgerun_proto::edgerun::v0::trust::revocation_record::Target::TargetIdentity(
                IdentityRef {
                    identity_id: b"user-a".to_vec(),
                    identity_kind: Some(1),
                    key_hint: None,
                },
            ),
        );
        revocation.scope_override = Some(crate::protocol::ScopeDescriptor {
            scope_version: 1,
            scope_kind: edgerun_proto::edgerun::v0::trust::ScopeKind::Node as i32,
            target_nodes: vec![NodeRef {
                node_id: b"node-a".to_vec(),
            }],
            target_streams: vec![],
            target_object_kinds: vec![],
            target_view_types: vec![],
            target_domains: vec![],
            time_bounds: None,
            scope_metadata: None,
        });
        sign_revocation_record(&mut revocation, &key);

        let result = validate_revocation_record(&revocation, 1_710_000_000_000, &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn revocation_target_delegation_requires_hash() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[43u8; 32].into()).unwrap();
        let mut revocation = signed_revocation_record(&key);
        if let Some(
            edgerun_proto::edgerun::v0::trust::revocation_record::Target::TargetDelegation(
                delegation,
            ),
        ) = &mut revocation.target
        {
            delegation.delegation_hash = None;
        }
        sign_revocation_record(&mut revocation, &key);

        let result = validate_revocation_record(&revocation, 1_710_000_000_000, &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn revocation_target_identity_requires_identity_id() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[43u8; 32].into()).unwrap();
        let mut revocation = signed_revocation_record(&key);
        revocation.target = Some(
            edgerun_proto::edgerun::v0::trust::revocation_record::Target::TargetIdentity(
                IdentityRef {
                    identity_id: vec![],
                    identity_kind: Some(1),
                    key_hint: None,
                },
            ),
        );
        sign_revocation_record(&mut revocation, &key);

        let result = validate_revocation_record(&revocation, 1_710_000_000_000, &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn revocation_kind_must_match_target_family() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[43u8; 32].into()).unwrap();
        let mut revocation = signed_revocation_record(&key);
        revocation.target = Some(
            edgerun_proto::edgerun::v0::trust::revocation_record::Target::TargetIdentity(
                IdentityRef {
                    identity_id: b"user-a".to_vec(),
                    identity_kind: Some(1),
                    key_hint: None,
                },
            ),
        );
        sign_revocation_record(&mut revocation, &key);

        let result = validate_revocation_record(&revocation, 1_710_000_000_000, &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn identity_trust_revocation_accepts_identity_target() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[43u8; 32].into()).unwrap();
        let mut revocation = signed_revocation_record(&key);
        revocation.revocation_kind =
            edgerun_proto::edgerun::v0::trust::RevocationKind::IdentityTrust as i32;
        revocation.target = Some(
            edgerun_proto::edgerun::v0::trust::revocation_record::Target::TargetIdentity(
                IdentityRef {
                    identity_id: b"user-a".to_vec(),
                    identity_kind: Some(1),
                    key_hint: None,
                },
            ),
        );
        sign_revocation_record(&mut revocation, &key);

        let result = validate_revocation_record(&revocation, 1_710_000_000_000, &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn revocation_scope_override_requires_valid_scope() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[43u8; 32].into()).unwrap();
        let mut revocation = signed_revocation_record(&key);
        revocation.scope_override = Some(crate::protocol::ScopeDescriptor {
            scope_version: 0,
            scope_kind: edgerun_proto::edgerun::v0::trust::ScopeKind::Node as i32,
            target_nodes: vec![NodeRef {
                node_id: b"node-a".to_vec(),
            }],
            target_streams: vec![],
            target_object_kinds: vec![],
            target_view_types: vec![],
            target_domains: vec![],
            time_bounds: None,
            scope_metadata: None,
        });
        sign_revocation_record(&mut revocation, &key);

        let result = validate_revocation_record(&revocation, 1_710_000_000_000, &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::VersionUnsupported));
    }

    // ------------------------------------------------------------------
    // Event signature verification — regression tests to catch no-op
    // verify_event_signature implementations.
    // ------------------------------------------------------------------

    /// Signs an event with a real ECDSA P-256 key and attaches the signature.
    fn sign_event_envelope(
        event: &EventEnvelope,
        signing_key: &edgerun_crypto::p256::ecdsa::SigningKey,
    ) -> EventEnvelope {
        let record = ProtocolRecord::EventEnvelope(event.clone());
        let canonical = canonical_bytes(&record, true);
        let record_hash =
            crate::crypto::record_hash(crate::crypto::HASH_DOMAIN_EVENT_ENVELOPE, &canonical);

        let sig = crate::crypto::sign_record(
            signing_key,
            crate::crypto::SIG_DOMAIN_EVENT_ENVELOPE,
            &record_hash,
        )
        .expect("signing failed");

        let mut event = event.clone();
        event.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: sig,
        });
        event
    }

    #[test]
    fn event_signature_valid_is_accepted() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let verifying_key = signing_key.verifying_key();
        let node_id = crate::crypto::verifying_key_to_node_id(verifying_key);

        let genesis = make_genesis_event();
        let signed = sign_event_envelope(&genesis, &signing_key);

        let result = validate_stream_append(&signed, None, Some(&node_id));
        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn event_signature_invalid_is_rejected() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let verifying_key = signing_key.verifying_key();
        let node_id = crate::crypto::verifying_key_to_node_id(verifying_key);

        let genesis = make_genesis_event();
        let mut signed = sign_event_envelope(&genesis, &signing_key);

        // Tamper with one byte of the signature
        if let Some(ref mut sig) = signed.signature {
            sig.value[0] ^= 0xFF;
        }

        let result = validate_stream_append(&signed, None, Some(&node_id));
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
    }

    #[test]
    fn event_signature_wrong_key_is_rejected() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let verifying_key = signing_key.verifying_key();
        let node_id = crate::crypto::verifying_key_to_node_id(verifying_key);

        let other_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[99u8; 32].into()).unwrap();

        let genesis = make_genesis_event();
        let signed = sign_event_envelope(&genesis, &other_key);

        // Verify with the WRONG key
        let result = validate_stream_append(&signed, None, Some(&node_id));
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
    }

    #[test]
    fn event_signature_bogus_bytes_are_rejected() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let verifying_key = signing_key.verifying_key();
        let node_id = crate::crypto::verifying_key_to_node_id(verifying_key);

        let mut event = make_genesis_event();
        // Put garbage in the signature value — this must NOT be accepted.
        event.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: vec![0xDE; 64],
        });

        let result = validate_stream_append(&event, None, Some(&node_id));
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
    }
}
