use crate::prelude::v1::*;

use super::helpers::*;
use super::reachability::validate_reachability_hint_map;
pub fn validate_network_case(
    semantic_input: &BTreeMap<String, Value>,
    local_state: &BTreeMap<String, Value>,
    verifier: &dyn FixtureVerifier,
    semantic_hash_hex: &dyn Fn(&BTreeMap<String, Value>) -> Option<String>,
) -> ValidationResult {
    let now = local_state
        .get("now")
        .and_then(Value::as_str)
        .and_then(|s| parse_ts(s).ok());
    if let Some(assignments) = get_map(semantic_input, "route_trust_assignments") {
        let issuer = string_value(assignments, "issuer", "");
        if issuer.is_empty() {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        let trusted = set_from_list(local_state.get("trust_roots")).contains(&issuer)
            || set_from_list(local_state.get("current_controller_set")).contains(&issuer);
        if !trusted {
            return reject(ReasonCode::AuthorityDenied, empty_map(), empty_map());
        }
        if get_seq(assignments, "assignments")
            .map(|v| v.is_empty())
            .unwrap_or(true)
        {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        return accept(
            mapping([
                ("decision", ystr("route_trust_assignments_accepted")),
                ("advisory_only", Value::Bool(true)),
            ]),
            empty_map(),
        );
    }
    if let Some(policy) = get_map(semantic_input, "aggregate_trust_policy") {
        let issuer = string_value(policy, "issuer", "");
        if issuer.is_empty() {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        let trusted = set_from_list(local_state.get("trust_roots")).contains(&issuer)
            || set_from_list(local_state.get("current_controller_set")).contains(&issuer);
        if !trusted {
            return reject(ReasonCode::AuthorityDenied, empty_map(), empty_map());
        }
        if policy
            .get("minimum_trust_score")
            .and_then(Value::as_i64)
            .is_some_and(|v| v < 0)
        {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        return accept(
            mapping([
                ("decision", ystr("aggregate_trust_policy_accepted")),
                ("advisory_only", Value::Bool(true)),
            ]),
            empty_map(),
        );
    }
    if let Some(policy) = get_map(semantic_input, "route_selection_policy") {
        let candidates = get_seq(local_state, "candidate_routes").unwrap_or(&[]);
        let assignment_scores = local_state
            .get("route_assignment_scores")
            .and_then(Value::as_map);
        let min_quality = policy
            .get("minimum_quality_hint")
            .and_then(Value::as_i64)
            .unwrap_or(i64::MIN);
        let max_cost = policy
            .get("maximum_cost_hint")
            .and_then(Value::as_i64)
            .unwrap_or(i64::MAX);
        let require_active = policy
            .get("require_active_session")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let preferred_advertisers: BTreeSet<String> = get_seq(policy, "preferred_advertisers")
            .unwrap_or(&[])
            .iter()
            .filter_map(Value::as_map)
            .map(|m| string_value(m, "identity_id", ""))
            .filter(|s| !s.is_empty())
            .collect();
        let preferred_next_hops: BTreeSet<String> = get_seq(policy, "preferred_next_hops")
            .unwrap_or(&[])
            .iter()
            .filter_map(Value::as_map)
            .map(|m| string_value(m, "node_id", ""))
            .filter(|s| !s.is_empty())
            .collect();
        let minimum_trust_score = get_map(local_state, "aggregate_trust_policy")
            .and_then(|m| m.get("minimum_trust_score"))
            .and_then(Value::as_i64)
            .unwrap_or(i64::MIN);
        let allowed_responders: Option<BTreeSet<String>> =
            get_map(local_state, "aggregate_trust_policy")
                .and_then(|m| get_seq(m, "allowed_responders"))
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_map)
                        .map(|m| string_value(m, "identity_id", ""))
                        .filter(|s| !s.is_empty())
                        .collect()
                });
        let preferred_aggregators: BTreeSet<String> =
            get_map(local_state, "aggregate_trust_policy")
                .and_then(|m| get_seq(m, "preferred_aggregators"))
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_map)
                        .map(|m| string_value(m, "identity_id", ""))
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .unwrap_or_default();

        let mut best: Option<(i64, bool, bool, String, String, String)> = None;
        for candidate in candidates.iter().filter_map(Value::as_map) {
            let quality = number_value(candidate, "quality_hint", 0);
            if quality < min_quality {
                continue;
            }
            let cost = number_value(candidate, "cost_hint", 0);
            if cost > max_cost {
                continue;
            }
            if require_active
                && !candidate
                    .get("has_active_session")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            {
                continue;
            }
            let advertiser = string_value(candidate, "advertiser", "");
            let next_hop = string_value(candidate, "next_hop_node", "");
            if allowed_responders
                .as_ref()
                .is_some_and(|set| !set.contains(&advertiser))
            {
                continue;
            }
            let assignment_score = assignment_scores
                .and_then(|m| m.get(&advertiser))
                .and_then(Value::as_i64)
                .unwrap_or(0);
            let preferred_bonus = if preferred_aggregators.contains(&advertiser) {
                1
            } else {
                0
            };
            let score = assignment_score + quality - cost + preferred_bonus;
            if score < minimum_trust_score {
                continue;
            }
            let preferred_advertiser = preferred_advertisers.contains(&advertiser);
            let preferred_next_hop = preferred_next_hops.contains(&next_hop);
            let advertised_at = string_value(candidate, "advertised_at", "");
            let candidate_tuple = (
                score,
                preferred_advertiser,
                preferred_next_hop,
                advertiser,
                next_hop,
                advertised_at,
            );
            let replace = match &best {
                None => true,
                Some(current) => {
                    candidate_tuple.0 > current.0
                        || (candidate_tuple.0 == current.0 && candidate_tuple.1 && !current.1)
                        || (candidate_tuple.0 == current.0
                            && candidate_tuple.1 == current.1
                            && candidate_tuple.2
                            && !current.2)
                        || (candidate_tuple.0 == current.0
                            && candidate_tuple.1 == current.1
                            && candidate_tuple.2 == current.2
                            && !candidate_tuple.5.is_empty()
                            && (current.5.is_empty() || candidate_tuple.5 < current.5))
                        || (candidate_tuple.0 == current.0
                            && candidate_tuple.1 == current.1
                            && candidate_tuple.2 == current.2
                            && candidate_tuple.5 == current.5
                            && candidate_tuple.4 < current.4)
                }
            };
            if replace {
                best = Some(candidate_tuple);
            }
        }
        let Some((score, _pa, _pn, advertiser, next_hop, _ts)) = best else {
            return reject(ReasonCode::PolicyDenied, empty_map(), empty_map());
        };
        return accept(
            mapping([
                ("decision", ystr("route_selected")),
                ("selected_advertiser", ystr(advertiser)),
                ("selected_next_hop", ystr(next_hop)),
                ("route_score", Value::Int(score)),
                ("advisory_only", Value::Bool(true)),
            ]),
            empty_map(),
        );
    }
    if let Some(hello) = get_map(semantic_input, "session_hello") {
        let target = string_value(hello, "target_node", "");
        if !target.is_empty() && target != string_value(local_state, "local_node", "") {
            return reject(ReasonCode::TargetMismatch, empty_map(), empty_map());
        }
        let nonce = hello
            .get("session_nonce")
            .or_else(|| hello.get("session_nonce_hex"))
            .and_then(nonce_bytes)
            .unwrap_or_default();
        if nonce.is_empty() {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        let supported: BTreeSet<i64> = get_seq(hello, "supported_protocol_versions")
            .map(|arr| arr.iter().filter_map(Value::as_i64).collect())
            .unwrap_or_default();
        let local_supported: BTreeSet<i64> = local_state
            .get("supported_protocol_versions")
            .and_then(Value::as_seq)
            .map(|arr| arr.iter().filter_map(Value::as_i64).collect())
            .unwrap_or_else(|| BTreeSet::from([1]));
        let overlap = supported.intersection(&local_supported).copied().max();
        if overlap.is_none() {
            return reject(ReasonCode::VersionUnsupported, empty_map(), empty_map());
        }
        let expected = hello
            .get("signature_fixture")
            .and_then(Value::as_str)
            .unwrap_or(&string_value(hello, "initiator", ""))
            .to_string();
        if matches!(
            verifier.verify_signed_fixture(hello, &expected),
            Some(false)
        ) {
            return reject(ReasonCode::CryptoInvalid, empty_map(), empty_map());
        }
        for hint in get_seq(hello, "initiator_locators").unwrap_or(&[]) {
            let Some(map) = hint.as_map() else {
                return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
            };
            if let Err(code) = validate_reachability_hint_map(map, now, verifier, semantic_hash_hex)
            {
                return reject(code, empty_map(), empty_map());
            }
        }
        return accept(
            mapping([
                ("decision", ystr("session_hello_accepted")),
                (
                    "selected_protocol_version",
                    Value::Int(overlap.unwrap_or_default()),
                ),
            ]),
            empty_map(),
        );
    }
    if let Some(accept_msg) = get_map(semantic_input, "session_accept") {
        let expected_nonce = local_state
            .get("expected_session_nonce")
            .and_then(nonce_bytes);
        let echoed = accept_msg
            .get("echoed_session_nonce")
            .and_then(nonce_bytes)
            .unwrap_or_default();
        if echoed.is_empty() {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        if expected_nonce.as_ref().is_some_and(|v| *v != echoed) {
            return reject(ReasonCode::TargetMismatch, empty_map(), empty_map());
        }
        let selected = number_value(accept_msg, "selected_protocol_version", 0);
        let local_supported: BTreeSet<i64> = local_state
            .get("supported_protocol_versions")
            .and_then(Value::as_seq)
            .map(|arr| arr.iter().filter_map(Value::as_i64).collect())
            .unwrap_or_else(|| BTreeSet::from([1]));
        if !local_supported.contains(&selected) {
            return reject(ReasonCode::VersionUnsupported, empty_map(), empty_map());
        }
        let expected = accept_msg
            .get("signature_fixture")
            .and_then(Value::as_str)
            .unwrap_or(&string_value(accept_msg, "responder", ""))
            .to_string();
        if matches!(
            verifier.verify_signed_fixture(accept_msg, &expected),
            Some(false)
        ) {
            return reject(ReasonCode::CryptoInvalid, empty_map(), empty_map());
        }
        for hint in get_seq(accept_msg, "responder_locators").unwrap_or(&[]) {
            let Some(map) = hint.as_map() else {
                return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
            };
            if let Err(code) = validate_reachability_hint_map(map, now, verifier, semantic_hash_hex)
            {
                return reject(code, empty_map(), empty_map());
            }
        }
        return accept(
            mapping([
                ("decision", ystr("session_accepted")),
                ("selected_protocol_version", Value::Int(selected)),
            ]),
            empty_map(),
        );
    }
    if let Some(route) = get_map(semantic_input, "route_advertisement") {
        if let Some(at) = route.get("advertised_at").and_then(Value::as_str) {
            if let Some(exp) = route.get("expires_at").and_then(Value::as_str) {
                if parse_ts(exp)
                    .ok()
                    .zip(parse_ts(at).ok())
                    .is_some_and(|(e, a)| e < a)
                {
                    return reject(ReasonCode::TimeInvalid, empty_map(), empty_map());
                }
            }
        }
        if let (Some(now), Some(exp)) = (now, route.get("expires_at").and_then(Value::as_str)) {
            if parse_ts(exp).map_or(true, |t| t < now) {
                return reject(ReasonCode::TimeInvalid, empty_map(), empty_map());
            }
        }
        let target = string_value(route, "target_node", "");
        for hint in get_seq(route, "reachability").unwrap_or(&[]) {
            let Some(map) = hint.as_map() else {
                return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
            };
            if string_value(map, "subject_node", "") != target {
                return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
            }
            if let Err(code) = validate_reachability_hint_map(map, now, verifier, semantic_hash_hex)
            {
                return reject(code, empty_map(), empty_map());
            }
        }
        let expected = route
            .get("signature_fixture")
            .and_then(Value::as_str)
            .unwrap_or(&string_value(route, "advertiser", ""))
            .to_string();
        if matches!(
            verifier.verify_signed_fixture(route, &expected),
            Some(false)
        ) {
            return reject(ReasonCode::CryptoInvalid, empty_map(), empty_map());
        }
        return accept(
            mapping([
                ("decision", ystr("route_advertisement_accepted")),
                ("advisory_only", Value::Bool(true)),
            ]),
            empty_map(),
        );
    }
    if let Some(relay) = get_map(semantic_input, "relay_envelope") {
        let target = string_value(relay, "intended_recipient_node", "");
        if !target.is_empty() && target != string_value(local_state, "local_node", "") {
            return reject(ReasonCode::TargetMismatch, empty_map(), empty_map());
        }
        let has_payload_object = relay.contains_key("payload_object");
        let has_inline_payload = relay.contains_key("inline_payload");
        if has_payload_object == has_inline_payload {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        if let (Some(now), Some(until)) = (now, relay.get("store_until").and_then(Value::as_str)) {
            if parse_ts(until).map_or(true, |t| t < now) {
                return reject(ReasonCode::TimeInvalid, empty_map(), empty_map());
            }
        }
        let expected = relay
            .get("signature_fixture")
            .and_then(Value::as_str)
            .unwrap_or(&string_value(relay, "original_sender", ""))
            .to_string();
        if matches!(
            verifier.verify_signed_fixture(relay, &expected),
            Some(false)
        ) {
            return reject(ReasonCode::CryptoInvalid, empty_map(), empty_map());
        }
        return accept(mapping([("decision", ystr("relay_accepted"))]), empty_map());
    }
    if let Some(hint) = get_map(semantic_input, "reachability_hint") {
        if let Err(code) = validate_reachability_hint_map(hint, now, verifier, semantic_hash_hex) {
            return reject(code, empty_map(), empty_map());
        }
        return accept(
            mapping([("decision", ystr("reachability_hint_accepted"))]),
            empty_map(),
        );
    }
    reject(ReasonCode::StructuralInvalid, empty_map(), empty_map())
}

// ===================================================================
// Proto-level RouteAdvertisement validator
// ===================================================================

use crate::crypto::{
    verify_canonical_record, ECDSA_P256_PUBLIC_KEY_LEN, ECDSA_P256_SIGNATURE_LEN,
    SIG_DOMAIN_ROUTE_ADVERTISEMENT,
};
use crate::protocol::{canonical_bytes, ProtocolRecord};

/// Validates the structural integrity and signature of a RouteAdvertisement
/// at the proto type level (spec §14.24).
pub fn validate_route_advertisement(
    adv: &edgerun_proto::edgerun::v0::network::RouteAdvertisement,
) -> ValidationResult {
    let Some(ref target) = adv.target_node else {
        return reject(
            ReasonCode::StructuralInvalid,
            mapping([("reason", ystr("missing_target_node"))]),
            empty_map(),
        );
    };
    if target.node_id.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            mapping([("reason", ystr("empty_target_node_id"))]),
            empty_map(),
        );
    }
    let Some(ref advertiser) = adv.advertiser else {
        return reject(
            ReasonCode::StructuralInvalid,
            mapping([("reason", ystr("missing_advertiser"))]),
            empty_map(),
        );
    };
    if advertiser.identity_id.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            mapping([("reason", ystr("missing_advertiser"))]),
            empty_map(),
        );
    }
    if adv.advertised_at.is_none() {
        return reject(
            ReasonCode::StructuralInvalid,
            mapping([("reason", ystr("missing_advertised_at"))]),
            empty_map(),
        );
    }
    if adv.reachability.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            mapping([("reason", ystr("no_reachability_hints"))]),
            empty_map(),
        );
    }

    if let Some(ref sig) = adv.signature {
        if sig.value.len() != ECDSA_P256_SIGNATURE_LEN {
            return reject(
                ReasonCode::StructuralInvalid,
                mapping([("reason", ystr("bad_signature_length"))]),
                empty_map(),
            );
        }
        let Some(ref key_hint) = advertiser.key_hint else {
            return reject(
                ReasonCode::StructuralInvalid,
                mapping([("reason", ystr("missing_advertiser_key_hint"))]),
                empty_map(),
            );
        };
        if key_hint.len() != ECDSA_P256_PUBLIC_KEY_LEN {
            return reject(
                ReasonCode::StructuralInvalid,
                mapping([("reason", ystr("bad_key_hint_length"))]),
                empty_map(),
            );
        }

        let mut vk_sec1 = [0u8; 65];
        vk_sec1[0] = 0x04;
        vk_sec1[1..].copy_from_slice(key_hint);
        let vk = match edgerun_crypto::p256::ecdsa::VerifyingKey::from_sec1_bytes(&vk_sec1) {
            Ok(v) => v,
            Err(_) => {
                return reject(
                    ReasonCode::CryptoInvalid,
                    mapping([("reason", ystr("invalid_public_key"))]),
                    empty_map(),
                );
            }
        };

        let canonical = canonical_bytes(&ProtocolRecord::RouteAdvertisement(adv.clone()), true);
        if !verify_canonical_record(&vk, SIG_DOMAIN_ROUTE_ADVERTISEMENT, &canonical, &sig.value) {
            return reject(
                ReasonCode::CryptoInvalid,
                mapping([("reason", ystr("signature_verification_failed"))]),
                empty_map(),
            );
        }
    }

    accept(
        mapping([
            ("validation_level", ystr("route_advertisement_valid")),
            (
                "target_node",
                ystr(crate::util::bytes_to_hex(
                    &adv.target_node
                        .as_ref()
                        .map(|t| t.node_id.clone())
                        .unwrap_or_default(),
                )),
            ),
        ]),
        empty_map(),
    )
}

