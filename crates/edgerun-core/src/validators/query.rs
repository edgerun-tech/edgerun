use crate::prelude::v1::*;

use super::delegation::validate_delegation_case;
use super::helpers::*;
pub fn validate_query_case(
    semantic_input: &BTreeMap<String, Value>,
    local_state: &BTreeMap<String, Value>,
    verifier: &dyn FixtureVerifier,
    semantic_hash_hex: &dyn Fn(&BTreeMap<String, Value>) -> Option<String>,
) -> ValidationResult {
    if let Some(proof) = get_map(semantic_input, "snapshot_set_proof") {
        if let Some(result) = validate_source_query_id(
            proof,
            local_state.get("current_query_id").and_then(Value::as_str),
        ) {
            return result;
        }
        let snapshots = get_seq(proof, "snapshots").unwrap_or(&[]);
        if snapshots.is_empty() {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        for snapshot in snapshots.iter().filter_map(Value::as_map) {
            if string_value(snapshot, "snapshot_id", "").is_empty() {
                return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
            }
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
        if let Some(result) = validate_source_query_id(
            proof,
            local_state.get("current_query_id").and_then(Value::as_str),
        ) {
            return result;
        }
        let events = get_seq(proof, "events").unwrap_or(&[]);
        if events.is_empty() {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        for event in events.iter().filter_map(Value::as_map) {
            if !event_ref_is_valid(event) {
                return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
            }
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
                if object_id.is_empty() {
                    return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
                }
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
        if let Some(result) = validate_source_query_id(
            proof,
            local_state.get("current_query_id").and_then(Value::as_str),
        ) {
            return result;
        }
        let Some(object_ref) = get_map(proof, "object_ref") else {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        };
        let object_id = string_value(object_ref, "object_id", "");
        if object_id.is_empty() {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
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
                if bundled_id.is_empty() {
                    return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
                }
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
        if let Some(result) = validate_source_query_id(
            proof,
            local_state.get("current_query_id").and_then(Value::as_str),
        ) {
            return result;
        }
        let included = get_seq(proof, "included_responders").unwrap_or(&[]);
        let excluded = get_seq(proof, "excluded_responders").unwrap_or(&[]);
        let included_ids: BTreeSet<String> = included
            .iter()
            .filter_map(Value::as_map)
            .map(|m| string_value(m, "identity_id", ""))
            .filter(|s| !s.is_empty())
            .collect();
        if included
            .iter()
            .chain(excluded.iter())
            .filter_map(Value::as_map)
            .any(|m| string_value(m, "identity_id", "").is_empty())
        {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
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
            if object_id.is_empty() {
                return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
            }
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
        if let Some(result) = validate_source_query_id(
            proof,
            local_state.get("current_query_id").and_then(Value::as_str),
        ) {
            return result;
        }
        if !proof.contains_key("policy_object") && !proof.contains_key("assignments_object") {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        if let Some(available) = local_state.get("available_objects").and_then(Value::as_map) {
            for key in ["policy_object", "assignments_object"] {
                if let Some(obj) = get_map(proof, key) {
                    let object_id = string_value(obj, "object_id", "");
                    if object_id.is_empty() {
                        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
                    }
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
        let actual = string_value(fragment, "query_id", "");
        if actual.is_empty() {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        if let Some(expected_query_id) = local_state.get("current_query_id").and_then(Value::as_str)
        {
            if actual != expected_query_id {
                return reject(ReasonCode::TargetMismatch, empty_map(), empty_map());
            }
        }
        if string_value(fragment, "responder", "").is_empty()
            || string_value(fragment, "completeness", "").is_empty()
        {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
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
        if let Some(snapshot_refs) = get_seq(fragment, "snapshot_refs") {
            for snapshot in snapshot_refs.iter().filter_map(Value::as_map) {
                if string_value(snapshot, "snapshot_id", "").is_empty() {
                    return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
                }
            }
        }
        if let Some(event_refs) = get_seq(fragment, "event_refs") {
            for event in event_refs.iter().filter_map(Value::as_map) {
                if !event_ref_is_valid(event) {
                    return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
                }
            }
        }
        for key in ["object_refs", "proof_objects"] {
            if let Some(objects) = get_seq(fragment, key) {
                for object in objects.iter().filter_map(Value::as_map) {
                    if string_value(object, "object_id", "").is_empty() {
                        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
                    }
                }
            }
        }
        for key in ["bundled_result_object", "result_metadata"] {
            if let Some(object) = get_map(fragment, key) {
                if string_value(object, "object_id", "").is_empty() {
                    return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
                }
            }
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
        if let Some(result) = validate_source_query_id(
            aggregate,
            local_state.get("current_query_id").and_then(Value::as_str),
        ) {
            return result;
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
        if let Some(result) = validate_source_query_id(
            bundle,
            local_state.get("current_query_id").and_then(Value::as_str),
        ) {
            return result;
        }
        let payload_type = string_value(bundle, "payload_type", "");
        if payload_type.is_empty() {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        let Some(payload_object) = get_map(bundle, "payload_object") else {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        };
        if string_value(payload_object, "object_id", "").is_empty() {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        let allowed = set_from_list(local_state.get("allowed_proof_payload_types"));
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
    let Some(query) = get_map(semantic_input, "query") else {
        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
    };
    if !query_required_fields_are_present(query) {
        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
    }
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
    if query
        .get("result_limit")
        .and_then(Value::as_i64)
        .is_some_and(|v| v <= 0)
    {
        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
    }
    if let Some(required_proof_classes) = get_seq(query, "required_proof_classes") {
        if required_proof_classes
            .iter()
            .any(|proof_class| proof_class.as_str().is_none_or(str::is_empty))
        {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
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
        if ["max_results", "max_total_bytes", "max_federated_responders"]
            .iter()
            .any(|key| {
                cost.get(*key)
                    .and_then(Value::as_i64)
                    .is_some_and(|v| v <= 0)
            })
        {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
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

fn event_ref_is_valid(event: &BTreeMap<String, Value>) -> bool {
    !string_value(event, "stream_id", "").is_empty()
        && number_value(event, "seq", -1) >= 0
        && (event.contains_key("event_hash")
            || event.contains_key("event_hash_hex")
            || event.contains_key("hash_hex")
            || event.contains_key("hash_fixture"))
}

fn query_required_fields_are_present(query: &BTreeMap<String, Value>) -> bool {
    !string_value(query, "query_id", "").is_empty()
        && !string_value(query, "requester", "").is_empty()
        && !string_value(query, "query_class", "").is_empty()
        && query
            .get("target_scope")
            .is_some_and(|scope| !matches!(scope, Value::Null))
}

fn validate_source_query_id(
    artifact: &BTreeMap<String, Value>,
    expected_query_id: Option<&str>,
) -> Option<ValidationResult> {
    let actual = string_value(artifact, "source_query_id", "");
    if actual.is_empty() {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            empty_map(),
            empty_map(),
        ));
    }
    if expected_query_id.is_some_and(|expected| actual != expected) {
        return Some(reject(ReasonCode::TargetMismatch, empty_map(), empty_map()));
    }
    None
}
