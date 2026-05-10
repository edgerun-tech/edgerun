use alloc::vec::Vec;

use edgerun_crypto::{Ed25519Signer, Ed25519SigningKey, sha256};
use edgerun_wire::{WireError, access, deserialize, to_bytes, util};

use crate::protocol::*;

const NODE_AVAILABLE_DOMAIN: &[u8] = b"edgerun:v1:work:node-available";
const NODE_HEARTBEAT_DOMAIN: &[u8] = b"edgerun:v1:work:node-heartbeat";
const NODE_ASSIGNMENT_DOMAIN: &[u8] = b"edgerun:v1:work:node-assignment";
const NETWORK_MESSAGE_DOMAIN: &[u8] = b"edgerun:v1:work:network-message";
const WORK_ADMISSION_DOMAIN: &[u8] = b"edgerun:v1:work:admission";
const WORK_RECEIPT_DOMAIN: &[u8] = b"edgerun:v1:work:receipt";

pub fn blake3_hash(bytes: &[u8]) -> Hash {
    *blake3::hash(bytes).as_bytes()
}

pub fn sha256_hash(bytes: &[u8]) -> Hash {
    sha256(bytes)
}

pub fn packet_hash(packet: &WorkPacket) -> Result<Hash, WorkProtocolError> {
    Ok(blake3_hash(&packet_bytes(packet)?))
}

pub fn packet_bytes(packet: &WorkPacket) -> Result<Vec<u8>, WorkProtocolError> {
    let bytes = to_bytes::<WireError>(packet).map_err(|_| WorkProtocolError::InvalidPacket)?;
    if bytes.len() > MAX_WORK_FRAME_LEN {
        return Err(WorkProtocolError::PacketTooLarge);
    }
    Ok(bytes.to_vec())
}

pub fn packet_from_bytes(bytes: &[u8]) -> Result<WorkPacket, WorkProtocolError> {
    if bytes.is_empty() {
        return Err(WorkProtocolError::EmptyPacket);
    }
    if bytes.len() > MAX_WORK_FRAME_LEN {
        return Err(WorkProtocolError::PacketTooLarge);
    }
    if bytes
        .as_ptr()
        .align_offset(core::mem::align_of::<ArchivedWorkPacket>())
        != 0
    {
        let mut aligned = util::AlignedVec::<16>::with_capacity(bytes.len());
        aligned.extend_from_slice(bytes);
        return packet_from_aligned_bytes(aligned.as_slice());
    }
    packet_from_aligned_bytes(bytes)
}

fn packet_from_aligned_bytes(bytes: &[u8]) -> Result<WorkPacket, WorkProtocolError> {
    let archived = access::<ArchivedWorkPacket, WireError>(bytes)
        .map_err(|_| WorkProtocolError::InvalidPacket)?;
    deserialize::<WorkPacket, WireError>(archived).map_err(|_| WorkProtocolError::InvalidPacket)
}

pub fn derive_node_id(public_key: &PublicKey, role: u16) -> NodeId {
    let mut input = Vec::with_capacity(34);
    input.extend_from_slice(public_key);
    input.extend_from_slice(&role.to_be_bytes());
    blake3_hash(&input)
}

pub fn empty_signature() -> WorkSignature {
    WorkSignature {
        algorithm: SIGNATURE_ALGORITHM_SOLANA_ED25519,
        public_key: Vec::new(),
        signature: Vec::new(),
    }
}

pub fn node_identity_from_key(key: &Ed25519SigningKey, role: u16) -> NodeIdentity {
    let mut public_key = [0u8; 32];
    public_key.copy_from_slice(key.verifying_key().as_bytes());
    NodeIdentity {
        node_id: derive_node_id(&public_key, role),
        role,
        public_key,
    }
}

pub fn sign_ed25519(key: &Ed25519SigningKey, preimage: &[u8]) -> WorkSignature {
    WorkSignature {
        algorithm: SIGNATURE_ALGORITHM_SOLANA_ED25519,
        public_key: key.verifying_key().as_bytes().to_vec(),
        signature: key.sign(preimage).to_bytes().to_vec(),
    }
}

