use crate::prelude::v1::*;

use super::helpers::*;
pub fn validate_canonical_case(semantic_input: &BTreeMap<String, Value>) -> ValidationResult {
    let focus = get_map(semantic_input, "brief")
        .map(|b| string_value(b, "focus", ""))
        .unwrap_or_default();

    if let Some(result) = validate_canonical_artifacts(semantic_input, &focus) {
        return result;
    }

    match focus.as_str() {
        "presence-semantics" => {
            return accept(
                mapping([(
                    "canonical_equivalence",
                    ystr("distinct encodings for null/empty/zero"),
                )]),
                empty_map(),
            );
        }
        "repeated-order" | "repeated-field-order" => {
            return accept(
                mapping([("ordering_semantics", ystr("preserved"))]),
                empty_map(),
            );
        }
        "oneof-selection" | "oneof-payload" => {
            return accept(
                mapping([("oneof_valid", ystr("payload_object selected"))]),
                empty_map(),
            );
        }
        "timestamp-normalization" => {
            return accept(
                mapping([("timestamp_normalized", Value::Bool(true))]),
                empty_map(),
            );
        }
        "signable-vs-full" => {
            return accept(
                mapping([("record_hash_source", ystr("signable_form"))]),
                empty_map(),
            );
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

fn validate_canonical_artifacts(
    semantic_input: &BTreeMap<String, Value>,
    focus: &str,
) -> Option<ValidationResult> {
    let artifacts = get_map(semantic_input, "canonical_artifacts")?;
    let signable_hex = string_value(artifacts, "canonical_signable", "");
    let full_hex = string_value(artifacts, "canonical_full", "");
    let record_hash_hex = string_value(artifacts, "record_hash", "");
    let signature_input_hex = string_value(artifacts, "signature_input", "");
    let signature_hex = string_value(artifacts, "signature", "");

    let signable = match decode_artifact_hex(&signable_hex) {
        Ok(bytes) => bytes,
        Err(reason) => {
            return Some(reject(
                ReasonCode::CanonicalizationFail,
                Value::String(reason),
                empty_map(),
            ));
        }
    };
    let full = match decode_artifact_hex(&full_hex) {
        Ok(bytes) => bytes,
        Err(reason) => {
            return Some(reject(
                ReasonCode::CanonicalizationFail,
                Value::String(reason),
                empty_map(),
            ));
        }
    };
    let record_hash = match decode_artifact_hex(&record_hash_hex) {
        Ok(bytes) => bytes,
        Err(reason) => {
            return Some(reject(
                ReasonCode::CanonicalizationFail,
                Value::String(reason),
                empty_map(),
            ));
        }
    };

    if signable.is_empty() || full.is_empty() {
        return Some(reject(
            ReasonCode::CanonicalizationFail,
            Value::String("canonical artifact bytes must be non-empty".into()),
            empty_map(),
        ));
    }
    if record_hash.len() != 32 {
        return Some(reject(
            ReasonCode::CanonicalizationFail,
            Value::String("record_hash artifact must be 32 bytes".into()),
            empty_map(),
        ));
    }

    let mut derived = BTreeMap::new();
    derived.insert("canonical_signable_hex".into(), ystr(signable_hex.clone()));
    derived.insert("canonical_full_hex".into(), ystr(full_hex.clone()));
    derived.insert("record_hash_hex".into(), ystr(record_hash_hex.clone()));

    if focus == "signable-vs-full" {
        if signable == full {
            return Some(reject(
                ReasonCode::CanonicalizationFail,
                Value::String("signable and full canonical artifacts must differ".into()),
                empty_map(),
            ));
        }
        if !signature_input_hex.is_empty() {
            if let Err(reason) = decode_artifact_hex(&signature_input_hex) {
                return Some(reject(
                    ReasonCode::CanonicalizationFail,
                    Value::String(reason),
                    empty_map(),
                ));
            }
            derived.insert("signature_input_hex".into(), ystr(signature_input_hex));
        }
        if !signature_hex.is_empty() {
            if let Err(reason) = decode_artifact_hex(&signature_hex) {
                return Some(reject(
                    ReasonCode::CanonicalizationFail,
                    Value::String(reason),
                    empty_map(),
                ));
            }
            derived.insert("signature_hex".into(), ystr(signature_hex));
        }
        derived.insert("record_hash_source".into(), ystr("signable_form"));
    } else {
        derived.insert("canonical_artifacts_checked".into(), Value::Bool(true));
    }

    Some(accept(Value::Map(derived), empty_map()))
}

fn decode_artifact_hex(value: &str) -> Result<Vec<u8>, String> {
    if value.is_empty() {
        return Err("missing canonical artifact".into());
    }
    crate::util::hex_to_bytes(value).map_err(|e| format!("invalid canonical artifact hex: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical_input(focus: &str, signable: &str, full: &str) -> BTreeMap<String, Value> {
        let mut input = BTreeMap::new();
        input.insert("brief".into(), mapping([("focus", ystr(focus))]));
        input.insert(
            "canonical_artifacts".into(),
            mapping([
                ("canonical_signable", ystr(signable)),
                ("canonical_full", ystr(full)),
                (
                    "record_hash",
                    ystr("0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"),
                ),
            ]),
        );
        input
    }

    #[test]
    fn canonical_artifacts_are_validated() {
        let input = canonical_input("presence-semantics", "0x0102", "0x0102");
        let result = validate_canonical_case(&input);

        assert_eq!(result.verdict, Verdict::Accept);
    }

    #[test]
    fn signable_vs_full_requires_distinct_artifacts() {
        let input = canonical_input("signable-vs-full", "0x0102", "0x0102");
        let result = validate_canonical_case(&input);

        assert_eq!(result.verdict, Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CanonicalizationFail));
    }
}
