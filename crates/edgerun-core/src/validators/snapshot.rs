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
    let Some(base) = base_heads[0].as_map() else {
        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
    };
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
