use alloc::vec::Vec;

use edgerun_crypto::Ed25519SigningKey;

use crate::channel::*;
use crate::codec::{blake3_hash, sign_ed25519, verify_signature};
use crate::protocol::{NodeIdentity, WORK_WIRE_ABI_VERSION};

const ROUTE_ADVERTISEMENT_DOMAIN: &[u8] = b"edgerun:v1:work:route-advertisement";
const ROUTE_SNAPSHOT_DOMAIN: &[u8] = b"edgerun:v1:work:route-snapshot";

pub fn route_advertisement_preimage(value: &RouteAdvertisement) -> Vec<u8> {
    let mut out = domain(ROUTE_ADVERTISEMENT_DOMAIN);
    encode_node(&mut out, &value.node);
    out.extend_from_slice(&value.relay_node_id);
    encode_endpoint(&mut out, &value.endpoint);
    encode_u16_list(&mut out, &value.roles);
    encode_u16_list(&mut out, &value.departments);
    out.extend_from_slice(&value.status.to_be_bytes());
    out.extend_from_slice(&value.sequence.to_be_bytes());
    out.extend_from_slice(&value.valid_until_unix_ms.to_be_bytes());
    out.extend_from_slice(&value.previous_route_hash);
    out
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
        && verify_signature(&value.node, &value.signature, &route_advertisement_preimage(value))
}

pub fn verify_available_route_advertisement(value: &RouteAdvertisement) -> bool {
    verify_route_advertisement(value) && value.status == ROUTE_STATUS_AVAILABLE
}

pub fn is_valid_route_status(status: u16) -> bool {
    matches!(status, ROUTE_STATUS_AVAILABLE | ROUTE_STATUS_DRAINING | ROUTE_STATUS_UNAVAILABLE)
}

pub fn route_snapshot_preimage(value: &RouteSnapshot) -> Vec<u8> {
    let mut out = domain(ROUTE_SNAPSHOT_DOMAIN);
    encode_node(&mut out, &value.issued_by);
    out.extend_from_slice(&value.sequence.to_be_bytes());
    out.extend_from_slice(&value.route_root);
    for route in &value.routes {
        out.extend_from_slice(&blake3_hash(&route_advertisement_preimage(route)));
    }
    out
}

pub fn sign_route_snapshot(key: &Ed25519SigningKey, mut value: RouteSnapshot) -> RouteSnapshot {
    value.signature = sign_ed25519(key, &route_snapshot_preimage(&value));
    value
}

pub fn verify_route_snapshot(value: &RouteSnapshot) -> bool {
    value.abi_version == WORK_WIRE_ABI_VERSION
        && verify_signature(&value.issued_by, &value.signature, &route_snapshot_preimage(value))
}

fn domain(domain: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(domain.len() + 1);
    out.extend_from_slice(domain);
    out.push(0);
    out
}

fn encode_node(out: &mut Vec<u8>, node: &NodeIdentity) {
    out.extend_from_slice(&node.node_id);
    out.extend_from_slice(&node.role.to_be_bytes());
    out.extend_from_slice(&node.public_key);
}

fn encode_endpoint(out: &mut Vec<u8>, endpoint: &ChannelEndpoint) {
    out.extend_from_slice(&endpoint.channel_id);
    out.extend_from_slice(&endpoint.kind.to_be_bytes());
    out.extend_from_slice(&(endpoint.address.len() as u64).to_be_bytes());
    out.extend_from_slice(&endpoint.address);
    out.extend_from_slice(&(endpoint.label.as_bytes().len() as u64).to_be_bytes());
    out.extend_from_slice(endpoint.label.as_bytes());
}

fn encode_u16_list(out: &mut Vec<u8>, values: &[u16]) {
    out.extend_from_slice(&(values.len() as u64).to_be_bytes());
    for value in values {
        out.extend_from_slice(&value.to_be_bytes());
    }
}
