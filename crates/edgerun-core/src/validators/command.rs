use crate::prelude::v1::*;

use super::delegation::validate_delegation_case;
use super::helpers::*;
pub fn validate_command_case(
    semantic_input: &BTreeMap<String, Value>,
    local_state: &BTreeMap<String, Value>,
    verifier: &dyn FixtureVerifier,
    semantic_hash_hex: &dyn Fn(&BTreeMap<String, Value>) -> Option<String>,
) -> ValidationResult {
    let Some(command) = get_map(semantic_input, "command") else {
        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
    };
    for field in ["envelope_version", "command_version"] {
        if let Some(reason) = version_field_error(command, field) {
            return reject(reason, empty_map(), empty_map());
        }
    }
    let command_id = string_value(command, "command_id", "");
    if command_id.is_empty() {
        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
    }
    let issuer = string_value(command, "issuer", "");
    if issuer.is_empty() {
        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
    }
    let target_node = string_value(command, "target_node", "");
    if target_node.is_empty() {
        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
    }
    let command_type = string_value(command, "command_type", "");
    if command_type.is_empty()
        || command_type == "UNSPECIFIED"
        || command_type == "COMMAND_TYPE_UNSPECIFIED"
    {
        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
    }
    if !matches!(
        command_type.as_str(),
        "ADD_CONTROLLER"
            | "REMOVE_CONTROLLER"
            | "TRANSFER_CONTROL"
            | "PUBLISH_SNAPSHOT"
            | "STORE_OBJECT"
            | "FETCH_OBJECT"
            | "QUERY"
            | "EXECUTE_WORKLOAD"
            | "TERMINATE_WORKLOAD"
            | "CREATE_DELEGATION"
            | "CREATE_REVOCATION"
            | "STORE_AND_FORWARD"
    ) {
        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
    }
    if target_node != string_value(local_state, "local_node", "") {
        return reject(ReasonCode::TargetMismatch, empty_map(), empty_map());
    }
    let now = parse_ts(&string_value(local_state, "now", "1970-01-01T00:00:00Z"))
        .unwrap_or(crate::util::DateTimeUtc::epoch());
    if !command.contains_key("issued_at") {
        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
    }
    if command
        .get("issued_at")
        .and_then(Value::as_str)
        .is_some_and(|s| parse_ts(s).is_err())
    {
        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
    }
    if let Some(s) = command.get("not_before").and_then(Value::as_str) {
        if parse_ts(s).is_ok_and(|t| t > now) {
            return reject(ReasonCode::TimeInvalid, empty_map(), empty_map());
        }
    }
    if let Some(s) = command.get("expires_at").and_then(Value::as_str) {
        if parse_ts(s).map_or(true, |t| t < now) {
            return reject(ReasonCode::TimeInvalid, empty_map(), empty_map());
        }
    }
    if let (Some(not_before), Some(expires_at)) = (
        command.get("not_before").and_then(Value::as_str),
        command.get("expires_at").and_then(Value::as_str),
    ) {
        if parse_ts(not_before)
            .and_then(|nb| parse_ts(expires_at).map(|exp| nb > exp))
            .unwrap_or(true)
        {
            return reject(ReasonCode::TimeInvalid, empty_map(), empty_map());
        }
    }
    if !matches!(command.get("signature"), Some(Value::Map(_))) {
        return reject(ReasonCode::CryptoInvalid, empty_map(), empty_map());
    }
    if let Some(result) = validate_command_nested_refs(command) {
        return result;
    }
    let command_hash = string_value(
        command,
        "command_hash_fixture",
        &string_value(
            command,
            "command_hash_hex",
            &semantic_hash_hex(command).unwrap_or_default(),
        ),
    );
    // Replay cache is keyed by command_hash; command_id is stored as an idempotency hint only.
    if let Some(replay) = get_map(local_state, "replay_cache") {
        if replay.contains_key(&command_hash) {
            return duplicate(
                ReasonCode::ReplayDetected,
                mapping([("command_hash", ystr(command_hash))]),
            );
        }
    }
    let expected = command
        .get("signature_fixture")
        .and_then(Value::as_str)
        .unwrap_or(&issuer)
        .to_string();
    if matches!(
        verifier.verify_signed_fixture(command, &expected),
        Some(false)
    ) {
        return reject(ReasonCode::CryptoInvalid, empty_map(), empty_map());
    }
    let controllers = set_from_list(local_state.get("current_controller_set"));
    if !controllers.contains(&issuer) {
        let required_scope = command
            .get("scope")
            .or_else(|| command.get("target_node"))
            .cloned()
            .unwrap_or(Value::Null);
        let transient = match mapping([
            (
                "brief",
                mapping([
                    ("issuer", ystr(issuer.clone())),
                    ("action", ystr(command_required_action(command))),
                    ("scope", required_scope),
                ]),
            ),
            (
                "delegation_chain",
                command
                    .get("delegation_chain")
                    .cloned()
                    .unwrap_or(Value::Seq(vec![])),
            ),
        ]) {
            Value::Map(m) => m,
            _ => unreachable!(),
        };
        let state = match mapping([
            (
                "trust_roots",
                local_state
                    .get("current_controller_set")
                    .cloned()
                    .unwrap_or(Value::Seq(vec![])),
            ),
            (
                "revocations",
                local_state
                    .get("revocations")
                    .cloned()
                    .unwrap_or(Value::Seq(vec![])),
            ),
            (
                "current_controller_set",
                local_state
                    .get("current_controller_set")
                    .cloned()
                    .unwrap_or(Value::Seq(vec![])),
            ),
        ]) {
            Value::Map(m) => m,
            _ => unreachable!(),
        };
        if validate_delegation_case(&transient, &state, verifier).verdict != Verdict::Accept {
            return reject(
                ReasonCode::AuthorityDenied,
                mapping([("command_hash", ystr(command_hash))]),
                empty_map(),
            );
        }
    }
    accept(
        mapping([
            ("decision", ystr("validated_pending_commit")),
            ("command_hash", ystr(command_hash.clone())),
        ]),
        mapping([(
            "replay_cache_entry",
            mapping([
                ("command_id", ystr(command_id)),
                ("command_hash", ystr(command_hash)),
            ]),
        )]),
    )
}

