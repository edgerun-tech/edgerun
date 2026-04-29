use crate::prelude::v1::*;

use super::event_semantics::validate_event_family_semantics;
use super::helpers::*;
pub fn validate_stream_append_case(
    semantic_input: &BTreeMap<String, Value>,
    local_state: &BTreeMap<String, Value>,
    verifier: &dyn FixtureVerifier,
    semantic_hash_hex: &dyn Fn(&BTreeMap<String, Value>) -> Option<String>,
) -> ValidationResult {
    let Some(event) = get_map(semantic_input, "candidate_event") else {
        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
    };
    for field in ["envelope_version", "event_version"] {
        if let Some(reason) = version_field_error(event, field) {
            return reject(reason, empty_map(), empty_map());
        }
    }
    if string_value(event, "recorded_at", "").is_empty() || get_map(event, "signature").is_none() {
        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
    }
    if string_value(event, "event_type", "").is_empty() {
        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
    }
    let stream_id = string_value(event, "stream_id", "");
    if stream_id.is_empty() {
        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
    }
    if !event_refs_are_valid(event) {
        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
    }
    let seq_no = number_value(event, "seq", -1);
    if seq_no < 0 {
        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
    }
    if seq_no == 0 {
        if event.contains_key("prev_ref") {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
    } else if get_map(event, "prev_ref").is_none_or(|prev| event_hash_alias_value(prev).is_empty())
    {
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
    // Per spec §18.4: POSITION_CHECK comes before FAMILY_CHECK
    // Check for duplicate seq in existing events (part of POSITION_CHECK)
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
    let stream_heads = local_state.get("stream_heads").and_then(|v| v.as_map());
    let head = stream_heads.and_then(|m| m.get(&stream_id).and_then(|hv| hv.as_map()));
    if head.is_none() {
        if seq_no == 0 {
            // Per spec §18.4: FAMILY_CHECK after POSITION_CHECK for genesis
            if let Some(result) = validate_event_family_semantics(semantic_input, event) {
                return result;
            }
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
        // Per spec §18.4: FAMILY_CHECK (event semantics) comes after POSITION_CHECK
        if let Some(result) = validate_event_family_semantics(semantic_input, event) {
            return result;
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

fn event_refs_are_valid(event: &BTreeMap<String, Value>) -> bool {
    object_ref_is_valid(event.get("payload_object"))
        && object_ref_is_valid(event.get("event_metadata"))
        && ref_list_is_valid(get_seq(event, "related_objects"), object_ref_value_is_valid)
        && ref_list_is_valid(get_seq(event, "related_events"), event_ref_value_is_valid)
}

fn event_ref_value_is_valid(value: &Value) -> bool {
    let Value::Map(map) = value else {
        return false;
    };
    !string_value(map, "stream_id", "").is_empty() && !event_hash_alias_value(map).is_empty()
}

fn ref_list_is_valid(items: Option<&[Value]>, item_valid: fn(&Value) -> bool) -> bool {
    items.unwrap_or(&[]).iter().all(|item| item_valid(item))
}
