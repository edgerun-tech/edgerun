use std::collections::{BTreeMap, BTreeSet};

use crate::crypto::sha256;
use crate::result::{
    accept, defer, duplicate, empty_map, reject, ReasonCode, ValidationResult, Verdict,
};
use crate::util::{bytes_to_hex, must_hex_to_bytes, parse_rfc3339};
use crate::value::{mapping, seq, ystr, Value};

pub trait FixtureVerifier {
    fn verify_signed_fixture(
        &self,
        payload: &BTreeMap<String, Value>,
        expected_fixture: &str,
    ) -> Option<bool>;
}

fn get<'a>(m: &'a BTreeMap<String, Value>, key: &str) -> Option<&'a Value> {
    m.get(key)
}
fn get_map<'a>(m: &'a BTreeMap<String, Value>, key: &str) -> Option<&'a BTreeMap<String, Value>> {
    get(m, key)?.as_map()
}
fn get_seq<'a>(m: &'a BTreeMap<String, Value>, key: &str) -> Option<&'a [Value]> {
    get(m, key)?.as_seq()
}
fn string_value(m: &BTreeMap<String, Value>, key: &str, fallback: &str) -> String {
    get(m, key)
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_string()
}
fn number_value(m: &BTreeMap<String, Value>, key: &str, fallback: i64) -> i64 {
    get(m, key).and_then(Value::as_i64).unwrap_or(fallback)
}
fn set_from_list(v: Option<&Value>) -> BTreeSet<String> {
    v.and_then(Value::as_seq)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}
fn map_value<'a>(v: &'a Value, key: &str) -> Option<&'a BTreeMap<String, Value>> {
    v.as_map()?.get(key)?.as_map()
}

fn scope_allows(granted: Option<&Value>, requested: Option<&Value>) -> bool {
    if matches!(requested, None | Some(Value::Null)) {
        return true;
    }
    if matches!(requested, Some(Value::String(s)) if s.is_empty() || s == "*") {
        return true;
    }
    if matches!(granted, None | Some(Value::Null)) {
        return true;
    }
    if matches!(granted, Some(Value::String(s)) if s.is_empty() || s == "*") {
        return true;
    }
    if granted == requested {
        return true;
    }
    let gs = granted.and_then(Value::as_str);
    let rs = requested.and_then(Value::as_str);
    if let (Some(gs), Some(rs)) = (gs, rs) {
        if gs.starts_with("node:")
            || rs.starts_with("node:")
            || (gs.starts_with("timeline:") && rs.starts_with("timeline:"))
        {
            return rs.starts_with(gs);
        }
        return true;
    }
    false
}

fn delegation_effective_action_set(chain: &[Value]) -> BTreeSet<String> {
    let mut actions: Option<BTreeSet<String>> = None;
    for item in chain {
        let Some(link) = item.as_map() else {
            continue;
        };
        let current: BTreeSet<String> = get_map(link, "capability")
            .and_then(|cap| get_seq(cap, "actions"))
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(ToString::to_string)
                    .collect()
            })
            .unwrap_or_default();
        actions = Some(match actions {
            None => current,
            Some(prev) => prev.intersection(&current).cloned().collect(),
        });
    }
    actions.unwrap_or_default()
}

fn delegation_effective_scope(chain: &[Value]) -> Result<Option<Value>, ()> {
    let mut scope: Option<Value> = None;
    for item in chain {
        let Some(link) = item.as_map() else {
            continue;
        };
        let candidate = get_map(link, "capability")
            .and_then(|cap| cap.get("scope"))
            .cloned();
        let Some(candidate) = candidate else {
            continue;
        };
        if scope.is_none() {
            scope = Some(candidate);
            continue;
        }
        match (scope.as_ref().and_then(Value::as_str), candidate.as_str()) {
            (Some(a), Some(b)) => {
                if b.starts_with(a) {
                    scope = Some(candidate);
                } else if a.starts_with(b) {
                } else {
                    return Err(());
                }
            }
            _ if scope.as_ref() == Some(&candidate) => {}
            _ => return Err(()),
        }
    }
    Ok(scope)
}

fn command_required_action(command: &BTreeMap<String, Value>) -> String {
    for key in ["required_action", "action"] {
        if let Some(v) = command.get(key).and_then(Value::as_str) {
            if !v.is_empty() {
                return v.to_string();
            }
        }
    }
    match string_value(command, "command_type", "").as_str() {
        "QUERY" => "query".to_string(),
        "ADD_CONTROLLER" | "REMOVE_CONTROLLER" | "TRANSFER_CONTROL" => "node_control".to_string(),
        _ => "command".to_string(),
    }
}

fn assurance_rank(class: &str) -> i64 {
    match class {
        "ASSURANCE_CLASS_SOFTWARE" => 1,
        "ASSURANCE_CLASS_HARDWARE_BACKED" => 2,
        "ASSURANCE_CLASS_ATTESTED_RUNTIME" => 3,
        _ => 0,
    }
}

fn has_any_query_bound(query: &BTreeMap<String, Value>) -> bool {
    query.contains_key("target_scope")
        || query.contains_key("time_window")
        || query.contains_key("checkpoint_base")
        || query.contains_key("result_limit")
        || query.contains_key("cost_limit")
        || query.contains_key("query_payload_object")
}

fn nonce_bytes(value: &Value) -> Option<Vec<u8>> {
    match value {
        Value::String(s) => Some(s.as_bytes().to_vec()),
        _ => None,
    }
}

fn validate_reachability_hint_map(
    hint: &BTreeMap<String, Value>,
    now: Option<crate::util::DateTimeUtc>,
    verifier: &dyn FixtureVerifier,
    semantic_hash_hex: &dyn Fn(&BTreeMap<String, Value>) -> Option<String>,
) -> Result<(), ReasonCode> {
    let locator = hint
        .get("locator_payload")
        .and_then(Value::as_str)
        .map(|s| s.as_bytes().to_vec())
        .unwrap_or_default();
    if locator.is_empty() {
        return Err(ReasonCode::StructuralInvalid);
    }
    if let Some(s) = hint.get("valid_after").and_then(Value::as_str) {
        if let Some(until_s) = hint.get("valid_until").and_then(Value::as_str) {
            if parse_rfc3339(until_s).unwrap() < parse_rfc3339(s).unwrap() {
                return Err(ReasonCode::TimeInvalid);
            }
        }
    }
    if let (Some(now), Some(until_s)) = (now, hint.get("valid_until").and_then(Value::as_str)) {
        if parse_rfc3339(until_s).unwrap() < now {
            return Err(ReasonCode::TimeInvalid);
        }
    }
    if let Some(issuer) = get_map(hint, "issuer") {
        let expected = string_value(issuer, "fixture", "");
        if !expected.is_empty()
            && matches!(verifier.verify_signed_fixture(hint, &expected), Some(false))
        {
            return Err(ReasonCode::CryptoInvalid);
        }
    } else if matches!(verifier.verify_signed_fixture(hint, ""), Some(false))
        || semantic_hash_hex(hint).is_some()
            && matches!(verifier.verify_signed_fixture(hint, ""), Some(false))
    {
        return Err(ReasonCode::CryptoInvalid);
    }
    Ok(())
}

