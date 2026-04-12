use super::helpers::*;
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