pub fn verify_signature(identity: &NodeIdentity, signature: &WorkSignature, preimage: &[u8]) -> bool {
    if signature.algorithm != SIGNATURE_ALGORITHM_SOLANA_ED25519 {
        return false;
    }
    if signature.public_key.as_slice() != identity.public_key {
        return false;
    }
    verify_solana_ed25519(&identity.public_key, preimage, &signature.signature)
}

pub fn verify_solana_ed25519(public_key: &PublicKey, preimage: &[u8], signature: &[u8]) -> bool {
    if signature.len() != 64 {
        return false;
    }
    let Ok(verifying_key) = edgerun_crypto::ed25519_dalek::VerifyingKey::from_bytes(public_key) else {
        return false;
    };
    let Ok(signature) = edgerun_crypto::ed25519_dalek::Signature::from_slice(signature) else {
        return false;
    };
    verifying_key.verify_strict(preimage, &signature).is_ok()
}

pub fn node_available_preimage(value: &NodeAvailable) -> Vec<u8> {
    let mut out = domain(NODE_AVAILABLE_DOMAIN);
    encode_node(&mut out, &value.node);
    out.extend_from_slice(&value.sequence.to_be_bytes());
    out.extend_from_slice(&value.unix_ms.to_be_bytes());
    encode_bytes(&mut out, value.listen_host.as_bytes());
    out.extend_from_slice(&value.listen_port.to_be_bytes());
    out.extend_from_slice(&value.heartbeat_secs.to_be_bytes());
    out.extend_from_slice(&value.log_head);
    out
}

pub fn node_heartbeat_preimage(value: &NodeHeartbeat) -> Vec<u8> {
    let mut out = domain(NODE_HEARTBEAT_DOMAIN);
    encode_node(&mut out, &value.node);
    out.extend_from_slice(&value.sequence.to_be_bytes());
    out.extend_from_slice(&value.unix_ms.to_be_bytes());
    out.extend_from_slice(&value.connection_hash);
    out.extend_from_slice(&value.log_head);
    out
}

pub fn relay_assignment_preimage(value: &RelayAssignment) -> Vec<u8> {
    let mut out = domain(NODE_ASSIGNMENT_DOMAIN);
    out.extend_from_slice(&value.node_id);
    encode_relay(&mut out, &value.relay);
    encode_node(&mut out, &value.assigned_by);
    out.extend_from_slice(&value.sequence.to_be_bytes());
    out.extend_from_slice(&value.valid_until_unix_ms.to_be_bytes());
    out
}

pub fn network_message_preimage(value: &NetworkMessage) -> Vec<u8> {
    let mut out = domain(NETWORK_MESSAGE_DOMAIN);
    out.extend_from_slice(&value.message_id);
    out.extend_from_slice(&value.prev_hash);
    out.extend_from_slice(&value.from);
    out.extend_from_slice(&value.to);
    out.extend_from_slice(&value.via_relay);
    out.extend_from_slice(&value.department.to_be_bytes());
    out.extend_from_slice(&value.work_type.to_be_bytes());
    out.extend_from_slice(&value.sequence.to_be_bytes());
    out.extend_from_slice(&value.payload_hash);
    encode_bytes(&mut out, &value.payload);
    out
}

pub fn work_admission_preimage(value: &WorkAdmission) -> Vec<u8> {
    let mut out = domain(WORK_ADMISSION_DOMAIN);
    out.extend_from_slice(&value.admission_id);
    out.extend_from_slice(&value.dao_id);
    encode_node(&mut out, &value.admission_node);
    out.extend_from_slice(&value.request_hash);
    encode_relay(&mut out, &value.assigned_relay);
    out.extend_from_slice(&value.admitted_budget.to_be_bytes());
    out.extend_from_slice(&value.policy_hash);
    out.extend_from_slice(&value.sequence.to_be_bytes());
    out.extend_from_slice(&value.valid_until_unix_ms.to_be_bytes());
    out
}

