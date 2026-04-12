use super::helpers::*;
pub fn validate_crypto_case(
    semantic_input: &BTreeMap<String, Value>,
    verifier: &dyn FixtureVerifier,
    semantic_hash_hex: &dyn Fn(&BTreeMap<String, Value>) -> Option<String>,
) -> ValidationResult {
    let Some(payload) = get_map(semantic_input, "signed_record") else {
        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
    };
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
