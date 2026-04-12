use super::helpers::*;
pub fn validate_trust_case(
    semantic_input: &BTreeMap<String, Value>,
    local_state: &BTreeMap<String, Value>,
    verifier: &dyn FixtureVerifier,
    semantic_hash_hex: &dyn Fn(&BTreeMap<String, Value>) -> Option<String>,
) -> ValidationResult {
    let now = parse_ts(&string_value(local_state, "now", "1970-01-01T00:00:00Z")).unwrap_or(crate::util::DateTimeUtc::epoch());
    if let Some(claim) = get_map(semantic_input, "assurance_claim") {
        if !claim.contains_key("subject_identity") && !claim.contains_key("subject_node") {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        if let Some(s) = claim.get("issued_at").and_then(Value::as_str) {
            if parse_ts(s).map_or(false, |t| t > now) {
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
            if parse_ts(s).map_or(true, |t| t < now) {
                return reject(ReasonCode::TimeInvalid, empty_map(), empty_map());
            }
        }
        if let Some(max_age) = local_state
            .get("max_assurance_evidence_age_seconds")
            .and_then(Value::as_i64)
        {
            if let Some(issued) = claim.get("issued_at").and_then(Value::as_str) {
                let Ok(issued_dt) = parse_ts(issued) else {
                    return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
                };
                let age_secs = now.duration_secs(&issued_dt) as i64;
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
            if parse_ts(s).map_or(false, |t| t > now) {
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