pub fn work_receipt_preimage(value: &WorkReceipt) -> Vec<u8> {
    let mut out = domain(WORK_RECEIPT_DOMAIN);
    out.extend_from_slice(&value.receipt_id);
    out.extend_from_slice(&value.request_hash);
    out.extend_from_slice(&value.admission_hash);
    encode_node(&mut out, &value.worker);
    out.extend_from_slice(&value.relay_node_id);
    out.extend_from_slice(&value.input_hash);
    out.extend_from_slice(&value.output_hash);
    out.extend_from_slice(&value.units_used.to_be_bytes());
    out.extend_from_slice(&value.total_claim.to_be_bytes());
    out.extend_from_slice(&value.sequence.to_be_bytes());
    out
}

pub fn sign_node_available(key: &Ed25519SigningKey, mut value: NodeAvailable) -> NodeAvailable {
    value.signature = sign_ed25519(key, &node_available_preimage(&value));
    value
}

pub fn sign_node_heartbeat(key: &Ed25519SigningKey, mut value: NodeHeartbeat) -> NodeHeartbeat {
    value.signature = sign_ed25519(key, &node_heartbeat_preimage(&value));
    value
}

pub fn sign_relay_assignment(key: &Ed25519SigningKey, mut value: RelayAssignment) -> RelayAssignment {
    value.signature = sign_ed25519(key, &relay_assignment_preimage(&value));
    value
}

pub fn sign_network_message(key: &Ed25519SigningKey, mut value: NetworkMessage) -> NetworkMessage {
    value.payload_hash = blake3_hash(&value.payload);
    value.signature = sign_ed25519(key, &network_message_preimage(&value));
    value
}

pub fn sign_work_admission(key: &Ed25519SigningKey, mut value: WorkAdmission) -> WorkAdmission {
    value.signature = sign_ed25519(key, &work_admission_preimage(&value));
    value
}

pub fn sign_work_receipt(key: &Ed25519SigningKey, mut value: WorkReceipt) -> WorkReceipt {
    value.signature = sign_ed25519(key, &work_receipt_preimage(&value));
    value
}

pub fn verify_node_available(value: &NodeAvailable) -> bool {
    value.abi_version == WORK_WIRE_ABI_VERSION
        && value.node.role != 0
        && value.node.node_id == derive_node_id(&value.node.public_key, value.node.role)
        && verify_signature(&value.node, &value.signature, &node_available_preimage(value))
}

pub fn verify_node_heartbeat(value: &NodeHeartbeat) -> bool {
    value.abi_version == WORK_WIRE_ABI_VERSION
        && value.node.node_id == derive_node_id(&value.node.public_key, value.node.role)
        && verify_signature(&value.node, &value.signature, &node_heartbeat_preimage(value))
}

pub fn verify_relay_assignment(value: &RelayAssignment) -> bool {
    value.abi_version == WORK_WIRE_ABI_VERSION
        && value.assigned_by.role == NODE_ROLE_ADMISSION
        && verify_signature(&value.assigned_by, &value.signature, &relay_assignment_preimage(value))
}

pub fn verify_network_message(value: &NetworkMessage, signer: &NodeIdentity) -> bool {
    value.abi_version == WORK_WIRE_ABI_VERSION
        && value.from == signer.node_id
        && value.payload_hash == blake3_hash(&value.payload)
        && verify_signature(signer, &value.signature, &network_message_preimage(value))
}

pub fn verify_work_admission(value: &WorkAdmission) -> bool {
    value.abi_version == WORK_WIRE_ABI_VERSION
        && value.admission_node.role == NODE_ROLE_ADMISSION
        && verify_signature(&value.admission_node, &value.signature, &work_admission_preimage(value))
}

pub fn verify_work_receipt(value: &WorkReceipt) -> bool {
    value.abi_version == WORK_WIRE_ABI_VERSION
        && verify_signature(&value.worker, &value.signature, &work_receipt_preimage(value))
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

fn encode_relay(out: &mut Vec<u8>, relay: &RelayEndpoint) {
    out.extend_from_slice(&relay.relay_node_id);
    encode_bytes(out, relay.host.as_bytes());
    out.extend_from_slice(&relay.port.to_be_bytes());
}

fn encode_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
    out.extend_from_slice(bytes);
}