#[cfg(test)]
mod proto_tests {
    use super::*;
    use edgerun_proto::edgerun::v0::common::{
        Directness, IdentityRef, NodeRef, Signature, TransportClass,
    };
    use edgerun_proto::edgerun::v0::network::{ReachabilityHint, RouteAdvertisement};
    use prost_types::Timestamp;

    fn make_test_keypair() -> (edgerun_crypto::p256::ecdsa::SigningKey, Vec<u8>) {
        let sk = edgerun_crypto::p256::ecdsa::SigningKey::random(&mut edgerun_crypto::OsRng);
        let vk = *sk.verifying_key();
        let sec1 = vk.to_encoded_point(false);
        let pk = sec1.as_bytes()[1..].to_vec();
        (sk, pk)
    }

    fn sign_ad(
        sk: &edgerun_crypto::p256::ecdsa::SigningKey,
        adv: &RouteAdvertisement,
    ) -> RouteAdvertisement {
        use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashSigner;
        let canonical = canonical_bytes(&ProtocolRecord::RouteAdvertisement(adv.clone()), true);
        let record_hash = crate::crypto::sha256(&canonical);
        let sig_input =
            crate::crypto::signature_input(SIG_DOMAIN_ROUTE_ADVERTISEMENT, &record_hash);
        // Sign sig_input directly (per spec §17.11)
        let sig: edgerun_crypto::p256::ecdsa::Signature = sk.sign_prehash(&sig_input).unwrap();
        let mut signed = adv.clone();
        signed.signature = Some(Signature {
            algorithm: 1,
            value: sig.to_bytes().to_vec(),
        });
        signed
    }

