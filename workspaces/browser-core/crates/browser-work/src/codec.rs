use alloc::vec::Vec;

use crate::protocol::{ChannelEndpoint, Hash, NodeIdentity, WorkAdmission, WorkSignature};

pub fn blake3_hash(bytes: &[u8]) -> Hash {
    *crate::blake3::hash(bytes).as_bytes()
}

pub fn work_admission_packet_bytes(admission: &WorkAdmission) -> Vec<u8> {
    let mut out = Vec::new();
    push_u16(&mut out, 5);
    push_u16(&mut out, admission.abi_version);
    out.extend_from_slice(&admission.admission_id);
    out.extend_from_slice(&admission.dao_id);
    out.extend_from_slice(&admission.user);
    push_node_identity(&mut out, &admission.admission_node);
    out.extend_from_slice(&admission.request_hash);
    out.extend_from_slice(&admission.assigned_route_commitment);
    push_channel_endpoint(&mut out, &admission.assigned_channel);
    push_u64(&mut out, admission.assigned_relay_path.len() as u64);
    for relay in &admission.assigned_relay_path {
        out.extend_from_slice(relay);
    }
    push_u64(&mut out, admission.admitted_budget);
    out.extend_from_slice(&admission.policy_hash);
    push_u64(&mut out, admission.sequence);
    push_u64(&mut out, admission.valid_until_unix_ms);
    push_signature(&mut out, &admission.signature);
    out
}

fn push_node_identity(out: &mut Vec<u8>, identity: &NodeIdentity) {
    out.extend_from_slice(&identity.node_id);
    push_u16(out, identity.role);
    out.extend_from_slice(&identity.public_key);
}

fn push_channel_endpoint(out: &mut Vec<u8>, endpoint: &ChannelEndpoint) {
    push_u16(out, endpoint.abi_version);
    out.extend_from_slice(&endpoint.channel_id);
    push_u16(out, endpoint.kind);
    push_bytes(out, &endpoint.address);
    push_bytes(out, endpoint.label.as_bytes());
}

fn push_signature(out: &mut Vec<u8>, signature: &WorkSignature) {
    push_u16(out, signature.algorithm);
    push_bytes(out, &signature.public_key);
    push_bytes(out, &signature.signature);
}

fn push_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    push_u64(out, bytes.len() as u64);
    out.extend_from_slice(bytes);
}

fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}