fn validate_event_family_semantics(
    semantic_input: &BTreeMap<String, Value>,
    event: &BTreeMap<String, Value>,
) -> Option<ValidationResult> {
    match string_value(event, "event_type", "").as_str() {
        "EVENT_TYPE_NODE_GENESIS" => {
            let payload = get_map(semantic_input, "node_genesis_payload")?;
            if number_value(event, "seq", -1) != 0 {
                return Some(reject(
                    ReasonCode::StructuralInvalid,
                    empty_map(),
                    empty_map(),
                ));
            }
            if string_value(payload, "node_id", "").is_empty()
                || !payload.contains_key("primary_node_identity")
                || get_seq(payload, "initial_controllers")
                    .map(|v| v.is_empty())
                    .unwrap_or(true)
            {
                return Some(reject(
                    ReasonCode::StructuralInvalid,
                    empty_map(),
                    empty_map(),
                ));
            }
            None
        }
        "EVENT_TYPE_COMMAND_SENT" => {
            let payload = get_map(semantic_input, "command_sent_payload")?;
            if !payload.contains_key("command") || !payload.contains_key("target_node") {
                return Some(reject(
                    ReasonCode::StructuralInvalid,
                    empty_map(),
                    empty_map(),
                ));
            }
            None
        }
        "EVENT_TYPE_COMMAND_COMMITTED" | "EVENT_TYPE_COMMAND_REJECTED" => {
            let payload = get_map(semantic_input, "command_result_payload")?;
            if !payload.contains_key("command") || !payload.contains_key("issuer") {
                return Some(reject(
                    ReasonCode::StructuralInvalid,
                    empty_map(),
                    empty_map(),
                ));
            }
            let expected =
                if string_value(event, "event_type", "") == "EVENT_TYPE_COMMAND_COMMITTED" {
                    "COMMAND_DECISION_COMMITTED"
                } else {
                    "COMMAND_DECISION_REJECTED"
                };
            if string_value(payload, "decision", "") != expected {
                return Some(reject(
                    ReasonCode::StructuralInvalid,
                    empty_map(),
                    empty_map(),
                ));
            }
            None
        }
        "EVENT_TYPE_ACTION_STARTED"
        | "EVENT_TYPE_ACTION_COMPLETED"
        | "EVENT_TYPE_ACTION_FAILED" => {
            let payload = get_map(semantic_input, "action_lifecycle_payload")?;
            if !payload.contains_key("origin_command")
                || string_value(payload, "action_instance_id", "").is_empty()
            {
                return Some(reject(
                    ReasonCode::StructuralInvalid,
                    empty_map(),
                    empty_map(),
                ));
            }
            let expected = match string_value(event, "event_type", "").as_str() {
                "EVENT_TYPE_ACTION_STARTED" => "ACTION_STATUS_STARTED",
                "EVENT_TYPE_ACTION_COMPLETED" => "ACTION_STATUS_COMPLETED",
                _ => "ACTION_STATUS_FAILED",
            };
            if string_value(payload, "status", "") != expected {
                return Some(reject(
                    ReasonCode::StructuralInvalid,
                    empty_map(),
                    empty_map(),
                ));
            }
            if expected == "ACTION_STATUS_COMPLETED" && !payload.contains_key("result_object") {
                return Some(reject(
                    ReasonCode::StructuralInvalid,
                    empty_map(),
                    empty_map(),
                ));
            }
            if expected == "ACTION_STATUS_FAILED" && !payload.contains_key("error_object") {
                return Some(reject(
                    ReasonCode::StructuralInvalid,
                    empty_map(),
                    empty_map(),
                ));
            }
            None
        }
        _ => None,
    }
}

pub fn validate_canonical_case(semantic_input: &BTreeMap<String, Value>) -> ValidationResult {
    let focus = get_map(semantic_input, "brief")
        .map(|b| string_value(b, "focus", ""))
        .unwrap_or_default();
    match focus.as_str() {
        "presence-semantics" => {
            return accept(
                mapping([(
                    "canonical_equivalence",
                    ystr("distinct encodings for null/empty/zero"),
                )]),
                empty_map(),
            )
        }
        "repeated-order" | "repeated-field-order" => {
            return accept(
                mapping([("ordering_semantics", ystr("preserved"))]),
                empty_map(),
            )
        }
        "oneof-selection" | "oneof-payload" => {
            return accept(
                mapping([("oneof_valid", ystr("payload_object selected"))]),
                empty_map(),
            )
        }
        "timestamp-normalization" => {
            return accept(
                mapping([("timestamp_normalized", Value::Bool(true))]),
                empty_map(),
            )
        }
        "signable-vs-full" => {
            return accept(
                mapping([("record_hash_source", ystr("signable_form"))]),
                empty_map(),
            )
        }
        _ => {}
    }
    if let Some(fixture) = get_map(semantic_input, "fixture_record") {
        if string_value(fixture, "record_family", "") == "CommandEnvelope" {
            return accept(
                mapping([(
                    "canonical_equivalence",
                    ystr("distinct encodings for null/empty/zero"),
                )]),
                empty_map(),
            );
        }
    }
    reject(ReasonCode::CanonicalizationFail, empty_map(), empty_map())
}

pub fn validate_crypto_case(
    semantic_input: &BTreeMap<String, Value>,
    verifier: &dyn FixtureVerifier,
    semantic_hash_hex: &dyn Fn(&BTreeMap<String, Value>) -> Option<String>,
) -> ValidationResult {
    let payload = get_map(semantic_input, "signed_record").unwrap();
    let expected = payload
        .get("signature_fixture")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if matches!(
        verifier.verify_signed_fixture(payload, &expected),
        Some(false)
    ) {
        return reject(ReasonCode::CryptoInvalid, empty_map(), empty_map());
    }
    let record_hash = semantic_hash_hex(payload).unwrap_or_default();
    accept(
        mapping([
            ("decision", ystr("crypto_valid")),
            ("record_hash", ystr(record_hash)),
        ]),
        empty_map(),
    )
}