    fn make_valid_ad(pk: Vec<u8>) -> RouteAdvertisement {
        RouteAdvertisement {
            advertisement_version: 1,
            target_node: Some(NodeRef {
                node_id: vec![1, 2, 3],
            }),
            advertiser: Some(IdentityRef {
                identity_id: vec![4, 5, 6],
                identity_kind: Some(2),
                key_hint: Some(pk),
            }),
            next_hop_node: None,
            reachability: vec![ReachabilityHint {
                hint_version: 1,
                subject_node: Some(NodeRef {
                    node_id: vec![1, 2, 3],
                }),
                transport_class: TransportClass::Quic as i32,
                locator_payload: vec![0, 0, 0, 0],
                directness: Directness::Direct as i32,
                valid_after: Some(Timestamp {
                    seconds: 1000,
                    nanos: 0,
                }),
                valid_until: Some(Timestamp {
                    seconds: 2000,
                    nanos: 0,
                }),
                cost_hint: None,
                quality_hint: None,
                issuer: None,
                signature: None,
            }],
            metric_hint: None,
            advertised_at: Some(Timestamp {
                seconds: 1000,
                nanos: 0,
            }),
            expires_at: None,
            route_metadata: None,
            signature: None,
        }
    }

    #[test]
    fn test_valid_signed_route_ad() {
        let (sk, pk) = make_test_keypair();
        let ad = make_valid_ad(pk);
        let signed = sign_ad(&sk, &ad);
        let result = validate_route_advertisement(&signed);
        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn test_valid_unsigned_route_ad() {
        let (_sk, pk) = make_test_keypair();
        let ad = make_valid_ad(pk);
        let result = validate_route_advertisement(&ad);
        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn test_reject_missing_target_node() {
        let (_sk, pk) = make_test_keypair();
        let mut ad = make_valid_ad(pk);
        ad.target_node = None;
        let result = validate_route_advertisement(&ad);
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
    }

    #[test]
    fn test_reject_invalid_signature() {
        let (sk, pk) = make_test_keypair();
        let ad = make_valid_ad(pk);
        let mut signed = sign_ad(&sk, &ad);
        if let Some(ref mut sig) = signed.signature {
            sig.value[0] ^= 0xFF;
        }
        let result = validate_route_advertisement(&signed);
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
    }
}
