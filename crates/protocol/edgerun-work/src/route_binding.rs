use alloc::vec::Vec;

use crate::channel::RouteBinding;
use crate::codec::blake3_hash;
use crate::preimage::{HashBuilder, PreimageBuilder};
use crate::protocol::{Hash, WORK_WIRE_ABI_VERSION};

const ROUTE_BINDING_DOMAIN: &[u8] = b"edgerun:v1:work:route-binding";
const ROUTE_COMMITMENT_DOMAIN: &[u8] = b"edgerun:v1:work:route-commitment";
const ROUTE_ROOT_DOMAIN: &[u8] = b"edgerun:v1:work:route-root";

#[cfg(feature = "std")]
pub(crate) fn current_unix_ms() -> u64 {
    crate::std_runtime::unix_ms()
}

#[cfg(not(feature = "std"))]
pub(crate) fn current_unix_ms() -> u64 {
    0
}

pub fn route_binding_preimage(value: &RouteBinding) -> Vec<u8> {
    PreimageBuilder::domain(ROUTE_BINDING_DOMAIN)
        .node(&value.node)
        .node_id(&value.relay_node_id)
        .channel(&value.endpoint)
        .u16_list(&value.roles)
        .u16_list(&value.departments)
        .u64(value.valid_until_unix_ms)
        .finish()
}

pub fn verify_route_binding(value: &RouteBinding) -> bool {
    value.abi_version == WORK_WIRE_ABI_VERSION
        && value.endpoint.abi_version == WORK_WIRE_ABI_VERSION
        && value.roles.contains(&value.node.role)
}

pub fn verify_live_route_binding(value: &RouteBinding, now_unix_ms: u64) -> bool {
    verify_route_binding(value) && route_is_available(value, now_unix_ms)
}

pub fn route_is_available(route: &RouteBinding, now_unix_ms: u64) -> bool {
    route.valid_until_unix_ms >= now_unix_ms
}

pub fn route_hash(route: &RouteBinding) -> Hash {
    blake3_hash(&route_binding_preimage(route))
}

pub fn available_route_hash(route: &RouteBinding, now_unix_ms: u64) -> Option<Hash> {
    route_is_available(route, now_unix_ms).then(|| route_hash(route))
}

pub fn route_commitment(route: &RouteBinding) -> Hash {
    HashBuilder::domain(ROUTE_COMMITMENT_DOMAIN)
        .bytes(&route_binding_preimage(route))
        .finish()
}

pub fn route_root_hash(routes: &[RouteBinding]) -> Hash {
    let mut builder = HashBuilder::domain(ROUTE_ROOT_DOMAIN).u64(routes.len() as u64);
    for route in routes {
        builder = builder.hash(&route_commitment(route));
    }
    builder.finish()
}
