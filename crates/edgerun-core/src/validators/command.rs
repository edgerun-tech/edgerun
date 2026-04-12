use super::helpers::*;
use super::delegation::validate_delegation_case;
pub fn validate_command_case(
    semantic_input: &BTreeMap<String, Value>,
    local_state: &BTreeMap<String, Value>,
    verifier: &dyn FixtureVerifier,
    semantic_hash_hex: &dyn Fn(&BTreeMap<String, Value>) -> Option<String>,
) -> ValidationResult {
    let Some(command) = get_map(semantic_input, "command") else {
        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
    };
    if string_value(command, "target_node", "") != string_value(local_state, "local_node", "") {
        return reject(ReasonCode::TargetMismatch, empty_map(), empty_map());
    }
    let now = parse_ts(&string_value(local_state, "now", "1970-01-01T00:00:00Z")).unwrap_or(crate::util::DateTimeUtc::epoch());
    if let Some(s) = command.get("not_before").and_then(Value::as_str) {
        if parse_ts(s).map_or(false, |t| t > now) {
            return reject(ReasonCode::TimeInvalid, empty_map(), empty_map());
        }
    }
    if let Some(s) = command.get("expires_at").and_then(Value::as_str) {
        if parse_ts(s).map_or(true, |t| t < now) {
            return reject(ReasonCode::TimeInvalid, empty_map(), empty_map());
        }
    }
    let command_id = string_value(command, "command_id", "");
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
    let issuer = string_value(command, "issuer", "");
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
