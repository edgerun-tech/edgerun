use crate::prelude::v1::*;

use super::helpers::*;
pub fn validate_snapshot_case(
    semantic_input: &BTreeMap<String, Value>,
    local_state: &BTreeMap<String, Value>,
    verifier: &dyn FixtureVerifier,
    semantic_hash_hex: &dyn Fn(&BTreeMap<String, Value>) -> Option<String>,
) -> ValidationResult {
    let Some(snap) = get_map(semantic_input, "snapshot") else {
        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
    };
    if let Some(Value::String(snapshot_id)) = snap.get("snapshot_id") {
        if snapshot_id.is_empty() {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
    }
    let producer = string_value(snap, "producer", "");
    if producer.is_empty() {
        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
    }
    if !snapshot_refs_are_valid(snap) {
        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
    }
    if !snap.contains_key("payload_object") || snap.get("payload_object") == Some(&Value::Null) {
        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
    }
    let base_heads = get_seq(snap, "base_heads")
        .map(|v| v.to_vec())
        .unwrap_or_default();
    let base_checkpoints = get_seq(snap, "base_checkpoints")
        .map(|v| v.to_vec())
        .unwrap_or_default();
    if base_heads.is_empty() && base_checkpoints.is_empty() {
        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
    }
    for base in &base_heads {
        let Some(base) = base.as_map() else {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        };
        if !base_head_is_valid(base) {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
    }
    for checkpoint in &base_checkpoints {
        let Some(checkpoint) = checkpoint.as_map() else {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        };
        if !base_checkpoint_is_valid(checkpoint) {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
    }
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
    let snapshot_hash = semantic_hash_hex(snap).unwrap_or_default();
    let Some(base) = first_base_head(&base_heads, &base_checkpoints) else {
        return defer(
            ReasonCode::MissingDependency,
            mapping([
                ("acceptance_class", ystr("deferred_missing_checkpoint")),
                ("snapshot_descriptor_hash", ystr(snapshot_hash)),
            ]),
        );
    };
    let local_head = local_state
        .get("stream_heads")
        .and_then(|v| map_value(v, &string_value(base, "stream_id", "")));
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

fn snapshot_refs_are_valid(snap: &BTreeMap<String, Value>) -> bool {
    object_ref_is_valid(snap.get("payload_object"))
        && object_ref_is_valid(snap.get("snapshot_metadata"))
        && supersedes_are_valid(snap.get("supersedes"))
}

fn supersedes_are_valid(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => true,
        Some(Value::Seq(items)) => items.iter().all(|item| match item {
            Value::String(snapshot_id) => !snapshot_id.is_empty(),
            Value::Map(map) => map
                .get("snapshot_id")
                .and_then(Value::as_str)
                .is_some_and(|snapshot_id| !snapshot_id.is_empty()),
            _ => false,
        }),
        Some(_) => false,
    }
}

fn base_head_is_valid(base: &BTreeMap<String, Value>) -> bool {
    !string_value(base, "stream_id", "").is_empty()
        && number_value(base, "seq", -1) >= 0
        && !event_hash_alias_value(base).is_empty()
}

fn base_checkpoint_is_valid(checkpoint: &BTreeMap<String, Value>) -> bool {
    let checkpoint_id_present = checkpoint
        .get("checkpoint_id")
        .and_then(Value::as_str)
        .is_some_and(|id| !id.is_empty());
    let heads = get_seq(checkpoint, "heads").unwrap_or(&[]);
    if !checkpoint_id_present && heads.is_empty() {
        return false;
    }
    heads
        .iter()
        .filter_map(Value::as_map)
        .all(base_head_is_valid)
        && heads.iter().all(|head| head.as_map().is_some())
}

fn first_base_head<'a>(
    base_heads: &'a [Value],
    base_checkpoints: &'a [Value],
) -> Option<&'a BTreeMap<String, Value>> {
    base_heads.first().and_then(Value::as_map).or_else(|| {
        base_checkpoints
            .iter()
            .filter_map(Value::as_map)
            .find_map(|checkpoint| {
                get_seq(checkpoint, "heads")
                    .and_then(|heads| heads.first())
                    .and_then(Value::as_map)
            })
    })
}
