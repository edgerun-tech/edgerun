use crate::prelude::v1::*;

use super::helpers::*;
pub fn validate_reachability_hint_map(
    hint: &BTreeMap<String, Value>,
    now: Option<crate::util::DateTimeUtc>,
    verifier: &dyn FixtureVerifier,
    semantic_hash_hex: &dyn Fn(&BTreeMap<String, Value>) -> Option<String>,
) -> Result<(), ReasonCode> {
    if number_value(hint, "hint_version", 0) != 1 {
        return Err(ReasonCode::VersionUnsupported);
    }
    if string_value(hint, "subject_node", "").is_empty() {
        return Err(ReasonCode::StructuralInvalid);
    }
    if matches!(
        hint.get("transport_class").and_then(Value::as_str),
        None | Some("") | Some("TRANSPORT_CLASS_UNSPECIFIED")
    ) {
        return Err(ReasonCode::StructuralInvalid);
    }
    if matches!(
        hint.get("directness").and_then(Value::as_str),
        None | Some("") | Some("DIRECTNESS_UNSPECIFIED")
    ) {
        return Err(ReasonCode::StructuralInvalid);
    }
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
            if parse_ts(until_s)? < parse_ts(s)? {
                return Err(ReasonCode::TimeInvalid);
            }
        }
    }
    if let (Some(now), Some(until_s)) = (now, hint.get("valid_until").and_then(Value::as_str)) {
        if parse_ts(until_s)? < now {
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