fn validate_command_nested_refs(command: &BTreeMap<String, Value>) -> Option<ValidationResult> {
    if command
        .get("payload_object")
        .is_some_and(|value| !object_ref_value_is_valid(value))
    {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            empty_map(),
            empty_map(),
        ));
    }
    if command
        .get("inline_payload")
        .and_then(Value::as_str)
        .is_some_and(str::is_empty)
    {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            empty_map(),
            empty_map(),
        ));
    }
    if command
        .get("command_metadata")
        .is_some_and(|value| !object_ref_value_is_valid(value))
    {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            empty_map(),
            empty_map(),
        ));
    }
    if let Some(assurance) = get_map(command, "requested_assurance") {
        if let Some(attesters) = get_seq(assurance, "acceptable_attesters") {
            for attester in attesters {
                match attester {
                    Value::String(value) if value.is_empty() => {
                        return Some(reject(
                            ReasonCode::StructuralInvalid,
                            empty_map(),
                            empty_map(),
                        ));
                    }
                    Value::Map(identity)
                        if string_value(identity, "identity_id", "").is_empty() =>
                    {
                        return Some(reject(
                            ReasonCode::StructuralInvalid,
                            empty_map(),
                            empty_map(),
                        ));
                    }
                    _ => {}
                }
            }
        }
        if assurance
            .get("max_evidence_age")
            .and_then(Value::as_i64)
            .is_some_and(|age| age < 0)
        {
            return Some(reject(
                ReasonCode::StructuralInvalid,
                empty_map(),
                empty_map(),
            ));
        }
        if assurance
            .get("assurance_metadata")
            .is_some_and(|value| !object_ref_value_is_valid(value))
        {
            return Some(reject(
                ReasonCode::StructuralInvalid,
                empty_map(),
                empty_map(),
            ));
        }
    }
    None
}