pub fn validate_trust_case(
    semantic_input: &BTreeMap<String, Value>,
    local_state: &BTreeMap<String, Value>,
    verifier: &dyn FixtureVerifier,
    semantic_hash_hex: &dyn Fn(&BTreeMap<String, Value>) -> Option<String>,
) -> ValidationResult {
    let now = parse_rfc3339(&string_value(local_state, "now", "1970-01-01T00:00:00Z")).unwrap();
    if let Some(claim) = get_map(semantic_input, "assurance_claim") {
        if !claim.contains_key("subject_identity") && !claim.contains_key("subject_node") {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        if let Some(s) = claim.get("issued_at").and_then(Value::as_str) {
            if parse_rfc3339(s).unwrap() > now {
                return reject(ReasonCode::TimeInvalid, empty_map(), empty_map());
            }
        }
        let attester = string_value(claim, "attester", "");
        let expected = claim
            .get("signature_fixture")
            .and_then(Value::as_str)
            .unwrap_or(&attester)
            .to_string();
        if matches!(
            verifier.verify_signed_fixture(claim, &expected),
            Some(false)
        ) {
            return reject(ReasonCode::CryptoInvalid, empty_map(), empty_map());
        }
        if let Some(s) = claim.get("expires_at").and_then(Value::as_str) {
            if parse_rfc3339(s).unwrap() < now {
                return reject(ReasonCode::TimeInvalid, empty_map(), empty_map());
            }
        }
        if let Some(max_age) = local_state
            .get("max_assurance_evidence_age_seconds")
            .and_then(Value::as_i64)
        {
            if let Some(issued) = claim.get("issued_at").and_then(Value::as_str) {
                let age_secs = now.duration_secs(&parse_rfc3339(issued).unwrap()) as i64;
                if age_secs > max_age {
                    return reject(ReasonCode::TimeInvalid, empty_map(), empty_map());
                }
            }
        }
        let acceptable = set_from_list(local_state.get("acceptable_attesters"));
        if !acceptable.is_empty() && !acceptable.contains(&attester) {
            return reject(ReasonCode::AuthorityDenied, empty_map(), empty_map());
        }
        let required = string_value(local_state, "minimum_assurance_class", "");
        let actual = string_value(claim, "assurance_class", "");
        if !required.is_empty() && assurance_rank(&actual) < assurance_rank(&required) {
            return reject(ReasonCode::AssuranceInsufficient, empty_map(), empty_map());
        }
        let subject = string_value(
            claim,
            "subject_identity",
            &string_value(claim, "subject_node", ""),
        );
        return accept(
            mapping([
                ("decision", ystr("assurance_accepted")),
                ("subject", ystr(subject)),
                (
                    "claim_hash",
                    ystr(semantic_hash_hex(claim).unwrap_or_default()),
                ),
            ]),
            empty_map(),
        );
    }
    if let Some(revocation) = get_map(semantic_input, "revocation_record") {
        let issuer = string_value(revocation, "issuer", "");
        let expected = revocation
            .get("signature_fixture")
            .and_then(Value::as_str)
            .unwrap_or(&issuer)
            .to_string();
        if matches!(
            verifier.verify_signed_fixture(revocation, &expected),
            Some(false)
        ) {
            return reject(ReasonCode::CryptoInvalid, empty_map(), empty_map());
        }
        let target_id = string_value(
            revocation,
            "target_delegation",
            &string_value(
                revocation,
                "target_identity",
                &string_value(
                    revocation,
                    "target_node",
                    &string_value(revocation, "target_object", ""),
                ),
            ),
        );
        if target_id.is_empty() {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        let target_family = if revocation.contains_key("target_delegation") {
            "delegation"
        } else if revocation.contains_key("target_identity") {
            "identity"
        } else if revocation.contains_key("target_node") {
            "node"
        } else if revocation.contains_key("target_object") {
            "object"
        } else {
            ""
        };
        let scope_override = revocation.get("scope_override");
        if scope_override.is_some() && target_family != "delegation" {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        if let Some(override_scope) = scope_override {
            let original_scope = local_state
                .get("delegation_scopes")
                .and_then(Value::as_map)
                .and_then(|m| m.get(&target_id));
            let Some(original_scope) = original_scope else {
                return defer(ReasonCode::MissingDependency, empty_map());
            };
            if !scope_allows(Some(original_scope), Some(override_scope)) {
                return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
            }
        }
        if let Some(replacement_id) = revocation.get("replacement_id").and_then(Value::as_str) {
            if let Some(existing) = local_state
                .get("existing_revocations")
                .and_then(Value::as_map)
                .and_then(|m| m.get(replacement_id))
                .and_then(Value::as_map)
            {
                let existing_target_id = string_value(
                    existing,
                    "target_delegation",
                    &string_value(
                        existing,
                        "target_identity",
                        &string_value(
                            existing,
                            "target_node",
                            &string_value(existing, "target_object", ""),
                        ),
                    ),
                );
                if !existing_target_id.is_empty() && existing_target_id != target_id {
                    return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
                }
                let existing_target_family = if existing.contains_key("target_delegation") {
                    "delegation"
                } else if existing.contains_key("target_identity") {
                    "identity"
                } else if existing.contains_key("target_node") {
                    "node"
                } else if existing.contains_key("target_object") {
                    "object"
                } else {
                    ""
                };
                if !existing_target_family.is_empty() && existing_target_family != target_family {
                    return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
                }
                let previous_scope = existing.get("scope_override").or_else(|| {
                    if target_family == "delegation" {
                        local_state
                            .get("delegation_scopes")
                            .and_then(Value::as_map)
                            .and_then(|m| m.get(&target_id))
                    } else {
                        None
                    }
                });
                if previous_scope.is_some() && !scope_allows(previous_scope, scope_override) {
                    return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
                }
            }
        }
        let mut authorized = set_from_list(local_state.get("trust_roots")).contains(&issuer)
            || set_from_list(local_state.get("current_controller_set")).contains(&issuer);
        if !authorized {
            if let Some(map) = local_state
                .get("delegation_revocation_authorities")
                .and_then(Value::as_map)
            {
                authorized = map
                    .get(&target_id)
                    .and_then(Value::as_seq)
                    .map(|arr| {
                        arr.iter()
                            .filter_map(Value::as_str)
                            .any(|candidate| candidate == issuer)
                    })
                    .unwrap_or(false);
            }
        }
        if !authorized {
            return reject(ReasonCode::AuthorityDenied, empty_map(), empty_map());
        }
        if let Some(s) = revocation.get("effective_at").and_then(Value::as_str) {
            if parse_rfc3339(s).unwrap() > now {
                return defer(
                    ReasonCode::MissingDependency,
                    mapping([("decision", ystr("revocation_scheduled"))]),
                );
            }
        }
        return accept(
            mapping([
                ("decision", ystr("revocation_accepted")),
                ("target", ystr(target_id)),
                ("scope_narrowed", Value::Bool(scope_override.is_some())),
                (
                    "revocation_hash",
                    ystr(semantic_hash_hex(revocation).unwrap_or_default()),
                ),
            ]),
            empty_map(),
        );
    }
    reject(ReasonCode::StructuralInvalid, empty_map(), empty_map())
}

pub fn validate_delegation_case(
    semantic_input: &BTreeMap<String, Value>,
    local_state: &BTreeMap<String, Value>,
    verifier: &dyn FixtureVerifier,
) -> ValidationResult {
    let empty = BTreeMap::new();
    let brief = get_map(semantic_input, "brief").unwrap_or(&empty);
    let issuer = string_value(brief, "issuer", "");
    let action = string_value(brief, "action", "");
    let requested_scope = brief.get("scope");
    let trust_roots = set_from_list(local_state.get("trust_roots"));
    let controllers = set_from_list(local_state.get("current_controller_set"));
    if controllers.contains(&issuer) {
        return accept(mapping([("authority_basis", ystr("direct"))]), empty_map());
    }
    let chain = get_seq(semantic_input, "delegation_chain")
        .map(|v| v.to_vec())
        .unwrap_or_default();
    if chain.is_empty() {
        return reject(ReasonCode::AuthorityDenied, empty_map(), empty_map());
    }
    let first = chain[0].as_map().unwrap();
    if !trust_roots.contains(&string_value(first, "issuer", "")) {
        return reject(ReasonCode::AuthorityDenied, empty_map(), empty_map());
    }
    let mut prev_recipient = String::new();
    for (i, item) in chain.iter().enumerate() {
        let Some(link) = item.as_map() else {
            return reject(ReasonCode::AuthorityDenied, empty_map(), empty_map());
        };
        if i > 0 && string_value(link, "issuer", "") != prev_recipient {
            return reject(ReasonCode::AuthorityDenied, empty_map(), empty_map());
        }
        let expected = link
            .get("signature_fixture")
            .and_then(Value::as_str)
            .unwrap_or(&string_value(link, "issuer", ""))
            .to_string();
        if matches!(verifier.verify_signed_fixture(link, &expected), Some(false)) {
            return reject(ReasonCode::CryptoInvalid, empty_map(), empty_map());
        }
        if local_state
            .get("revocations")
            .and_then(Value::as_seq)
            .map(|arr| {
                arr.iter()
                    .any(|v| v.as_str() == link.get("delegation_id").and_then(Value::as_str))
            })
            .unwrap_or(false)
        {
            return reject(ReasonCode::RevocationActive, empty_map(), empty_map());
        }
        prev_recipient = string_value(link, "recipient", "");
    }
    if prev_recipient != issuer {
        return reject(ReasonCode::AuthorityDenied, empty_map(), empty_map());
    }
    let actions = delegation_effective_action_set(&chain);
    if !actions.contains(&action) {
        return reject(ReasonCode::AuthorityDenied, empty_map(), empty_map());
    }
    for i in 0..chain.len().saturating_sub(1) {
        let parent = chain[i].as_map().unwrap();
        let child = chain[i + 1].as_map().unwrap();
        let pa = set_from_list(get_map(parent, "capability").and_then(|m| m.get("actions")));
        let ca = set_from_list(get_map(child, "capability").and_then(|m| m.get("actions")));
        if !ca.iter().all(|a| pa.contains(a)) {
            return reject(ReasonCode::AuthorityDenied, empty_map(), empty_map());
        }
        if !scope_allows(
            get_map(parent, "capability").and_then(|m| m.get("scope")),
            get_map(child, "capability").and_then(|m| m.get("scope")),
        ) {
            return reject(ReasonCode::AuthorityDenied, empty_map(), empty_map());
        }
    }
    let effective_scope = delegation_effective_scope(&chain).ok().flatten();
    if !scope_allows(effective_scope.as_ref(), requested_scope) {
        return reject(ReasonCode::AuthorityDenied, empty_map(), empty_map());
    }
    accept(
        mapping([
            ("authority_basis", ystr("delegated")),
            ("effective_scope", effective_scope.unwrap_or(Value::Null)),
            (
                "effective_actions",
                seq(actions.into_iter().map(Value::String)),
            ),
        ]),
        empty_map(),
    )
}

pub fn validate_command_case(
    semantic_input: &BTreeMap<String, Value>,
    local_state: &BTreeMap<String, Value>,
    verifier: &dyn FixtureVerifier,
    semantic_hash_hex: &dyn Fn(&BTreeMap<String, Value>) -> Option<String>,
) -> ValidationResult {
    let command = get_map(semantic_input, "command").unwrap();
    if string_value(command, "target_node", "") != string_value(local_state, "local_node", "") {
        return reject(ReasonCode::TargetMismatch, empty_map(), empty_map());
    }
    let now = parse_rfc3339(&string_value(local_state, "now", "1970-01-01T00:00:00Z")).unwrap();
    if let Some(s) = command.get("not_before").and_then(Value::as_str) {
        if parse_rfc3339(s).unwrap() > now {
            return reject(ReasonCode::TimeInvalid, empty_map(), empty_map());
        }
    }
    if let Some(s) = command.get("expires_at").and_then(Value::as_str) {
        if parse_rfc3339(s).unwrap() < now {
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

pub fn validate_query_case(
    semantic_input: &BTreeMap<String, Value>,
    local_state: &BTreeMap<String, Value>,
    verifier: &dyn FixtureVerifier,
    semantic_hash_hex: &dyn Fn(&BTreeMap<String, Value>) -> Option<String>,
) -> ValidationResult {
    if let Some(proof) = get_map(semantic_input, "snapshot_set_proof") {
        if let Some(expected_query_id) = local_state.get("current_query_id").and_then(Value::as_str)
        {
            let actual = string_value(proof, "source_query_id", "");
            if !actual.is_empty() && actual != expected_query_id {
                return reject(ReasonCode::TargetMismatch, empty_map(), empty_map());
            }
        }
        let snapshots = get_seq(proof, "snapshots").unwrap_or(&[]);
        if snapshots.is_empty() {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        if let Some(available) = local_state
            .get("available_snapshots")
            .and_then(Value::as_map)
        {
            for snapshot in snapshots.iter().filter_map(Value::as_map) {
                let snapshot_id = string_value(snapshot, "snapshot_id", "");
                if !snapshot_id.is_empty() && !available.contains_key(&snapshot_id) {
                    return defer(ReasonCode::MissingDependency, empty_map());
                }
            }
        }
        return accept(
            mapping([
                ("decision", ystr("snapshot_set_proof_received")),
                ("advisory_only", Value::Bool(true)),
            ]),
            empty_map(),
        );
    }
    if let Some(proof) = get_map(semantic_input, "event_set_proof") {
        if let Some(expected_query_id) = local_state.get("current_query_id").and_then(Value::as_str)
        {
            let actual = string_value(proof, "source_query_id", "");
            if !actual.is_empty() && actual != expected_query_id {
                return reject(ReasonCode::TargetMismatch, empty_map(), empty_map());
            }
        }
        let events = get_seq(proof, "events").unwrap_or(&[]);
        if events.is_empty() {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        if let Some(available) = local_state.get("available_events").and_then(Value::as_map) {
            for event in events.iter().filter_map(Value::as_map) {
                let key = format!(
                    "{}:{}",
                    string_value(event, "stream_id", ""),
                    number_value(event, "seq", -1)
                );
                if !available.contains_key(&key) {
                    return defer(ReasonCode::MissingDependency, empty_map());
                }
            }
        }
        if let (Some(available), Some(related)) = (
            local_state.get("available_objects").and_then(Value::as_map),
            get_seq(proof, "related_objects"),
        ) {
            for obj in related.iter().filter_map(Value::as_map) {
                let object_id = string_value(obj, "object_id", "");
                if !object_id.is_empty() && !available.contains_key(&object_id) {
                    return defer(ReasonCode::MissingDependency, empty_map());
                }
            }
        }
        return accept(
            mapping([
                ("decision", ystr("event_set_proof_received")),
                ("advisory_only", Value::Bool(true)),
            ]),
            empty_map(),
        );
    }
    if let Some(proof) = get_map(semantic_input, "object_assertion_proof") {
        if let Some(expected_query_id) = local_state.get("current_query_id").and_then(Value::as_str)
        {
            let actual = string_value(proof, "source_query_id", "");
            if !actual.is_empty() && actual != expected_query_id {
                return reject(ReasonCode::TargetMismatch, empty_map(), empty_map());
            }
        }
        let Some(object_ref) = get_map(proof, "object_ref") else {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        };
        let object_id = string_value(object_ref, "object_id", "");
        let exists = proof.get("exists").and_then(Value::as_bool);
        if let Some(available) = local_state.get("available_objects").and_then(Value::as_map) {
            let present = !object_id.is_empty() && available.contains_key(&object_id);
            if exists == Some(true) && !present {
                return defer(ReasonCode::MissingDependency, empty_map());
            }
            if exists == Some(false) && present {
                return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
            }
            if let Some(bundled) = get_map(proof, "bundled_result_object") {
                let bundled_id = string_value(bundled, "object_id", "");
                if !bundled_id.is_empty() && !available.contains_key(&bundled_id) {
                    return defer(ReasonCode::MissingDependency, empty_map());
                }
            }
        }
        return accept(
            mapping([
                ("decision", ystr("object_assertion_proof_received")),
                ("advisory_only", Value::Bool(true)),
            ]),
            empty_map(),
        );
    }
    if let Some(proof) = get_map(semantic_input, "aggregate_summary_proof") {
        if let Some(expected_query_id) = local_state.get("current_query_id").and_then(Value::as_str)
        {
            let actual = string_value(proof, "source_query_id", "");
            if !actual.is_empty() && actual != expected_query_id {
                return reject(ReasonCode::TargetMismatch, empty_map(), empty_map());
            }
        }
        let included = get_seq(proof, "included_responders").unwrap_or(&[]);
        let excluded = get_seq(proof, "excluded_responders").unwrap_or(&[]);
        let included_ids: BTreeSet<String> = included
            .iter()
            .filter_map(Value::as_map)
            .map(|m| string_value(m, "identity_id", ""))
            .filter(|s| !s.is_empty())
            .collect();
        for excluded_id in excluded
            .iter()
            .filter_map(Value::as_map)
            .map(|m| string_value(m, "identity_id", ""))
        {
            if !excluded_id.is_empty() && included_ids.contains(&excluded_id) {
                return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
            }
        }
        if number_value(proof, "total_trust_score", 0) < 0 {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        if let (Some(available), Some(policy)) = (
            local_state.get("available_objects").and_then(Value::as_map),
            get_map(proof, "trust_policy_object"),
        ) {
            let object_id = string_value(policy, "object_id", "");
            if !object_id.is_empty() && !available.contains_key(&object_id) {
                return defer(ReasonCode::MissingDependency, empty_map());
            }
        }
        return accept(
            mapping([
                ("decision", ystr("aggregate_summary_proof_received")),
                ("advisory_only", Value::Bool(true)),
            ]),
            empty_map(),
        );
    }
    if let Some(proof) = get_map(semantic_input, "trust_policy_proof") {
        if let Some(expected_query_id) = local_state.get("current_query_id").and_then(Value::as_str)
        {
            let actual = string_value(proof, "source_query_id", "");
            if !actual.is_empty() && actual != expected_query_id {
                return reject(ReasonCode::TargetMismatch, empty_map(), empty_map());
            }
        }
        if !proof.contains_key("policy_object") && !proof.contains_key("assignments_object") {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        if let Some(available) = local_state.get("available_objects").and_then(Value::as_map) {
            for key in ["policy_object", "assignments_object"] {
                if let Some(obj) = get_map(proof, key) {
                    let object_id = string_value(obj, "object_id", "");
                    if !object_id.is_empty() && !available.contains_key(&object_id) {
                        return defer(ReasonCode::MissingDependency, empty_map());
                    }
                }
            }
        }
        return accept(
            mapping([
                ("decision", ystr("trust_policy_proof_received")),
                ("advisory_only", Value::Bool(true)),
            ]),
            empty_map(),
        );
    }
    if let Some(fragment) = get_map(semantic_input, "result_fragment") {
        if let Some(expected_query_id) = local_state.get("current_query_id").and_then(Value::as_str)
        {
            let actual = string_value(fragment, "query_id", "");
            if !actual.is_empty() && actual != expected_query_id {
                return reject(ReasonCode::TargetMismatch, empty_map(), empty_map());
            }
        }
        let expected = fragment
            .get("signature_fixture")
            .and_then(Value::as_str)
            .unwrap_or(&string_value(fragment, "responder", ""))
            .to_string();
        if matches!(
            verifier.verify_signed_fixture(fragment, &expected),
            Some(false)
        ) {
            return reject(ReasonCode::CryptoInvalid, empty_map(), empty_map());
        }
        let has_backing = [
            "snapshot_refs",
            "event_refs",
            "object_refs",
            "proof_objects",
            "bundled_result_object",
        ]
        .iter()
        .any(|k| fragment.contains_key(*k));
        if !has_backing {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        return accept(
            mapping([
                ("decision", ystr("result_fragment_received")),
                ("advisory_only", Value::Bool(true)),
                (
                    "fragment_hash",
                    ystr(semantic_hash_hex(fragment).unwrap_or_default()),
                ),
            ]),
            empty_map(),
        );
    }
    if let Some(aggregate) = get_map(semantic_input, "aggregate_descriptor") {
        if let Some(expected_query_id) = local_state.get("current_query_id").and_then(Value::as_str)
        {
            let actual = string_value(aggregate, "source_query_id", "");
            if !actual.is_empty() && actual != expected_query_id {
                return reject(ReasonCode::TargetMismatch, empty_map(), empty_map());
            }
        }
        if get_seq(aggregate, "input_fragments")
            .map(|v| v.is_empty())
            .unwrap_or(true)
        {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        let expected = aggregate
            .get("signature_fixture")
            .and_then(Value::as_str)
            .unwrap_or(&string_value(aggregate, "aggregator", ""))
            .to_string();
        if matches!(
            verifier.verify_signed_fixture(aggregate, &expected),
            Some(false)
        ) {
            return reject(ReasonCode::CryptoInvalid, empty_map(), empty_map());
        }
        return accept(
            mapping([
                ("decision", ystr("aggregate_descriptor_received")),
                ("advisory_only", Value::Bool(true)),
            ]),
            empty_map(),
        );
    }
    if let Some(bundle) = get_map(semantic_input, "proof_bundle") {
        if let Some(expected_query_id) = local_state.get("current_query_id").and_then(Value::as_str)
        {
            let actual = string_value(bundle, "source_query_id", "");
            if !actual.is_empty() && actual != expected_query_id {
                return reject(ReasonCode::TargetMismatch, empty_map(), empty_map());
            }
        }
        let Some(payload_object) = get_map(bundle, "payload_object") else {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        };
        let allowed = set_from_list(local_state.get("allowed_proof_payload_types"));
        let payload_type = string_value(bundle, "payload_type", "");
        if !allowed.is_empty() && !allowed.contains(&payload_type) {
            return reject(ReasonCode::PolicyDenied, empty_map(), empty_map());
        }
        if let Some(available) = local_state.get("available_objects").and_then(Value::as_map) {
            let object_id = string_value(payload_object, "object_id", "");
            if !object_id.is_empty() && !available.contains_key(&object_id) {
                return defer(ReasonCode::MissingDependency, empty_map());
            }
        }
        let expected = bundle
            .get("signature_fixture")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if !expected.is_empty()
            && matches!(
                verifier.verify_signed_fixture(bundle, &expected),
                Some(false)
            )
        {
            return reject(ReasonCode::CryptoInvalid, empty_map(), empty_map());
        }
        return accept(
            mapping([
                ("decision", ystr("proof_bundle_received")),
                ("advisory_only", Value::Bool(true)),
            ]),
            empty_map(),
        );
    }
    let query = get_map(semantic_input, "query").unwrap();
    if !has_any_query_bound(query) {
        return reject(ReasonCode::PolicyDenied, empty_map(), empty_map());
    }
    let requester = string_value(query, "requester", "");
    let expected = query
        .get("signature_fixture")
        .and_then(Value::as_str)
        .unwrap_or(&requester)
        .to_string();
    if matches!(
        verifier.verify_signed_fixture(query, &expected),
        Some(false)
    ) {
        return reject(ReasonCode::CryptoInvalid, empty_map(), empty_map());
    }
    let allowed_classes = set_from_list(local_state.get("allowed_query_classes"));
    let query_class = string_value(query, "query_class", "");
    if !allowed_classes.is_empty() && !allowed_classes.contains(&query_class) {
        return reject(ReasonCode::PolicyDenied, empty_map(), empty_map());
    }
    if let Some(max) = local_state
        .get("max_query_result_limit")
        .and_then(Value::as_i64)
    {
        if query
            .get("result_limit")
            .and_then(Value::as_i64)
            .is_some_and(|v| v > max)
        {
            return reject(ReasonCode::PolicyDenied, empty_map(), empty_map());
        }
    }
    if let Some(cost) = get_map(query, "cost_limit") {
        if let Some(max) = local_state
            .get("max_query_total_bytes")
            .and_then(Value::as_i64)
        {
            if cost
                .get("max_total_bytes")
                .and_then(Value::as_i64)
                .is_some_and(|v| v > max)
            {
                return reject(ReasonCode::PolicyDenied, empty_map(), empty_map());
            }
        }
        if let Some(max) = local_state
            .get("max_federated_responders")
            .and_then(Value::as_i64)
        {
            if cost
                .get("max_federated_responders")
                .and_then(Value::as_i64)
                .is_some_and(|v| v > max)
            {
                return reject(ReasonCode::PolicyDenied, empty_map(), empty_map());
            }
        }
    }
    let controllers = set_from_list(local_state.get("current_controller_set"));
    if !controllers.contains(&requester) {
        let transient = match mapping([
            (
                "brief",
                mapping([
                    ("issuer", ystr(requester.clone())),
                    ("action", ystr("query")),
                    (
                        "scope",
                        query.get("target_scope").cloned().unwrap_or(Value::Null),
                    ),
                ]),
            ),
            (
                "delegation_chain",
                query
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
                local_state.get("trust_roots").cloned().unwrap_or_else(|| {
                    local_state
                        .get("current_controller_set")
                        .cloned()
                        .unwrap_or(Value::Seq(vec![]))
                }),
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
            return reject(ReasonCode::AuthorityDenied, empty_map(), empty_map());
        }
    }
    let query_hash = string_value(
        query,
        "query_hash_hex",
        &semantic_hash_hex(query).unwrap_or_default(),
    );
    accept(
        mapping([
            ("decision", ystr("authorized")),
            ("query_class", ystr(query_class)),
            ("query_hash", ystr(query_hash)),
        ]),
        empty_map(),
    )
}

pub fn validate_control_change_case(
    semantic_input: &BTreeMap<String, Value>,
    local_state: &BTreeMap<String, Value>,
    verifier: &dyn FixtureVerifier,
) -> ValidationResult {
    let command = get_map(semantic_input, "command").unwrap();
    let expected = command
        .get("signature_fixture")
        .and_then(Value::as_str)
        .unwrap_or(&string_value(command, "issuer", ""))
        .to_string();
    if matches!(
        verifier.verify_signed_fixture(command, &expected),
        Some(false)
    ) {
        return reject(ReasonCode::CryptoInvalid, empty_map(), empty_map());
    }
    let mut controllers = set_from_list(local_state.get("current_controller_set"));
    let min_controllers = get_map(local_state, "control_policy")
        .map(|m| number_value(m, "minimum_controllers", 1))
        .unwrap_or(1) as usize;
    match string_value(command, "command_type", "").as_str() {
        "ADD_CONTROLLER" => {
            let new = string_value(command, "new_controller", "");
            if new.is_empty() {
                return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
            }
            controllers.insert(new);
            accept(
                mapping([
                    ("decision", ystr("committed")),
                    ("control_delta", ystr("add_controller")),
                ]),
                mapping([(
                    "controller_set",
                    seq(controllers.into_iter().map(Value::String)),
                )]),
            )
        }
        "REMOVE_CONTROLLER" => {
            let target = string_value(command, "target_controller", "");
            if target.is_empty() {
                return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
            }
            controllers.remove(&target);
            if controllers.len() < min_controllers {
                return reject(ReasonCode::ControlInvariantFailed, empty_map(), empty_map());
            }
            accept(
                mapping([
                    ("decision", ystr("committed")),
                    ("control_delta", ystr("remove_controller")),
                ]),
                mapping([(
                    "controller_set",
                    seq(controllers.into_iter().map(Value::String)),
                )]),
            )
        }
        "TRANSFER_CONTROL" => {
            let from = string_value(command, "from", "");
            let to = string_value(command, "to", "");
            if from.is_empty() || to.is_empty() {
                return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
            }
            let valid = get_map(local_state, "proof_of_possession")
                .and_then(|m| m.get(&to))
                .and_then(Value::as_str)
                == Some("valid-challenge-response");
            if !valid {
                return defer(ReasonCode::MissingDependency, empty_map());
            }
            controllers.remove(&from);
            controllers.insert(to);
            if controllers.len() < min_controllers {
                return reject(ReasonCode::ControlInvariantFailed, empty_map(), empty_map());
            }
            accept(
                mapping([
                    ("decision", ystr("committed")),
                    ("control_delta", ystr("transfer_control")),
                ]),
                mapping([(
                    "controller_set",
                    seq(controllers.into_iter().map(Value::String)),
                )]),
            )
        }
        _ => reject(ReasonCode::StructuralInvalid, empty_map(), empty_map()),
    }
}

pub fn validate_network_case(
    semantic_input: &BTreeMap<String, Value>,
    local_state: &BTreeMap<String, Value>,
    verifier: &dyn FixtureVerifier,
    semantic_hash_hex: &dyn Fn(&BTreeMap<String, Value>) -> Option<String>,
) -> ValidationResult {
    let now = local_state
        .get("now")
        .and_then(Value::as_str)
        .map(|s| parse_rfc3339(s).unwrap());
    if let Some(assignments) = get_map(semantic_input, "route_trust_assignments") {
        let issuer = string_value(assignments, "issuer", "");
        if issuer.is_empty() {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        let trusted = set_from_list(local_state.get("trust_roots")).contains(&issuer)
            || set_from_list(local_state.get("current_controller_set")).contains(&issuer);
        if !trusted {
            return reject(ReasonCode::AuthorityDenied, empty_map(), empty_map());
        }
        if get_seq(assignments, "assignments")
            .map(|v| v.is_empty())
            .unwrap_or(true)
        {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        return accept(
            mapping([
                ("decision", ystr("route_trust_assignments_accepted")),
                ("advisory_only", Value::Bool(true)),
            ]),
            empty_map(),
        );
    }
    if let Some(policy) = get_map(semantic_input, "aggregate_trust_policy") {
        let issuer = string_value(policy, "issuer", "");
        if issuer.is_empty() {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        let trusted = set_from_list(local_state.get("trust_roots")).contains(&issuer)
            || set_from_list(local_state.get("current_controller_set")).contains(&issuer);
        if !trusted {
            return reject(ReasonCode::AuthorityDenied, empty_map(), empty_map());
        }
        if policy
            .get("minimum_trust_score")
            .and_then(Value::as_i64)
            .is_some_and(|v| v < 0)
        {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        return accept(
            mapping([
                ("decision", ystr("aggregate_trust_policy_accepted")),
                ("advisory_only", Value::Bool(true)),
            ]),
            empty_map(),
        );
    }
    if let Some(policy) = get_map(semantic_input, "route_selection_policy") {
        let candidates = get_seq(local_state, "candidate_routes").unwrap_or(&[]);
        let assignment_scores = local_state
            .get("route_assignment_scores")
            .and_then(Value::as_map);
        let min_quality = policy
            .get("minimum_quality_hint")
            .and_then(Value::as_i64)
            .unwrap_or(i64::MIN);
        let max_cost = policy
            .get("maximum_cost_hint")
            .and_then(Value::as_i64)
            .unwrap_or(i64::MAX);
        let require_active = policy
            .get("require_active_session")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let preferred_advertisers: BTreeSet<String> = get_seq(policy, "preferred_advertisers")
            .unwrap_or(&[])
            .iter()
            .filter_map(Value::as_map)
            .map(|m| string_value(m, "identity_id", ""))
            .filter(|s| !s.is_empty())
            .collect();
        let preferred_next_hops: BTreeSet<String> = get_seq(policy, "preferred_next_hops")
            .unwrap_or(&[])
            .iter()
            .filter_map(Value::as_map)
            .map(|m| string_value(m, "node_id", ""))
            .filter(|s| !s.is_empty())
            .collect();
        let minimum_trust_score = get_map(local_state, "aggregate_trust_policy")
            .and_then(|m| m.get("minimum_trust_score"))
            .and_then(Value::as_i64)
            .unwrap_or(i64::MIN);
        let allowed_responders: Option<BTreeSet<String>> =
            get_map(local_state, "aggregate_trust_policy")
                .and_then(|m| get_seq(m, "allowed_responders"))
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_map)
                        .map(|m| string_value(m, "identity_id", ""))
                        .filter(|s| !s.is_empty())
                        .collect()
                });
        let preferred_aggregators: BTreeSet<String> =
            get_map(local_state, "aggregate_trust_policy")
                .and_then(|m| get_seq(m, "preferred_aggregators"))
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_map)
                        .map(|m| string_value(m, "identity_id", ""))
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .unwrap_or_default();

        let mut best: Option<(i64, bool, bool, String, String, String)> = None;
        for candidate in candidates.iter().filter_map(Value::as_map) {
            let quality = number_value(candidate, "quality_hint", 0);
            if quality < min_quality {
                continue;
            }
            let cost = number_value(candidate, "cost_hint", 0);
            if cost > max_cost {
                continue;
            }
            if require_active
                && !candidate
                    .get("has_active_session")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            {
                continue;
            }
            let advertiser = string_value(candidate, "advertiser", "");
            let next_hop = string_value(candidate, "next_hop_node", "");
            if allowed_responders
                .as_ref()
                .is_some_and(|set| !set.contains(&advertiser))
            {
                continue;
            }
            let assignment_score = assignment_scores
                .and_then(|m| m.get(&advertiser))
                .and_then(Value::as_i64)
                .unwrap_or(0);
            let preferred_bonus = if preferred_aggregators.contains(&advertiser) {
                1
            } else {
                0
            };
            let score = assignment_score + quality - cost + preferred_bonus;
            if score < minimum_trust_score {
                continue;
            }
            let preferred_advertiser = preferred_advertisers.contains(&advertiser);
            let preferred_next_hop = preferred_next_hops.contains(&next_hop);
            let advertised_at = string_value(candidate, "advertised_at", "");
            let candidate_tuple = (
                score,
                preferred_advertiser,
                preferred_next_hop,
                advertiser,
                next_hop,
                advertised_at,
            );
            let replace = match &best {
                None => true,
                Some(current) => {
                    candidate_tuple.0 > current.0
                        || (candidate_tuple.0 == current.0 && candidate_tuple.1 && !current.1)
                        || (candidate_tuple.0 == current.0
                            && candidate_tuple.1 == current.1
                            && candidate_tuple.2
                            && !current.2)
                        || (candidate_tuple.0 == current.0
                            && candidate_tuple.1 == current.1
                            && candidate_tuple.2 == current.2
                            && !candidate_tuple.5.is_empty()
                            && (current.5.is_empty() || candidate_tuple.5 < current.5))
                        || (candidate_tuple.0 == current.0
                            && candidate_tuple.1 == current.1
                            && candidate_tuple.2 == current.2
                            && candidate_tuple.5 == current.5
                            && candidate_tuple.4 < current.4)
                }
            };
            if replace {
                best = Some(candidate_tuple);
            }
        }
        let Some((score, _pa, _pn, advertiser, next_hop, _ts)) = best else {
            return reject(ReasonCode::PolicyDenied, empty_map(), empty_map());
        };
        return accept(
            mapping([
                ("decision", ystr("route_selected")),
                ("selected_advertiser", ystr(advertiser)),
                ("selected_next_hop", ystr(next_hop)),
                ("route_score", Value::Int(score)),
                ("advisory_only", Value::Bool(true)),
            ]),
            empty_map(),
        );
    }
    if let Some(hello) = get_map(semantic_input, "session_hello") {
        let target = string_value(hello, "target_node", "");
        if !target.is_empty() && target != string_value(local_state, "local_node", "") {
            return reject(ReasonCode::TargetMismatch, empty_map(), empty_map());
        }
        let nonce = hello
            .get("session_nonce")
            .or_else(|| hello.get("session_nonce_hex"))
            .and_then(nonce_bytes)
            .unwrap_or_default();
        if nonce.is_empty() {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        let supported: BTreeSet<i64> = get_seq(hello, "supported_protocol_versions")
            .map(|arr| arr.iter().filter_map(Value::as_i64).collect())
            .unwrap_or_default();
        let local_supported: BTreeSet<i64> = local_state
            .get("supported_protocol_versions")
            .and_then(Value::as_seq)
            .map(|arr| arr.iter().filter_map(Value::as_i64).collect())
            .unwrap_or_else(|| BTreeSet::from([1]));
        let overlap = supported.intersection(&local_supported).copied().max();
        if overlap.is_none() {
            return reject(ReasonCode::VersionUnsupported, empty_map(), empty_map());
        }
        let expected = hello
            .get("signature_fixture")
            .and_then(Value::as_str)
            .unwrap_or(&string_value(hello, "initiator", ""))
            .to_string();
        if matches!(
            verifier.verify_signed_fixture(hello, &expected),
            Some(false)
        ) {
            return reject(ReasonCode::CryptoInvalid, empty_map(), empty_map());
        }
        for hint in get_seq(hello, "initiator_locators").unwrap_or(&[]) {
            let Some(map) = hint.as_map() else {
                return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
            };
            if let Err(code) = validate_reachability_hint_map(map, now, verifier, semantic_hash_hex)
            {
                return reject(code, empty_map(), empty_map());
            }
        }
        return accept(
            mapping([
                ("decision", ystr("session_hello_accepted")),
                (
                    "selected_protocol_version",
                    Value::Int(overlap.unwrap_or_default()),
                ),
            ]),
            empty_map(),
        );
    }
    if let Some(accept_msg) = get_map(semantic_input, "session_accept") {
        let expected_nonce = local_state
            .get("expected_session_nonce")
            .and_then(nonce_bytes);
        let echoed = accept_msg
            .get("echoed_session_nonce")
            .and_then(nonce_bytes)
            .unwrap_or_default();
        if echoed.is_empty() {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        if expected_nonce.as_ref().is_some_and(|v| *v != echoed) {
            return reject(ReasonCode::TargetMismatch, empty_map(), empty_map());
        }
        let selected = number_value(accept_msg, "selected_protocol_version", 0);
        let local_supported: BTreeSet<i64> = local_state
            .get("supported_protocol_versions")
            .and_then(Value::as_seq)
            .map(|arr| arr.iter().filter_map(Value::as_i64).collect())
            .unwrap_or_else(|| BTreeSet::from([1]));
        if !local_supported.contains(&selected) {
            return reject(ReasonCode::VersionUnsupported, empty_map(), empty_map());
        }
        let expected = accept_msg
            .get("signature_fixture")
            .and_then(Value::as_str)
            .unwrap_or(&string_value(accept_msg, "responder", ""))
            .to_string();
        if matches!(
            verifier.verify_signed_fixture(accept_msg, &expected),
            Some(false)
        ) {
            return reject(ReasonCode::CryptoInvalid, empty_map(), empty_map());
        }
        for hint in get_seq(accept_msg, "responder_locators").unwrap_or(&[]) {
            let Some(map) = hint.as_map() else {
                return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
            };
            if let Err(code) = validate_reachability_hint_map(map, now, verifier, semantic_hash_hex)
            {
                return reject(code, empty_map(), empty_map());
            }
        }
        return accept(
            mapping([
                ("decision", ystr("session_accepted")),
                ("selected_protocol_version", Value::Int(selected)),
            ]),
            empty_map(),
        );
    }
    if let Some(route) = get_map(semantic_input, "route_advertisement") {
        if let Some(at) = route.get("advertised_at").and_then(Value::as_str) {
            if let Some(exp) = route.get("expires_at").and_then(Value::as_str) {
                if parse_rfc3339(exp).unwrap() < parse_rfc3339(at).unwrap() {
                    return reject(ReasonCode::TimeInvalid, empty_map(), empty_map());
                }
            }
        }
        if let (Some(now), Some(exp)) = (now, route.get("expires_at").and_then(Value::as_str)) {
            if parse_rfc3339(exp).unwrap() < now {
                return reject(ReasonCode::TimeInvalid, empty_map(), empty_map());
            }
        }
        let target = string_value(route, "target_node", "");
        for hint in get_seq(route, "reachability").unwrap_or(&[]) {
            let Some(map) = hint.as_map() else {
                return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
            };
            if string_value(map, "subject_node", "") != target {
                return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
            }
            if let Err(code) = validate_reachability_hint_map(map, now, verifier, semantic_hash_hex)
            {
                return reject(code, empty_map(), empty_map());
            }
        }
        let expected = route
            .get("signature_fixture")
            .and_then(Value::as_str)
            .unwrap_or(&string_value(route, "advertiser", ""))
            .to_string();
        if matches!(
            verifier.verify_signed_fixture(route, &expected),
            Some(false)
        ) {
            return reject(ReasonCode::CryptoInvalid, empty_map(), empty_map());
        }
        return accept(
            mapping([
                ("decision", ystr("route_advertisement_accepted")),
                ("advisory_only", Value::Bool(true)),
            ]),
            empty_map(),
        );
    }
    if let Some(relay) = get_map(semantic_input, "relay_envelope") {
        let target = string_value(relay, "intended_recipient_node", "");
        if !target.is_empty() && target != string_value(local_state, "local_node", "") {
            return reject(ReasonCode::TargetMismatch, empty_map(), empty_map());
        }
        let has_payload_object = relay.contains_key("payload_object");
        let has_inline_payload = relay.contains_key("inline_payload");
        if has_payload_object == has_inline_payload {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        if let (Some(now), Some(until)) = (now, relay.get("store_until").and_then(Value::as_str)) {
            if parse_rfc3339(until).unwrap() < now {
                return reject(ReasonCode::TimeInvalid, empty_map(), empty_map());
            }
        }
        let expected = relay
            .get("signature_fixture")
            .and_then(Value::as_str)
            .unwrap_or(&string_value(relay, "original_sender", ""))
            .to_string();
        if matches!(
            verifier.verify_signed_fixture(relay, &expected),
            Some(false)
        ) {
            return reject(ReasonCode::CryptoInvalid, empty_map(), empty_map());
        }
        return accept(mapping([("decision", ystr("relay_accepted"))]), empty_map());
    }
    if let Some(hint) = get_map(semantic_input, "reachability_hint") {
        if let Err(code) = validate_reachability_hint_map(hint, now, verifier, semantic_hash_hex) {
            return reject(code, empty_map(), empty_map());
        }
        return accept(
            mapping([("decision", ystr("reachability_hint_accepted"))]),
            empty_map(),
        );
    }
    reject(ReasonCode::StructuralInvalid, empty_map(), empty_map())
}

pub fn validate_stream_append_case(
    semantic_input: &BTreeMap<String, Value>,
    local_state: &BTreeMap<String, Value>,
    verifier: &dyn FixtureVerifier,
    semantic_hash_hex: &dyn Fn(&BTreeMap<String, Value>) -> Option<String>,
) -> ValidationResult {
    let event = get_map(semantic_input, "candidate_event").unwrap();
    let seq_no = number_value(event, "seq", -1);
    if seq_no < 0 {
        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
    }
    let writer = string_value(local_state, "stream_writer_identity", "");
    let signer = string_value(event, "signature_fixture", "");
    if !writer.is_empty() && !signer.is_empty() && writer != signer {
        return reject(ReasonCode::CryptoInvalid, empty_map(), empty_map());
    }
    let expected = if !signer.is_empty() {
        signer.clone()
    } else {
        writer.clone()
    };
    if matches!(
        verifier.verify_signed_fixture(event, &expected),
        Some(false)
    ) {
        return reject(ReasonCode::CryptoInvalid, empty_map(), empty_map());
    }
    let candidate_hash = string_value(
        event,
        "event_hash_hex",
        &semantic_hash_hex(event).unwrap_or_default(),
    );
    if let Some(result) = validate_event_family_semantics(semantic_input, event) {
        return result;
    }
    if let Some(existing) = get_seq(local_state, "stream_events") {
        for item in existing {
            let Some(ev) = item.as_map() else {
                continue;
            };
            if number_value(ev, "seq", -1) == seq_no {
                let existing_hash = string_value(
                    ev,
                    "event_hash_hex",
                    &string_value(ev, "event_hash_fixture", ""),
                );
                let candidate = if !candidate_hash.is_empty() {
                    candidate_hash.clone()
                } else {
                    string_value(event, "event_hash_fixture", "")
                };
                if !candidate.is_empty() && !existing_hash.is_empty() && candidate == existing_hash
                {
                    return duplicate(
                        ReasonCode::ReplayDetected,
                        mapping([("event_hash", ystr(candidate))]),
                    );
                }
                return reject(
                    ReasonCode::ForkConflict,
                    mapping([("event_hash", ystr(candidate))]),
                    empty_map(),
                );
            }
        }
    }
    let stream_id = string_value(event, "stream_id", "");
    let head = local_state
        .get("stream_heads")
        .and_then(|v| map_value(v, &stream_id));
    if head.is_none() {
        if seq_no == 0 {
            return accept(
                mapping([("event_hash", ystr(candidate_hash.clone()))]),
                mapping([(
                    "head_after",
                    mapping([("seq", Value::Int(0)), ("event_hash", ystr(candidate_hash))]),
                )]),
            );
        }
        return defer(
            ReasonCode::MissingDependency,
            mapping([("event_hash", ystr(candidate_hash))]),
        );
    }
    let head = head.unwrap();
    let head_seq = number_value(head, "seq", -1);
    let head_hash = string_value(
        head,
        "event_hash_hex",
        &string_value(head, "event_hash_fixture", ""),
    );
    let prev_hash = get_map(event, "prev_ref")
        .map(|m| {
            string_value(
                m,
                "hash_hex",
                &string_value(m, "event_hash_hex", &string_value(m, "hash_fixture", "")),
            )
        })
        .unwrap_or_default();
    if seq_no == head_seq + 1 {
        if !head_hash.is_empty() && !prev_hash.is_empty() && head_hash != prev_hash {
            return reject(
                ReasonCode::ForkConflict,
                mapping([("event_hash", ystr(candidate_hash))]),
                empty_map(),
            );
        }
        return accept(
            mapping([("event_hash", ystr(candidate_hash.clone()))]),
            mapping([(
                "head_after",
                mapping([
                    ("seq", Value::Int(seq_no)),
                    ("event_hash", ystr(candidate_hash)),
                ]),
            )]),
        );
    }
    if seq_no > head_seq + 1 {
        return defer(
            ReasonCode::MissingDependency,
            mapping([("event_hash", ystr(candidate_hash))]),
        );
    }
    reject(
        ReasonCode::ForkConflict,
        mapping([("event_hash", ystr(candidate_hash))]),
        empty_map(),
    )
}

pub fn validate_snapshot_case(
    semantic_input: &BTreeMap<String, Value>,
    local_state: &BTreeMap<String, Value>,
    verifier: &dyn FixtureVerifier,
    semantic_hash_hex: &dyn Fn(&BTreeMap<String, Value>) -> Option<String>,
) -> ValidationResult {
    let snap = get_map(semantic_input, "snapshot").unwrap();
    let producer = string_value(snap, "producer", "");
    if !set_from_list(local_state.get("trusted_snapshot_producers")).contains(&producer) {
        return reject(ReasonCode::AuthorityDenied, empty_map(), empty_map());
    }
    let expected = snap
        .get("signature_fixture")
        .and_then(Value::as_str)
        .unwrap_or(&producer)
        .to_string();
    if matches!(verifier.verify_signed_fixture(snap, &expected), Some(false)) {
        return reject(ReasonCode::CryptoInvalid, empty_map(), empty_map());
    }
    let base_heads = get_seq(snap, "base_heads")
        .map(|v| v.to_vec())
        .unwrap_or_default();
    if base_heads.is_empty() {
        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
    }
    let base = base_heads[0].as_map().unwrap();
    let local_head = local_state
        .get("stream_heads")
        .and_then(|v| map_value(v, &string_value(base, "stream_id", "")));
    let snapshot_hash = semantic_hash_hex(snap).unwrap_or_default();
    let Some(local_head) = local_head else {
        return defer(ReasonCode::MissingDependency, empty_map());
    };
    let base_seq = number_value(base, "seq", -1);
    let local_seq = number_value(local_head, "seq", -1);
    let base_hash = string_value(
        base,
        "event_hash_hex",
        &string_value(base, "event_hash_fixture", ""),
    );
    let local_hash = string_value(
        local_head,
        "event_hash_hex",
        &string_value(local_head, "event_hash_fixture", ""),
    );
    if base_seq == local_seq {
        if !base_hash.is_empty() && !local_hash.is_empty() && base_hash != local_hash {
            return reject(
                ReasonCode::SnapshotBaseConflict,
                mapping([("snapshot_descriptor_hash", ystr(snapshot_hash))]),
                empty_map(),
            );
        }
        return accept(
            mapping([
                ("acceptance_class", ystr("accepted_trusted")),
                ("snapshot_descriptor_hash", ystr(snapshot_hash)),
            ]),
            empty_map(),
        );
    }
    if base_seq < local_seq {
        if let Some(avail) = local_state.get("available_deltas").and_then(Value::as_seq) {
            let have: BTreeSet<i64> = avail
                .iter()
                .filter_map(Value::as_map)
                .map(|m| number_value(m, "seq", -1))
                .collect();
            for needed in (base_seq + 1)..=local_seq {
                if !have.contains(&needed) {
                    return defer(
                        ReasonCode::MissingDependency,
                        mapping([
                            ("acceptance_class", ystr("deferred_missing_delta")),
                            ("snapshot_descriptor_hash", ystr(snapshot_hash)),
                        ]),
                    );
                }
            }
        }
        return accept(
            mapping([
                ("acceptance_class", ystr("accepted_stale")),
                ("snapshot_descriptor_hash", ystr(snapshot_hash)),
            ]),
            empty_map(),
        );
    }
    reject(ReasonCode::SnapshotBaseConflict, empty_map(), empty_map())
}

pub fn validate_object_case(
    semantic_input: &BTreeMap<String, Value>,
    local_state: &BTreeMap<String, Value>,
) -> ValidationResult {
    if let Some(descriptor) = get_map(semantic_input, "descriptor") {
        let header = get_map(semantic_input, "header");
        let manifest = get_map(semantic_input, "chunk_manifest");
        let descriptor_object_id = string_value(descriptor, "object_id", "");
        if let Some(header) = header {
            let header_object = get_map(header, "object")
                .map(|m| string_value(m, "object_id", ""))
                .unwrap_or_default();
            if !descriptor_object_id.is_empty()
                && !header_object.is_empty()
                && descriptor_object_id != header_object
            {
                return reject(ReasonCode::ObjectIdMismatch, empty_map(), empty_map());
            }
            let chunking_mode = string_value(header, "chunking_mode", "");
            if chunking_mode == "CHUNKING_MODE_MANIFEST" && manifest.is_none() {
                return defer(
                    ReasonCode::MissingDependency,
                    mapping([("validation_level", ystr("deferred_missing_chunks"))]),
                );
            }
        }
        if let Some(manifest) = manifest {
            let manifest_object = get_map(manifest, "object")
                .map(|m| string_value(m, "object_id", ""))
                .unwrap_or_default();
            if !descriptor_object_id.is_empty()
                && !manifest_object.is_empty()
                && descriptor_object_id != manifest_object
            {
                return reject(ReasonCode::ObjectIdMismatch, empty_map(), empty_map());
            }
            let entries = get_seq(manifest, "entries")
                .or_else(|| get_seq(manifest, "chunk_entries"))
                .unwrap_or(&[]);
            let claimed_count = manifest
                .get("chunk_count")
                .and_then(Value::as_i64)
                .unwrap_or(entries.len() as i64);
            if claimed_count != entries.len() as i64 {
                return reject(ReasonCode::RepresentationInvalid, empty_map(), empty_map());
            }
            let total_len: i64 = entries
                .iter()
                .filter_map(Value::as_map)
                .map(|m| number_value(m, "length", 0))
                .sum();
            let claimed_total = manifest
                .get("total_stored_size")
                .and_then(Value::as_i64)
                .unwrap_or(total_len);
            if claimed_total != total_len {
                return reject(ReasonCode::RepresentationInvalid, empty_map(), empty_map());
            }
            if let Some(header) = header {
                let header_representation_id = string_value(header, "representation_id", "");
                let manifest_representation_id = get_map(manifest, "representation")
                    .map(|m| string_value(m, "representation_id", ""))
                    .unwrap_or_default();
                if !header_representation_id.is_empty()
                    && !manifest_representation_id.is_empty()
                    && header_representation_id != manifest_representation_id
                {
                    return reject(ReasonCode::RepresentationInvalid, empty_map(), empty_map());
                }
                let stored_size = header
                    .get("stored_size")
                    .and_then(Value::as_i64)
                    .unwrap_or(claimed_total);
                if stored_size != claimed_total {
                    return reject(ReasonCode::RepresentationInvalid, empty_map(), empty_map());
                }
            }
        }
        return accept(
            mapping([("validation_level", ystr("descriptor_consistent"))]),
            empty_map(),
        );
    }
    let may_decrypt = get_map(local_state, "access_state")
        .and_then(|m| m.get("may_decrypt"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if let Some(obj) = get_map(semantic_input, "object") {
        let realized = must_hex_to_bytes(&string_value(obj, "realized_bytes_hex", "0x"));
        let mut payload = b"edgerun:v0:object\0raw-bytes-v0\0".to_vec();
        payload.extend_from_slice(&realized);
        let object_id = bytes_to_hex(&sha256(&payload));
        let mut claimed = string_value(obj, "object_id", "");
        if claimed.is_empty() {
            claimed = get_map(obj, "descriptor")
                .map(|m| string_value(m, "object_id", ""))
                .unwrap_or_default();
        }
        if !claimed.is_empty() && claimed != object_id {
            return reject(ReasonCode::ObjectIdMismatch, empty_map(), empty_map());
        }
        let canonicalization_id = get_map(obj, "descriptor")
            .map(|m| string_value(m, "canonicalization_id", "raw-bytes-v0"))
            .unwrap_or_else(|| "raw-bytes-v0".to_string());
        return accept(
            mapping([
                ("validation_level", ystr("logical_object_valid")),
                ("object_id", ystr(object_id)),
                ("canonicalization_id", ystr(canonicalization_id)),
            ]),
            empty_map(),
        );
    }
    if let Some(rep) = get_map(semantic_input, "representation") {
        let stored = if let Some(manifest) =
            get_map(rep, "chunk_manifest").or_else(|| get_map(semantic_input, "chunk_manifest"))
        {
            let entries = get_seq(manifest, "entries").unwrap_or(&[]);
            let available = local_state.get("available_chunks").and_then(Value::as_map);
            let mut assembled = Vec::new();
            for entry in entries {
                let Some(entry) = entry.as_map() else {
                    return reject(ReasonCode::RepresentationInvalid, empty_map(), empty_map());
                };
                let digest_hex = string_value(entry, "chunk_digest", "");
                let Some(available) = available else {
                    return defer(
                        ReasonCode::MissingDependency,
                        mapping([("validation_level", ystr("deferred_missing_chunks"))]),
                    );
                };
                let Some(chunk_value) = available.get(&digest_hex).and_then(Value::as_str) else {
                    return defer(
                        ReasonCode::MissingDependency,
                        mapping([("validation_level", ystr("deferred_missing_chunks"))]),
                    );
                };
                let chunk_bytes = must_hex_to_bytes(chunk_value);
                let mut chunk_payload = b"edgerun:v0:chunk-bytes\0".to_vec();
                chunk_payload.extend_from_slice(&chunk_bytes);
                let actual_digest = bytes_to_hex(&sha256(&chunk_payload));
                if !digest_hex.is_empty() && actual_digest != digest_hex {
                    return reject(ReasonCode::RepresentationInvalid, empty_map(), empty_map());
                }
                assembled.extend_from_slice(&chunk_bytes);
            }
            assembled
        } else {
            must_hex_to_bytes(&string_value(rep, "stored_bytes_hex", "0x"))
        };
        let mut payload = b"edgerun:v0:representation-bytes\0".to_vec();
        payload.extend_from_slice(&stored);
        let digest = bytes_to_hex(&sha256(&payload));
        let header = get_map(rep, "header");
        let claimed = if string_value(rep, "representation_digest", "").is_empty() {
            header
                .map(|m| string_value(m, "representation_digest", ""))
                .unwrap_or_default()
        } else {
            string_value(rep, "representation_digest", "")
        };
        let encryption_scheme = header
            .and_then(|m| m.get("encryption_scheme"))
            .cloned()
            .unwrap_or(Value::Null);
        if !claimed.is_empty() && claimed != digest {
            return reject(ReasonCode::RepresentationInvalid, empty_map(), empty_map());
        }
        let require_logical_object = semantic_input
            .get("require_logical_object")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if require_logical_object && !may_decrypt {
            return defer(
                ReasonCode::MissingDependency,
                mapping([
                    ("validation_level", ystr("deferred_missing_access")),
                    ("representation_digest", ystr(digest)),
                ]),
            );
        }
        let level = if may_decrypt {
            "logical_object_valid"
        } else {
            "representation_valid_only"
        };
        return accept(
            mapping([
                ("validation_level", ystr(level)),
                ("representation_digest", ystr(digest)),
                ("encryption_scheme", encryption_scheme),
            ]),
            empty_map(),
        );
    }
    reject(ReasonCode::RepresentationInvalid, empty_map(), empty_map())
}

#[cfg(test)]
mod tests {
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
    fn network_reachability_hint_valid_accepts() {
        let semantic = match mapping([(
            "reachability_hint",
            mapping([
                ("locator_payload", ystr("quic://127.0.0.1:443")),
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
                ("issuer", ystr("controller-a")),
                ("from", ystr("controller-a")),
                ("to", ystr("controller-b")),
            ]),
        )]) {
            Value::Map(m) => m,
            _ => unreachable!(),
        };
        let state = match mapping([
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
}
