use crate::prelude::v1::*;

use super::helpers::*;
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
    let chain = if let Some(v) = semantic_input.get("delegation_chain") {
        eprintln!("DEL: v type={:?}", v);
        match v {
            Value::Seq(seq) => seq.clone(),
            Value::Map(m) => {
                // Single item as Map - check key "" for the item
                if let Some(item) = m.get("") {
                    vec![item.clone()]
                } else {
                    vec![]
                }
            }
            Value::Null => vec![],
            _ => vec![],
        }
    } else {
        vec![]
    };
    if chain.is_empty() {
        return reject(ReasonCode::AuthorityDenied, empty_map(), empty_map());
    }
    let Some(first) = chain[0].as_map() else {
        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
    };
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
        let link_id = string_value(link, "delegation_id", "");
        if !link_id.is_empty() {
            let is_revoked = local_state
                .get("revocations")
                .and_then(Value::as_seq)
                .map(|arr| {
                    arr.iter().any(|v| {
                        v.as_map()
                            .and_then(|m| m.get("target"))
                            .and_then(Value::as_str)
                            .map(|t| t == link_id)
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false);
            if is_revoked {
                return reject(ReasonCode::RevocationActive, empty_map(), empty_map());
            }
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
        let Some(parent) = chain[i].as_map() else {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        };
        let Some(child) = chain[i + 1].as_map() else {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        };
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
