//! Network-advisory canonical wire helpers.
//!
//! Route advertisements are signed advisory artifacts: verification proves who
//! advertised a route, not that the route is authoritative or safe to select.
//! Field tags intentionally follow the v0 proto schema numbers, while bytes are
//! encoded with edgerun-wire's deterministic custom format.

use alloc::vec::Vec;

use crate::protocol::*;
use edgerun_wire::{bytes, canonical_bytes, field, i64v, struct_value, u64v, WireValue};

#[must_use]
pub fn route_advertisement_signable_bytes(value: &RouteAdvertisement) -> Vec<u8> {
    canonical_bytes(&route_advertisement_value(value, true))
}

#[must_use]
pub fn route_advertisement_full_bytes(value: &RouteAdvertisement) -> Vec<u8> {
    canonical_bytes(&route_advertisement_value(value, false))
}

#[must_use]
pub fn reachability_hint_signable_bytes(value: &ReachabilityHint) -> Vec<u8> {
    canonical_bytes(&reachability_hint_value(value, true))
}

#[must_use]
pub fn reachability_hint_full_bytes(value: &ReachabilityHint) -> Vec<u8> {
    canonical_bytes(&reachability_hint_value(value, false))
}

fn signature_value(value: &Signature) -> WireValue {
    struct_value(alloc::vec![
        field(1, u64v(value.algorithm as u64)),
        field(2, bytes(&value.value)),
    ])
}

fn timestamp_value(value: &Timestamp) -> WireValue {
    struct_value(alloc::vec![
        field(1, i64v(value.seconds)),
        field(2, i64v(value.nanos as i64)),
    ])
}

fn identity_ref_value(value: &IdentityRef) -> WireValue {
    let mut fields = alloc::vec![field(1, bytes(&value.identity_id))];
    if let Some(v) = value.identity_kind {
        fields.push(field(2, u64v(v as u64)));
    }
    if let Some(v) = &value.key_hint {
        fields.push(field(3, bytes(v)));
    }
    struct_value(fields)
}

fn node_ref_value(value: &NodeRef) -> WireValue {
    struct_value(alloc::vec![field(1, bytes(&value.node_id))])
}

fn object_ref_value(value: &ObjectRef) -> WireValue {
    let mut fields = alloc::vec![field(1, bytes(&value.object_id))];
    if let Some(v) = value.object_kind {
        fields.push(field(2, u64v(v as u64)));
    }
    struct_value(fields)
}

fn list(values: Vec<WireValue>) -> WireValue {
    WireValue::List(values)
}

fn reachability_hint_list(values: &[ReachabilityHint]) -> WireValue {
    list(values.iter().map(|v| reachability_hint_value(v, false)).collect())
}

fn reachability_hint_value(value: &ReachabilityHint, signable: bool) -> WireValue {
    let mut fields = alloc::vec![
        field(1, u64v(value.hint_version as u64)),
        field(3, u64v(value.transport_class as u64)),
        field(4, bytes(&value.locator_payload)),
        field(5, u64v(value.directness as u64)),
    ];
    if let Some(v) = &value.subject_node {
        fields.push(field(2, node_ref_value(v)));
    }
    if let Some(v) = &value.valid_after {
        fields.push(field(6, timestamp_value(v)));
    }
    if let Some(v) = &value.valid_until {
        fields.push(field(7, timestamp_value(v)));
    }
    if let Some(v) = value.cost_hint {
        fields.push(field(8, u64v(v)));
    }
    if let Some(v) = value.quality_hint {
        fields.push(field(9, u64v(v)));
    }
    if let Some(v) = &value.issuer {
        fields.push(field(10, identity_ref_value(v)));
    }
    if !signable {
        if let Some(v) = &value.signature {
            fields.push(field(11, signature_value(v)));
        }
    }
    struct_value(fields)
}

fn route_advertisement_value(value: &RouteAdvertisement, signable: bool) -> WireValue {
    let mut fields = alloc::vec![
        field(1, u64v(value.advertisement_version as u64)),
        field(5, reachability_hint_list(&value.reachability)),
    ];
    if let Some(v) = &value.target_node {
        fields.push(field(2, node_ref_value(v)));
    }
    if let Some(v) = &value.advertiser {
        fields.push(field(3, identity_ref_value(v)));
    }
    if let Some(v) = &value.next_hop_node {
        fields.push(field(4, node_ref_value(v)));
    }
    if let Some(v) = &value.metric_hint {
        fields.push(field(6, object_ref_value(v)));
    }
    if let Some(v) = &value.advertised_at {
        fields.push(field(7, timestamp_value(v)));
    }
    if let Some(v) = &value.expires_at {
        fields.push(field(8, timestamp_value(v)));
    }
    if let Some(v) = &value.route_metadata {
        fields.push(field(9, object_ref_value(v)));
    }
    if !signable {
        if let Some(v) = &value.signature {
            fields.push(field(10, signature_value(v)));
        }
    }
    struct_value(fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_advertisement_signable_bytes_drop_signature() {
        let mut route = RouteAdvertisement {
            advertisement_version: 1,
            target_node: Some(NodeRef {
                node_id: b"target".to_vec(),
            }),
            ..RouteAdvertisement::default()
        };
        let without = route_advertisement_signable_bytes(&route);
        route.signature = Some(Signature {
            algorithm: 1,
            value: alloc::vec![0xab; 64],
        });
        let signable = route_advertisement_signable_bytes(&route);
        let full = route_advertisement_full_bytes(&route);
        assert_eq!(without, signable);
        assert_ne!(signable, full);
    }

    #[test]
    fn reachability_signature_changes_full_not_signable() {
        let mut hint = ReachabilityHint {
            hint_version: 1,
            locator_payload: b"lan://example".to_vec(),
            ..ReachabilityHint::default()
        };
        let signable = reachability_hint_signable_bytes(&hint);
        hint.signature = Some(Signature {
            algorithm: 1,
            value: alloc::vec![0xcd; 64],
        });
        assert_eq!(signable, reachability_hint_signable_bytes(&hint));
        assert_ne!(signable, reachability_hint_full_bytes(&hint));
    }
}
