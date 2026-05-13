use alloc::vec::Vec;

use edgerun_crypto::Ed25519SigningKey;

use crate::channel::*;
use crate::codec::blake3_hash;
use crate::preimage::PreimageBuilder;
use crate::protocol::{Hash, WORK_WIRE_ABI_VERSION};
use crate::signing::{sign_ed25519, verify_signature};

const ROUTE_ADVERTISEMENT_DOMAIN: &[u8] = b"edgerun:v1:work:route-advertisement";
const ROUTE_SNAPSHOT_DOMAIN: &[u8] = b"edgerun:v1:work:route-snapshot";

#[cfg(feature = "std")]
pub(crate) fn current_unix_ms() -> u64 {
    crate::std_runtime::unix_ms()
}

#[cfg(not(feature = "std"))]
pub(crate) fn current_unix_ms() -> u64 {
    0
}

pub fn route_advertisement_preimage(value: &RouteAdvertisement) -> Vec<u8> {
    PreimageBuilder::domain(ROUTE_ADVERTISEMENT_DOMAIN)
        .node(&value.node)
        .node_id(&value.relay_node_id)
        .channel(&value.endpoint)
        .u16_list(&value.roles)
        .u16_list(&value.departments)
        .u16(value.status)
        .u64(value.sequence)
        .u64(value.valid_until_unix_ms)
        .hash(&value.previous_route_hash)
        .finish()
}

pub fn sign_route_advertisement(
    key: &Ed25519SigningKey,
    mut value: RouteAdvertisement,
) -> RouteAdvertisement {
    value.signature = sign_ed25519(key, &route_advertisement_preimage(&value));
    value
}

pub fn verify_route_advertisement(value: &RouteAdvertisement) -> bool {
    value.abi_version == WORK_WIRE_ABI_VERSION
        && is_valid_route_status(value.status)
        && verify_signature(
            &value.node,
            &value.signature,
            &route_advertisement_preimage(value),
        )
}

pub fn verify_available_route_advertisement(value: &RouteAdvertisement) -> bool {
    verify_route_advertisement(value) && value.status == ROUTE_STATUS_AVAILABLE
}

pub fn verify_live_route_advertisement(value: &RouteAdvertisement, now_unix_ms: u64) -> bool {
    verify_route_advertisement(value) && route_is_available(value, now_unix_ms)
}

pub fn route_is_available(route: &RouteAdvertisement, now_unix_ms: u64) -> bool {
    route.status == ROUTE_STATUS_AVAILABLE && route.valid_until_unix_ms >= now_unix_ms
}

pub fn route_hash(route: &RouteAdvertisement) -> Hash {
    blake3_hash(&route_advertisement_preimage(route))
}

pub fn available_route_hash(route: &RouteAdvertisement, now_unix_ms: u64) -> Option<Hash> {
    route_is_available(route, now_unix_ms).then(|| route_hash(route))
}

pub fn is_valid_route_status(status: u16) -> bool {
    matches!(
        status,
        ROUTE_STATUS_AVAILABLE | ROUTE_STATUS_DRAINING | ROUTE_STATUS_UNAVAILABLE
    )
}

pub fn route_snapshot_preimage(value: &RouteSnapshot) -> Vec<u8> {
    let mut route_hashes = Vec::with_capacity(value.routes.len());
    for route in &value.routes {
        route_hashes.push(blake3_hash(&route_advertisement_preimage(route)));
    }
    PreimageBuilder::domain(ROUTE_SNAPSHOT_DOMAIN)
        .node(&value.issued_by)
        .u64(value.sequence)
        .hash(&value.route_root)
        .hash_list(&route_hashes)
        .finish()
}

pub fn sign_route_snapshot(key: &Ed25519SigningKey, mut value: RouteSnapshot) -> RouteSnapshot {
    value.signature = sign_ed25519(key, &route_snapshot_preimage(&value));
    value
}

pub fn verify_route_snapshot(value: &RouteSnapshot) -> bool {
    value.abi_version == WORK_WIRE_ABI_VERSION
        && verify_signature(
            &value.issued_by,
            &value.signature,
            &route_snapshot_preimage(value),
        )
}
