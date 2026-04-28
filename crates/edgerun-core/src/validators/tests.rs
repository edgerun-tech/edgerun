use crate::prelude::v1::*;

use super::helpers::*;
use super::*;
use crate::crypto::sha256;
use crate::value::yi64;

struct TestVerifier;

impl FixtureVerifier for TestVerifier {
    fn verify_signed_fixture(
        &self,
        payload: &BTreeMap<String, Value>,
        expected_fixture: &str,
    ) -> Option<bool> {
        match payload.get("signature_valid").and_then(Value::as_bool) {
            Some(v) => Some(v),
            None if expected_fixture.is_empty() => None,
            None => Some(true),
        }
    }
}

fn no_hash(_: &BTreeMap<String, Value>) -> Option<String> {
    None
}

#[test]
fn trust_revocation_future_is_deferred() {
    let semantic = match mapping([(
        "revocation_record",
        mapping([
            ("issuer", ystr("root")),
            ("target_delegation", ystr("del-1")),
            ("effective_at", ystr("2030-01-02T00:00:00Z")),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([
        ("now", ystr("2030-01-01T00:00:00Z")),
        ("trust_roots", seq([ystr("root")])),
    ]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_trust_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Defer);
    assert_eq!(result.reason_code, Some(ReasonCode::MissingDependency));
}

#[test]
fn query_result_fragment_needs_backing_refs() {
    let semantic = match mapping([(
        "result_fragment",
        mapping([("query_id", ystr("q1")), ("responder", ystr("node"))]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([("current_query_id", ystr("q1"))]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_query_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
}

#[test]
fn network_relay_requires_exactly_one_payload_form() {
    let semantic = match mapping([(
        "relay_envelope",
        mapping([
            ("original_sender", ystr("user")),
            ("intended_recipient_node", ystr("node-a")),
            ("inline_payload", ystr("abc")),
            ("payload_object", mapping([("object_id", ystr("obj-1"))])),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([
        ("now", ystr("2030-01-01T00:00:00Z")),
        ("local_node", ystr("node-a")),
    ]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_network_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
}

#[test]
fn stream_action_completed_requires_result_object() {
    let semantic = match mapping([
        (
            "candidate_event",
            mapping([
                ("event_type", ystr("EVENT_TYPE_ACTION_COMPLETED")),
                ("seq", yi64(1)),
                ("stream_id", ystr("node-stream")),
            ]),
        ),
        (
            "action_lifecycle_payload",
            mapping([
                ("origin_command", mapping([("command_id", ystr("cmd"))])),
                ("action_instance_id", ystr("act-1")),
                ("status", ystr("ACTION_STATUS_COMPLETED")),
            ]),
        ),
    ]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([(
        "stream_heads",
        mapping([(
            "node-stream",
            mapping([("seq", yi64(0)), ("event_hash_hex", ystr("0x00"))]),
        )]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_stream_append_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
}

#[test]
fn object_descriptor_and_header_must_match_object_id() {
    let semantic = match mapping([
        ("descriptor", mapping([("object_id", ystr("obj-a"))])),
        (
            "header",
            mapping([
                ("stored_size", yi64(5)),
                ("object", mapping([("object_id", ystr("obj-b"))])),
            ]),
        ),
    ]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = BTreeMap::new();
    let result = validate_object_case(&semantic, &state);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::ObjectIdMismatch));
}

#[test]
fn trust_assurance_insufficient_rejected() {
    let semantic = match mapping([(
        "assurance_claim",
        mapping([
            ("subject_identity", ystr("user-bob")),
            ("attester", ystr("user-alice")),
            ("assurance_class", ystr("ASSURANCE_CLASS_SOFTWARE")),
            ("issued_at", ystr("2030-01-01T00:00:00Z")),
            ("expires_at", ystr("2030-01-02T00:00:00Z")),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([
        ("now", ystr("2030-01-01T12:00:00Z")),
        ("acceptable_attesters", seq([ystr("user-alice")])),
        (
            "minimum_assurance_class",
            ystr("ASSURANCE_CLASS_HARDWARE_BACKED"),
        ),
    ]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_trust_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::AssuranceInsufficient));
}

#[test]
fn query_proof_bundle_allowed_is_advisory_accept() {
    let semantic = match mapping([(
        "proof_bundle",
        mapping([
            ("bundle_version", yi64(1)),
            ("source_query_id", ystr("q-current")),
            ("payload_type", ystr("PROOF_PAYLOAD_TYPE_STREAM_HEADS")),
            ("payload_object", mapping([("object_id", ystr("obj-1"))])),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([
        ("current_query_id", ystr("q-current")),
        (
            "allowed_proof_payload_types",
            seq([ystr("PROOF_PAYLOAD_TYPE_STREAM_HEADS")]),
        ),
    ]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_query_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Accept);
    assert_eq!(result.reason_code, None);
}

#[test]
fn network_session_accept_nonce_mismatch_rejected() {
    let semantic = match mapping([(
        "session_accept",
        mapping([
            ("responder", ystr("node-server")),
            ("echoed_session_nonce", ystr("nonce-b")),
            ("selected_protocol_version", yi64(1)),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([
        ("now", ystr("2030-01-01T00:00:00Z")),
        ("expected_session_nonce", ystr("nonce-a")),
        ("supported_protocol_versions", seq([yi64(1)])),
    ]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_network_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::TargetMismatch));
}

#[test]
fn object_descriptor_header_manifest_consistent_accepts() {
    let semantic = match mapping([
        ("descriptor", mapping([("object_id", ystr("obj-a"))])),
        (
            "header",
            mapping([
                ("representation_id", ystr("rep-1")),
                ("stored_size", yi64(5)),
                ("chunking_mode", ystr("CHUNKING_MODE_MANIFEST")),
                ("object", mapping([("object_id", ystr("obj-a"))])),
            ]),
        ),
        (
            "chunk_manifest",
            mapping([
                ("chunk_count", yi64(1)),
                ("total_stored_size", yi64(5)),
                ("object", mapping([("object_id", ystr("obj-a"))])),
                (
                    "representation",
                    mapping([("representation_id", ystr("rep-1"))]),
                ),
                (
                    "entries",
                    seq([mapping([
                        ("representation_id", ystr("rep-1")),
                        ("length", yi64(5)),
                    ])]),
                ),
            ]),
        ),
    ]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = BTreeMap::new();
    let result = validate_object_case(&semantic, &state);
    assert_eq!(result.verdict, Verdict::Accept);
    assert_eq!(result.reason_code, None);
}

#[test]
fn trust_assurance_bad_signature_rejected() {
    let semantic = match mapping([(
        "assurance_claim",
        mapping([
            ("subject_identity", ystr("alice")),
            ("attester", ystr("attester-a")),
            ("assurance_class", ystr("ASSURANCE_CLASS_SOFTWARE")),
            ("signature_valid", Value::Bool(false)),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([("acceptable_attesters", seq([ystr("attester-a")]))]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_trust_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
}

#[test]
fn query_result_fragment_with_backing_is_advisory_accept() {
    let semantic = match mapping([(
        "result_fragment",
        mapping([
            ("query_id", ystr("q1")),
            ("responder", ystr("node-a")),
            ("completeness", ystr("RESULT_COMPLETENESS_PARTIAL")),
            (
                "object_refs",
                seq([mapping([("object_id", ystr("obj-1"))])]),
            ),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([("current_query_id", ystr("q1"))]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_query_case(&semantic, &state, &TestVerifier, &no_hash);
    let derived = result.derived.as_map().expect("derived map");
    assert_eq!(result.verdict, Verdict::Accept);
    assert_eq!(
        derived.get("decision").and_then(Value::as_str),
        Some("result_fragment_received")
    );
}

#[test]
fn query_request_missing_query_id_is_rejected() {
    let semantic = match mapping([(
        "query",
        mapping([
            ("requester", ystr("node-a")),
            ("query_class", ystr("QUERY_CLASS_HEAD")),
            ("target_scope", ystr("node:node-a")),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([("current_controller_set", seq([ystr("node-a")]))]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_query_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
}

#[test]
fn query_request_missing_target_scope_is_rejected() {
    let semantic = match mapping([(
        "query",
        mapping([
            ("requester", ystr("node-a")),
            ("query_id", ystr("query-1")),
            ("query_class", ystr("QUERY_CLASS_HEAD")),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([("current_controller_set", seq([ystr("node-a")]))]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_query_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
}

#[test]
fn query_request_zero_result_limit_is_rejected() {
    let semantic = match mapping([(
        "query",
        mapping([
            ("requester", ystr("node-a")),
            ("query_id", ystr("query-1")),
            ("query_class", ystr("QUERY_CLASS_HEAD")),
            ("target_scope", ystr("node:node-a")),
            ("result_limit", yi64(0)),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([("current_controller_set", seq([ystr("node-a")]))]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_query_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
}

#[test]
fn network_reachability_hint_valid_accepts() {
    let semantic = match mapping([(
        "reachability_hint",
        mapping([
            ("hint_version", Value::Int(1)),
            ("subject_node", ystr("node-a")),
            ("transport_class", ystr("TRANSPORT_CLASS_QUIC")),
            ("locator_payload", ystr("quic://127.0.0.1:443")),
            ("directness", ystr("DIRECTNESS_DIRECT")),
            ("valid_after", ystr("2030-01-01T00:00:00Z")),
            ("valid_until", ystr("2030-01-01T00:05:00Z")),
            ("issuer", mapping([("fixture", ystr("node-a"))])),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([("now", ystr("2030-01-01T00:01:00Z"))]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_network_case(&semantic, &state, &TestVerifier, &no_hash);
    let derived = result.derived.as_map().expect("derived map");
    assert_eq!(result.verdict, Verdict::Accept);
    assert_eq!(
        derived.get("decision").and_then(Value::as_str),
        Some("reachability_hint_accepted")
    );
}

#[test]
fn object_chunked_representation_missing_chunk_is_deferred() {
    let semantic = match mapping([(
        "representation",
        mapping([(
            "chunk_manifest",
            mapping([(
                "entries",
                seq([mapping([
                    ("chunk_digest", ystr("deadbeef")),
                    ("length", yi64(3)),
                ])]),
            )]),
        )]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = BTreeMap::new();
    let result = validate_object_case(&semantic, &state);
    let derived = result.derived.as_map().expect("derived map");
    assert_eq!(result.verdict, Verdict::Defer);
    assert_eq!(result.reason_code, Some(ReasonCode::MissingDependency));
    assert_eq!(
        derived.get("validation_level").and_then(Value::as_str),
        Some("deferred_missing_chunks")
    );
}

#[test]
fn object_chunked_representation_valid_accepts() {
    let chunk_bytes = vec![0x61, 0x62, 0x63];
    let mut chunk_payload = b"edgerun:v0:chunk-bytes\0".to_vec();
    chunk_payload.extend_from_slice(&chunk_bytes);
    let chunk_digest = bytes_to_hex(&sha256(&chunk_payload));

    let mut rep_payload = b"edgerun:v0:representation-bytes\0".to_vec();
    rep_payload.extend_from_slice(&chunk_bytes);
    let representation_digest = bytes_to_hex(&sha256(&rep_payload));

    let semantic = match mapping([(
        "representation",
        mapping([
            (
                "header",
                mapping([("representation_digest", ystr(&representation_digest))]),
            ),
            (
                "chunk_manifest",
                mapping([(
                    "entries",
                    seq([mapping([
                        ("chunk_digest", ystr(&chunk_digest)),
                        ("length", yi64(3)),
                    ])]),
                )]),
            ),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([(
        "available_chunks",
        mapping([(&chunk_digest, ystr(&bytes_to_hex(&chunk_bytes)))]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_object_case(&semantic, &state);
    let derived = result.derived.as_map().expect("derived map");
    assert_eq!(result.verdict, Verdict::Accept);
    assert_eq!(
        derived.get("representation_digest").and_then(Value::as_str),
        Some(representation_digest.as_str())
    );
    assert_eq!(
        derived.get("validation_level").and_then(Value::as_str),
        Some("representation_valid_only")
    );
}

#[test]
fn control_transfer_with_valid_proof_commits() {
    let semantic = match mapping([(
        "command",
        mapping([
            ("command_type", ystr("TRANSFER_CONTROL")),
            ("command_id", ystr("ctrl-transfer")),
            ("issuer", ystr("controller-a")),
            ("target_node", ystr("node-a")),
            ("from", ystr("controller-a")),
            ("to", ystr("controller-b")),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([
        ("local_node", ystr("node-a")),
        ("current_controller_set", seq([ystr("controller-a")])),
        (
            "control_policy",
            mapping([("minimum_controllers", yi64(1))]),
        ),
        (
            "proof_of_possession",
            mapping([("controller-b", ystr("valid-challenge-response"))]),
        ),
    ]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_control_change_case(&semantic, &state, &TestVerifier);
    let derived = result.derived.as_map().expect("derived map");
    assert_eq!(result.verdict, Verdict::Accept);
    assert_eq!(
        derived.get("control_delta").and_then(Value::as_str),
        Some("transfer_control")
    );
}

#[test]
fn snapshot_matching_local_head_is_trusted_accept() {
    let semantic = match mapping([(
        "snapshot",
        mapping([
            ("producer", ystr("node-a")),
            (
                "base_heads",
                seq([mapping([
                    ("stream_id", ystr("stream-1")),
                    ("seq", yi64(3)),
                    ("event_hash_hex", ystr("hash-3")),
                ])]),
            ),
            ("payload_object", ystr("snapshot-payload")),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([
        ("trusted_snapshot_producers", seq([ystr("node-a")])),
        (
            "stream_heads",
            mapping([(
                "stream-1",
                mapping([("seq", yi64(3)), ("event_hash_hex", ystr("hash-3"))]),
            )]),
        ),
    ]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_snapshot_case(&semantic, &state, &TestVerifier, &no_hash);
    let derived = result.derived.as_map().expect("derived map");
    assert_eq!(result.verdict, Verdict::Accept);
    assert_eq!(
        derived.get("acceptance_class").and_then(Value::as_str),
        Some("accepted_trusted")
    );
}

#[test]
fn command_non_controller_without_delegation_is_rejected() {
    let semantic = match mapping([(
        "command",
        mapping([
            ("issuer", ystr("delegate-x")),
            ("target_node", ystr("node-a")),
            ("command_type", ystr("QUERY")),
            ("command_id", ystr("cmd-1")),
            ("issued_at", ystr("2030-01-01T00:00:00Z")),
            ("signature", Value::Map(BTreeMap::new())),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([
        ("current_controller_set", seq([ystr("controller-a")])),
        ("local_node", ystr("node-a")),
    ]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_command_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
}

#[test]
fn command_replay_same_hash_is_duplicate() {
    let semantic = match mapping([(
        "command",
        mapping([
            ("issuer", ystr("controller-a")),
            ("target_node", ystr("node-a")),
            ("command_type", ystr("QUERY")),
            ("command_id", ystr("cmd-1")),
            ("command_hash_hex", ystr("hash-1")),
            ("issued_at", ystr("2030-01-01T00:00:00Z")),
            ("signature", Value::Map(BTreeMap::new())),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    // Replay cache is keyed by command_hash
    let state = match mapping([
        ("current_controller_set", seq([ystr("controller-a")])),
        ("local_node", ystr("node-a")),
        ("replay_cache", mapping([("hash-1", ystr("cmd-1"))])),
    ]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_command_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Duplicate);
    assert_eq!(result.reason_code, Some(ReasonCode::ReplayDetected));
}

#[test]
fn command_different_hash_same_id_is_accepted() {
    // command_id is only an idempotency hint — different hash means different command
    let semantic = match mapping([(
        "command",
        mapping([
            ("issuer", ystr("controller-a")),
            ("target_node", ystr("node-a")),
            ("command_type", ystr("QUERY")),
            ("command_id", ystr("cmd-1")),
            ("command_hash_hex", ystr("hash-2")),
            ("issued_at", ystr("2030-01-01T00:00:00Z")),
            ("signature", Value::Map(BTreeMap::new())),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    // Replay cache has a different hash for the same command_id — should not affect this command
    let state = match mapping([
        ("current_controller_set", seq([ystr("controller-a")])),
        ("local_node", ystr("node-a")),
        ("replay_cache", mapping([("hash-1", ystr("cmd-1"))])),
    ]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_command_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Accept);
}

#[test]
fn snapshot_stale_missing_delta_is_deferred() {
    let semantic = match mapping([(
        "snapshot",
        mapping([
            ("producer", ystr("node-a")),
            (
                "base_heads",
                seq([mapping([
                    ("stream_id", ystr("stream-1")),
                    ("seq", yi64(2)),
                    ("event_hash_hex", ystr("hash-2")),
                ])]),
            ),
            ("payload_object", ystr("snapshot-payload")),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([
        ("trusted_snapshot_producers", seq([ystr("node-a")])),
        (
            "stream_heads",
            mapping([(
                "stream-1",
                mapping([("seq", yi64(4)), ("event_hash_hex", ystr("hash-4"))]),
            )]),
        ),
        ("available_deltas", seq([mapping([("seq", yi64(3))])])),
    ]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_snapshot_case(&semantic, &state, &TestVerifier, &no_hash);
    let derived = result.derived.as_map().expect("derived map");
    assert_eq!(result.verdict, Verdict::Defer);
    assert_eq!(result.reason_code, Some(ReasonCode::MissingDependency));
    assert_eq!(
        derived.get("acceptance_class").and_then(Value::as_str),
        Some("deferred_missing_delta")
    );
}

#[test]
fn snapshot_stale_with_full_deltas_is_accepted_stale() {
    let semantic = match mapping([(
        "snapshot",
        mapping([
            ("producer", ystr("node-a")),
            (
                "base_heads",
                seq([mapping([
                    ("stream_id", ystr("stream-1")),
                    ("seq", yi64(2)),
                    ("event_hash_hex", ystr("hash-2")),
                ])]),
            ),
            ("payload_object", ystr("snapshot-payload")),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([
        ("trusted_snapshot_producers", seq([ystr("node-a")])),
        (
            "stream_heads",
            mapping([(
                "stream-1",
                mapping([("seq", yi64(4)), ("event_hash_hex", ystr("hash-4"))]),
            )]),
        ),
        (
            "available_deltas",
            seq([mapping([("seq", yi64(3))]), mapping([("seq", yi64(4))])]),
        ),
    ]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_snapshot_case(&semantic, &state, &TestVerifier, &no_hash);
    let derived = result.derived.as_map().expect("derived map");
    assert_eq!(result.verdict, Verdict::Accept);
    assert_eq!(
        derived.get("acceptance_class").and_then(Value::as_str),
        Some("accepted_stale")
    );
}

#[test]
fn snapshot_checkpoint_base_with_head_is_accepted() {
    let semantic = match mapping([(
        "snapshot",
        mapping([
            ("producer", ystr("node-a")),
            (
                "base_checkpoints",
                seq([mapping([
                    ("checkpoint_id", ystr("checkpoint-1")),
                    (
                        "heads",
                        seq([mapping([
                            ("stream_id", ystr("stream-1")),
                            ("seq", yi64(3)),
                            ("event_hash_hex", ystr("hash-3")),
                        ])]),
                    ),
                ])]),
            ),
            ("payload_object", ystr("snapshot-payload")),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([
        ("trusted_snapshot_producers", seq([ystr("node-a")])),
        (
            "stream_heads",
            mapping([(
                "stream-1",
                mapping([("seq", yi64(3)), ("event_hash_hex", ystr("hash-3"))]),
            )]),
        ),
    ]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_snapshot_case(&semantic, &state, &TestVerifier, &no_hash);
    let derived = result.derived.as_map().expect("derived map");
    assert_eq!(result.verdict, Verdict::Accept);
    assert_eq!(
        derived.get("acceptance_class").and_then(Value::as_str),
        Some("accepted_trusted")
    );
}

#[test]
fn snapshot_checkpoint_id_without_heads_defers_resolution() {
    let semantic = match mapping([(
        "snapshot",
        mapping([
            ("producer", ystr("node-a")),
            (
                "base_checkpoints",
                seq([mapping([("checkpoint_id", ystr("checkpoint-1"))])]),
            ),
            ("payload_object", ystr("snapshot-payload")),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([("trusted_snapshot_producers", seq([ystr("node-a")]))]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_snapshot_case(&semantic, &state, &TestVerifier, &no_hash);
    let derived = result.derived.as_map().expect("derived map");
    assert_eq!(result.verdict, Verdict::Defer);
    assert_eq!(result.reason_code, Some(ReasonCode::MissingDependency));
    assert_eq!(
        derived.get("acceptance_class").and_then(Value::as_str),
        Some("deferred_missing_checkpoint")
    );
}

#[test]
fn snapshot_missing_payload_object_is_rejected() {
    let semantic = match mapping([(
        "snapshot",
        mapping([
            ("producer", ystr("node-a")),
            (
                "base_heads",
                seq([mapping([
                    ("stream_id", ystr("stream-1")),
                    ("seq", yi64(3)),
                    ("event_hash_hex", ystr("hash-3")),
                ])]),
            ),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([("trusted_snapshot_producers", seq([ystr("node-a")]))]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_snapshot_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
}

#[test]
fn trust_assurance_issued_in_future_rejected() {
    let semantic = match mapping([(
        "assurance_claim",
        mapping([
            ("subject_identity", ystr("user-bob")),
            ("attester", ystr("user-alice")),
            ("assurance_class", ystr("ASSURANCE_CLASS_SOFTWARE")),
            ("issued_at", ystr("2030-01-02T00:00:00Z")),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([
        ("now", ystr("2030-01-01T12:00:00Z")),
        ("acceptable_attesters", seq([ystr("user-alice")])),
    ]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_trust_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::TimeInvalid));
}

#[test]
fn query_aggregate_mismatched_query_id_rejected() {
    let semantic = match mapping([(
        "aggregate_descriptor",
        mapping([
            ("source_query_id", ystr("q-other")),
            ("aggregator", ystr("node-a")),
            (
                "input_fragments",
                seq([mapping([("object_id", ystr("frag-1"))])]),
            ),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([("current_query_id", ystr("q-current"))]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_query_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::TargetMismatch));
}

#[test]
fn query_proof_bundle_mismatched_query_id_rejected() {
    let semantic = match mapping([(
        "proof_bundle",
        mapping([
            ("bundle_version", yi64(1)),
            ("source_query_id", ystr("q-other")),
            ("payload_type", ystr("PROOF_PAYLOAD_TYPE_STREAM_HEADS")),
            ("payload_object", mapping([("object_id", ystr("obj-1"))])),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([
        ("current_query_id", ystr("q-current")),
        (
            "allowed_proof_payload_types",
            seq([ystr("PROOF_PAYLOAD_TYPE_STREAM_HEADS")]),
        ),
    ]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_query_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::TargetMismatch));
}

#[test]
fn query_proof_bundle_missing_local_payload_is_deferred() {
    let semantic = match mapping([(
        "proof_bundle",
        mapping([
            ("bundle_version", yi64(1)),
            ("source_query_id", ystr("q-current")),
            ("payload_type", ystr("PROOF_PAYLOAD_TYPE_STREAM_HEADS")),
            (
                "payload_object",
                mapping([("object_id", ystr("obj-missing"))]),
            ),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([
        ("current_query_id", ystr("q-current")),
        (
            "allowed_proof_payload_types",
            seq([ystr("PROOF_PAYLOAD_TYPE_STREAM_HEADS")]),
        ),
        (
            "available_objects",
            mapping([("obj-other", ystr("present"))]),
        ),
    ]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_query_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Defer);
    assert_eq!(result.reason_code, Some(ReasonCode::MissingDependency));
}

#[test]
fn query_proof_bundle_missing_source_query_id_is_rejected() {
    let semantic = match mapping([(
        "proof_bundle",
        mapping([
            ("bundle_version", yi64(1)),
            ("payload_type", ystr("PROOF_PAYLOAD_TYPE_STREAM_HEADS")),
            ("payload_object", mapping([("object_id", ystr("obj-1"))])),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([(
        "allowed_proof_payload_types",
        seq([ystr("PROOF_PAYLOAD_TYPE_STREAM_HEADS")]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_query_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
}

#[test]
fn query_proof_bundle_missing_payload_type_is_rejected() {
    let semantic = match mapping([(
        "proof_bundle",
        mapping([
            ("bundle_version", yi64(1)),
            ("source_query_id", ystr("q-current")),
            ("payload_object", mapping([("object_id", ystr("obj-1"))])),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([("current_query_id", ystr("q-current"))]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_query_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
}

#[test]
fn query_proof_bundle_unsupported_version_is_rejected() {
    let semantic = match mapping([(
        "proof_bundle",
        mapping([
            ("bundle_version", yi64(2)),
            ("source_query_id", ystr("q-current")),
            ("payload_type", ystr("PROOF_PAYLOAD_TYPE_STREAM_HEADS")),
            ("payload_object", mapping([("object_id", ystr("obj-1"))])),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([("current_query_id", ystr("q-current"))]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_query_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::VersionUnsupported));
}

#[test]
fn query_snapshot_set_proof_empty_is_rejected() {
    let semantic = match mapping([(
        "snapshot_set_proof",
        mapping([
            ("source_query_id", ystr("q-current")),
            ("snapshots", seq([])),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([("current_query_id", ystr("q-current"))]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_query_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
}

#[test]
fn query_snapshot_set_proof_missing_source_query_id_is_rejected() {
    let semantic = match mapping([(
        "snapshot_set_proof",
        mapping([(
            "snapshots",
            seq([mapping([("snapshot_id", ystr("snap-1"))])]),
        )]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = BTreeMap::new();
    let result = validate_query_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
}

#[test]
fn query_aggregate_descriptor_missing_source_query_id_is_rejected() {
    let semantic = match mapping([(
        "aggregate_descriptor",
        mapping([(
            "input_fragments",
            seq([mapping([("object_id", ystr("frag-1"))])]),
        )]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = BTreeMap::new();
    let result = validate_query_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
}

#[test]
fn query_aggregate_descriptor_missing_payload_object_is_rejected() {
    let semantic = match mapping([(
        "aggregate_descriptor",
        mapping([
            ("aggregate_id", ystr("agg-1")),
            ("source_query_id", ystr("q-current")),
            ("aggregator", ystr("node-a")),
            ("aggregated_at", ystr("2030-01-01T00:00:00Z")),
            (
                "input_fragments",
                seq([mapping([("object_id", ystr("frag-1"))])]),
            ),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([("current_query_id", ystr("q-current"))]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_query_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
}

#[test]
fn query_aggregate_descriptor_empty_input_fragment_is_rejected() {
    let semantic = match mapping([(
        "aggregate_descriptor",
        mapping([
            ("aggregate_id", ystr("agg-1")),
            ("source_query_id", ystr("q-current")),
            ("aggregator", ystr("node-a")),
            ("aggregated_at", ystr("2030-01-01T00:00:00Z")),
            ("input_fragments", seq([mapping([("object_id", ystr(""))])])),
            (
                "payload_object",
                mapping([("object_id", ystr("payload-1"))]),
            ),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([("current_query_id", ystr("q-current"))]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_query_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
}

#[test]
fn query_result_fragment_missing_query_id_is_rejected() {
    let semantic = match mapping([(
        "result_fragment",
        mapping([(
            "object_refs",
            seq([mapping([("object_id", ystr("obj-1"))])]),
        )]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = BTreeMap::new();
    let result = validate_query_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
}

#[test]
fn query_result_fragment_missing_completeness_is_rejected() {
    let semantic = match mapping([(
        "result_fragment",
        mapping([
            ("query_id", ystr("q-current")),
            ("responder", ystr("node-a")),
            (
                "object_refs",
                seq([mapping([("object_id", ystr("obj-1"))])]),
            ),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([("current_query_id", ystr("q-current"))]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_query_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
}

#[test]
fn query_result_fragment_event_ref_missing_seq_is_rejected() {
    let semantic = match mapping([(
        "result_fragment",
        mapping([
            ("query_id", ystr("q-current")),
            ("responder", ystr("node-a")),
            ("completeness", ystr("RESULT_COMPLETENESS_PARTIAL")),
            (
                "event_refs",
                seq([mapping([
                    ("stream_id", ystr("stream-a")),
                    ("event_hash_hex", ystr("hash-1")),
                ])]),
            ),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([("current_query_id", ystr("q-current"))]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_query_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
}

#[test]
fn query_object_assertion_exists_missing_is_deferred() {
    let semantic = match mapping([(
        "object_assertion_proof",
        mapping([
            ("source_query_id", ystr("q-current")),
            ("object_ref", mapping([("object_id", ystr("obj-1"))])),
            ("exists", Value::Bool(true)),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([
        ("current_query_id", ystr("q-current")),
        ("available_objects", mapping([("obj-2", ystr("present"))])),
    ]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_query_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Defer);
    assert_eq!(result.reason_code, Some(ReasonCode::MissingDependency));
}

#[test]
fn query_trust_policy_proof_requires_one_object() {
    let semantic = match mapping([(
        "trust_policy_proof",
        mapping([("source_query_id", ystr("q-current"))]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([("current_query_id", ystr("q-current"))]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_query_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
}

#[test]
fn network_route_trust_assignments_require_trusted_issuer() {
    let semantic = match mapping([(
        "route_trust_assignments",
        mapping([
            ("issuer", ystr("peer-a")),
            (
                "assignments",
                seq([mapping([
                    ("subject", mapping([("identity_id", ystr("node-a"))])),
                    ("trust_score", yi64(5)),
                ])]),
            ),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([("trust_roots", seq([ystr("root-a")]))]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_network_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
}

#[test]
fn network_route_selection_picks_best_eligible_route() {
    let semantic = match mapping([(
        "route_selection_policy",
        mapping([
            ("minimum_quality_hint", yi64(3)),
            ("maximum_cost_hint", yi64(5)),
            ("require_active_session", Value::Bool(true)),
            (
                "preferred_advertisers",
                seq([mapping([("identity_id", ystr("adv-b"))])]),
            ),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([
        (
            "candidate_routes",
            seq([
                mapping([
                    ("advertiser", ystr("adv-a")),
                    ("next_hop_node", ystr("hop-a")),
                    ("quality_hint", yi64(5)),
                    ("cost_hint", yi64(2)),
                    ("has_active_session", Value::Bool(true)),
                    ("advertised_at", ystr("2030-01-01T00:00:02Z")),
                ]),
                mapping([
                    ("advertiser", ystr("adv-b")),
                    ("next_hop_node", ystr("hop-b")),
                    ("quality_hint", yi64(4)),
                    ("cost_hint", yi64(1)),
                    ("has_active_session", Value::Bool(true)),
                    ("advertised_at", ystr("2030-01-01T00:00:03Z")),
                ]),
            ]),
        ),
        (
            "route_assignment_scores",
            mapping([("adv-a", yi64(1)), ("adv-b", yi64(2))]),
        ),
    ]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_network_case(&semantic, &state, &TestVerifier, &no_hash);
    let derived = result.derived.as_map().expect("derived map");
    assert_eq!(result.verdict, Verdict::Accept);
    assert_eq!(
        derived.get("selected_advertiser").and_then(Value::as_str),
        Some("adv-b")
    );
    assert_eq!(
        derived.get("selected_next_hop").and_then(Value::as_str),
        Some("hop-b")
    );
}

#[test]
fn network_route_selection_rejects_when_no_route_meets_policy() {
    let semantic = match mapping([(
        "route_selection_policy",
        mapping([
            ("minimum_quality_hint", yi64(8)),
            ("maximum_cost_hint", yi64(1)),
            ("require_active_session", Value::Bool(true)),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([(
        "candidate_routes",
        seq([mapping([
            ("advertiser", ystr("adv-a")),
            ("next_hop_node", ystr("hop-a")),
            ("quality_hint", yi64(5)),
            ("cost_hint", yi64(2)),
            ("has_active_session", Value::Bool(false)),
        ])]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_network_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::PolicyDenied));
}

#[test]
fn network_route_trust_assignments_from_trust_root_accepts() {
    let semantic = match mapping([(
        "route_trust_assignments",
        mapping([
            ("issuer", ystr("root-a")),
            (
                "assignments",
                seq([mapping([
                    ("subject", mapping([("identity_id", ystr("node-a"))])),
                    ("trust_score", yi64(7)),
                ])]),
            ),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([("trust_roots", seq([ystr("root-a")]))]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_network_case(&semantic, &state, &TestVerifier, &no_hash);
    let derived = result.derived.as_map().expect("derived map");
    assert_eq!(result.verdict, Verdict::Accept);
    assert_eq!(
        derived.get("decision").and_then(Value::as_str),
        Some("route_trust_assignments_accepted")
    );
}

#[test]
fn network_aggregate_trust_policy_from_trust_root_accepts() {
    let semantic = match mapping([(
        "aggregate_trust_policy",
        mapping([("issuer", ystr("root-a")), ("minimum_trust_score", yi64(2))]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([("trust_roots", seq([ystr("root-a")]))]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_network_case(&semantic, &state, &TestVerifier, &no_hash);
    let derived = result.derived.as_map().expect("derived map");
    assert_eq!(result.verdict, Verdict::Accept);
    assert_eq!(
        derived.get("decision").and_then(Value::as_str),
        Some("aggregate_trust_policy_accepted")
    );
}

#[test]
fn network_aggregate_trust_policy_negative_minimum_rejected() {
    let semantic = match mapping([(
        "aggregate_trust_policy",
        mapping([
            ("issuer", ystr("root-a")),
            ("minimum_trust_score", yi64(-1)),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([("trust_roots", seq([ystr("root-a")]))]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_network_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
}

#[test]
fn network_route_trust_assignments_empty_rejected() {
    let semantic = match mapping([(
        "route_trust_assignments",
        mapping([("issuer", ystr("root-a")), ("assignments", seq([]))]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([("trust_roots", seq([ystr("root-a")]))]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_network_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
}

#[test]
fn trust_revocation_scope_override_requires_delegation_target() {
    let semantic = match mapping([(
        "revocation_record",
        mapping([
            ("issuer", ystr("root")),
            ("target_identity", ystr("user-a")),
            ("scope_override", ystr("timeline:abc")),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([("trust_roots", seq([ystr("root")]))]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_trust_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
}

#[test]
fn trust_revocation_scope_override_broader_than_delegation_rejected() {
    let semantic = match mapping([(
        "revocation_record",
        mapping([
            ("issuer", ystr("root")),
            ("target_delegation", ystr("del-1")),
            ("scope_override", ystr("timeline:abc")),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([
        ("trust_roots", seq([ystr("root")])),
        (
            "delegation_scopes",
            mapping([("del-1", ystr("timeline:abc/child"))]),
        ),
    ]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_trust_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
}

#[test]
fn trust_revocation_scope_override_missing_scope_info_is_deferred() {
    let semantic = match mapping([(
        "revocation_record",
        mapping([
            ("issuer", ystr("root")),
            ("target_delegation", ystr("del-1")),
            ("scope_override", ystr("timeline:abc/child")),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([("trust_roots", seq([ystr("root")]))]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_trust_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Defer);
    assert_eq!(result.reason_code, Some(ReasonCode::MissingDependency));
}

#[test]
fn trust_revocation_replacement_target_mismatch_rejected() {
    let semantic = match mapping([(
        "revocation_record",
        mapping([
            ("issuer", ystr("root")),
            ("target_delegation", ystr("del-2")),
            ("replacement_id", ystr("rev-old")),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([
        ("trust_roots", seq([ystr("root")])),
        (
            "existing_revocations",
            mapping([("rev-old", mapping([("target_delegation", ystr("del-1"))]))]),
        ),
    ]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_trust_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
}

#[test]
fn trust_revocation_scope_override_narrower_accepts() {
    let semantic = match mapping([(
        "revocation_record",
        mapping([
            ("issuer", ystr("root")),
            ("target_delegation", ystr("del-1")),
            ("scope_override", ystr("timeline:abc/child")),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([
        ("trust_roots", seq([ystr("root")])),
        (
            "delegation_scopes",
            mapping([("del-1", ystr("timeline:abc"))]),
        ),
    ]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_trust_case(&semantic, &state, &TestVerifier, &no_hash);
    let derived = result.derived.as_map().expect("derived map");
    assert_eq!(result.verdict, Verdict::Accept);
    assert_eq!(
        derived.get("scope_narrowed").and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn trust_revocation_replacement_narrower_scope_accepts() {
    let semantic = match mapping([(
        "revocation_record",
        mapping([
            ("issuer", ystr("root")),
            ("target_delegation", ystr("del-1")),
            ("replacement_id", ystr("rev-old")),
            ("scope_override", ystr("timeline:abc/child")),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([
        ("trust_roots", seq([ystr("root")])),
        (
            "delegation_scopes",
            mapping([("del-1", ystr("timeline:abc"))]),
        ),
        (
            "existing_revocations",
            mapping([(
                "rev-old",
                mapping([
                    ("target_delegation", ystr("del-1")),
                    ("scope_override", ystr("timeline:abc")),
                ]),
            )]),
        ),
    ]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_trust_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Accept);
}

#[test]
fn network_route_selection_prefers_earlier_advertisement_on_tie() {
    let semantic = match mapping([(
        "route_selection_policy",
        mapping([("minimum_quality_hint", yi64(1))]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([(
        "candidate_routes",
        seq([
            mapping([
                ("advertiser", ystr("adv-a")),
                ("next_hop_node", ystr("hop-z")),
                ("quality_hint", yi64(3)),
                ("cost_hint", yi64(1)),
                ("advertised_at", ystr("2030-01-01T00:00:02Z")),
            ]),
            mapping([
                ("advertiser", ystr("adv-b")),
                ("next_hop_node", ystr("hop-a")),
                ("quality_hint", yi64(3)),
                ("cost_hint", yi64(1)),
                ("advertised_at", ystr("2030-01-01T00:00:01Z")),
            ]),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_network_case(&semantic, &state, &TestVerifier, &no_hash);
    let derived = result.derived.as_map().expect("derived map");
    assert_eq!(result.verdict, Verdict::Accept);
    assert_eq!(
        derived.get("selected_advertiser").and_then(Value::as_str),
        Some("adv-b")
    );
}

#[test]
fn network_route_selection_respects_allowed_responders() {
    let semantic = match mapping([(
        "route_selection_policy",
        mapping([("minimum_quality_hint", yi64(1))]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([
        (
            "candidate_routes",
            seq([
                mapping([
                    ("advertiser", ystr("adv-a")),
                    ("next_hop_node", ystr("hop-a")),
                    ("quality_hint", yi64(9)),
                    ("cost_hint", yi64(1)),
                ]),
                mapping([
                    ("advertiser", ystr("adv-b")),
                    ("next_hop_node", ystr("hop-b")),
                    ("quality_hint", yi64(5)),
                    ("cost_hint", yi64(1)),
                ]),
            ]),
        ),
        (
            "aggregate_trust_policy",
            mapping([(
                "allowed_responders",
                seq([mapping([("identity_id", ystr("adv-b"))])]),
            )]),
        ),
    ]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_network_case(&semantic, &state, &TestVerifier, &no_hash);
    let derived = result.derived.as_map().expect("derived map");
    assert_eq!(result.verdict, Verdict::Accept);
    assert_eq!(
        derived.get("selected_advertiser").and_then(Value::as_str),
        Some("adv-b")
    );
}

#[test]
fn network_route_selection_preferred_aggregator_bonus_breaks_tie() {
    let semantic = match mapping([(
        "route_selection_policy",
        mapping([("minimum_quality_hint", yi64(1))]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([
        (
            "candidate_routes",
            seq([
                mapping([
                    ("advertiser", ystr("adv-a")),
                    ("next_hop_node", ystr("hop-a")),
                    ("quality_hint", yi64(4)),
                    ("cost_hint", yi64(1)),
                ]),
                mapping([
                    ("advertiser", ystr("adv-b")),
                    ("next_hop_node", ystr("hop-b")),
                    ("quality_hint", yi64(4)),
                    ("cost_hint", yi64(1)),
                ]),
            ]),
        ),
        (
            "aggregate_trust_policy",
            mapping([(
                "preferred_aggregators",
                seq([mapping([("identity_id", ystr("adv-b"))])]),
            )]),
        ),
    ]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_network_case(&semantic, &state, &TestVerifier, &no_hash);
    let derived = result.derived.as_map().expect("derived map");
    assert_eq!(result.verdict, Verdict::Accept);
    assert_eq!(
        derived.get("selected_advertiser").and_then(Value::as_str),
        Some("adv-b")
    );
    assert_eq!(derived.get("route_score").and_then(Value::as_i64), Some(4));
}

#[test]
fn query_object_assertion_false_but_present_is_rejected() {
    let semantic = match mapping([(
        "object_assertion_proof",
        mapping([
            ("source_query_id", ystr("q-current")),
            ("object_ref", mapping([("object_id", ystr("obj-1"))])),
            ("exists", Value::Bool(false)),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([
        ("current_query_id", ystr("q-current")),
        ("available_objects", mapping([("obj-1", ystr("present"))])),
    ]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_query_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
}

#[test]
fn query_snapshot_set_proof_with_available_snapshot_accepts() {
    let semantic = match mapping([(
        "snapshot_set_proof",
        mapping([
            ("source_query_id", ystr("q-current")),
            (
                "snapshots",
                seq([mapping([("snapshot_id", ystr("snap-1"))])]),
            ),
        ]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([
        ("current_query_id", ystr("q-current")),
        (
            "available_snapshots",
            mapping([("snap-1", ystr("present"))]),
        ),
    ]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_query_case(&semantic, &state, &TestVerifier, &no_hash);
    let derived = result.derived.as_map().expect("derived map");
    assert_eq!(result.verdict, Verdict::Accept);
    assert_eq!(
        derived.get("decision").and_then(Value::as_str),
        Some("snapshot_set_proof_received")
    );
}

#[test]
fn network_route_selection_respects_minimum_trust_score_gate() {
    let semantic = match mapping([(
        "route_selection_policy",
        mapping([("minimum_quality_hint", yi64(1))]),
    )]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let state = match mapping([
        (
            "candidate_routes",
            seq([mapping([
                ("advertiser", ystr("adv-a")),
                ("next_hop_node", ystr("hop-a")),
                ("quality_hint", yi64(4)),
                ("cost_hint", yi64(1)),
            ])]),
        ),
        ("route_assignment_scores", mapping([("adv-a", yi64(0))])),
        (
            "aggregate_trust_policy",
            mapping([("minimum_trust_score", yi64(5))]),
        ),
    ]) {
        Value::Map(m) => m,
        _ => unreachable!(),
    };
    let result = validate_network_case(&semantic, &state, &TestVerifier, &no_hash);
    assert_eq!(result.verdict, Verdict::Reject);
    assert_eq!(result.reason_code, Some(ReasonCode::PolicyDenied));
}
