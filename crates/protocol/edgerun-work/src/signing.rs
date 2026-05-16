use alloc::vec::Vec;

use edgerun_crypto::Ed25519SigningKey;

use crate::codec::blake3_hash;
use crate::identity::verify_node_identity;
use crate::preimage::PreimageBuilder;
use crate::protocol::*;

const NODE_AVAILABLE_DOMAIN: &[u8] = b"edgerun:v1:work:node-available";
const NODE_HEARTBEAT_DOMAIN: &[u8] = b"edgerun:v1:work:node-heartbeat";
const NODE_ASSIGNMENT_DOMAIN: &[u8] = b"edgerun:v1:work:node-assignment";
const NETWORK_MESSAGE_DOMAIN: &[u8] = b"edgerun:v1:work:network-message";
const WORK_ADMISSION_DOMAIN: &[u8] = b"edgerun:v1:work:admission";
const WORK_RECEIPT_DOMAIN: &[u8] = b"edgerun:v1:work:receipt";

pub fn empty_signature() -> WorkSignature {
    WorkSignature {
        algorithm: SIGNATURE_ALGORITHM_SOLANA_ED25519,
        public_key: Vec::new(),
        signature: Vec::new(),
    }
}

pub fn sign_ed25519(key: &Ed25519SigningKey, preimage: &[u8]) -> WorkSignature {
    WorkSignature {
        algorithm: SIGNATURE_ALGORITHM_SOLANA_ED25519,
        public_key: key.verifying_key().as_bytes().to_vec(),
        signature: key.sign(preimage).to_bytes().to_vec(),
    }
}

pub fn verify_signature(
    identity: &NodeIdentity,
    signature: &WorkSignature,
    preimage: &[u8],
) -> bool {
    if !verify_node_identity(identity) {
        return false;
    }
    if signature.algorithm != SIGNATURE_ALGORITHM_SOLANA_ED25519 {
        return false;
    }
    if signature.public_key.as_slice() != &identity.public_key[..] {
        return false;
    }
    verify_solana_ed25519(&identity.public_key, preimage, &signature.signature)
}

pub fn verify_solana_ed25519(public_key: &PublicKey, preimage: &[u8], signature: &[u8]) -> bool {
    edgerun_crypto::verification::ed25519_verify_strict(public_key, preimage, signature).is_ok()
}

pub fn node_available_preimage(value: &NodeAvailable) -> Vec<u8> {
    let mut builder = PreimageBuilder::domain(NODE_AVAILABLE_DOMAIN)
        .node(&value.node)
        .u16(value.relay_endpoint.is_some() as u16);
    if let Some(endpoint) = &value.relay_endpoint {
        builder = builder.channel(endpoint);
    }
    builder
        .u64(value.sequence)
        .u64(value.unix_ms)
        .u64(value.heartbeat_secs)
        .hash(&value.log_head)
        .finish()
}

pub fn node_heartbeat_preimage(value: &NodeHeartbeat) -> Vec<u8> {
    PreimageBuilder::domain(NODE_HEARTBEAT_DOMAIN)
        .node(&value.node)
        .u64(value.sequence)
        .u64(value.unix_ms)
        .hash(&value.connection_hash)
        .hash(&value.log_head)
        .finish()
}

pub fn relay_assignment_preimage(value: &RelayAssignment) -> Vec<u8> {
    PreimageBuilder::domain(NODE_ASSIGNMENT_DOMAIN)
        .node_id(&value.node_id)
        .relay(&value.relay)
        .node(&value.assigned_by)
        .u64(value.sequence)
        .u64(value.valid_until_unix_ms)
        .finish()
}

pub fn network_message_preimage(value: &NetworkMessage) -> Vec<u8> {
    PreimageBuilder::domain(NETWORK_MESSAGE_DOMAIN)
        .hash(&value.message_id)
        .hash(&value.prev_hash)
        .node_id(&value.from)
        .node_id(&value.to)
        .node_id(&value.via_relay)
        .u16(value.department)
        .u16(value.work_type)
        .u64(value.sequence)
        .hash(&value.payload_hash)
        .bytes(&value.payload)
        .finish()
}

pub fn simple_network_message_id(
    from: &NodeId,
    to: &NodeId,
    sequence: u64,
    payload_hash: &Hash,
) -> Hash {
    let mut id_input = [0u8; 104];
    id_input[..32].copy_from_slice(from);
    id_input[32..64].copy_from_slice(to);
    id_input[64..72].copy_from_slice(&sequence.to_be_bytes());
    id_input[72..104].copy_from_slice(payload_hash);
    blake3_hash(&id_input)
}

pub fn work_admission_preimage(value: &WorkAdmission) -> Vec<u8> {
    PreimageBuilder::domain(WORK_ADMISSION_DOMAIN)
        .hash(&value.admission_id)
        .hash(&value.dao_id)
        .hash(&value.user)
        .node(&value.admission_node)
        .hash(&value.request_hash)
        .hash(&value.assigned_route_commitment)
        .channel(&value.assigned_channel)
        .hash_list(&value.assigned_relay_path)
        .u64(value.admitted_budget)
        .hash(&value.policy_hash)
        .u64(value.sequence)
        .u64(value.valid_until_unix_ms)
        .finish()
}

pub fn work_receipt_preimage(value: &WorkReceipt) -> Vec<u8> {
    PreimageBuilder::domain(WORK_RECEIPT_DOMAIN)
        .hash(&value.receipt_id)
        .hash(&value.request_hash)
        .hash(&value.admission_hash)
        .node(&value.worker)
        .node_id(&value.relay_node_id)
        .hash(&value.input_hash)
        .hash(&value.output_hash)
        .u64(value.units_used)
        .u64(value.total_claim)
        .u64(value.sequence)
        .finish()
}

pub fn sign_node_available(key: &Ed25519SigningKey, mut value: NodeAvailable) -> NodeAvailable {
    value.signature = sign_ed25519(key, &node_available_preimage(&value));
    value
}

pub fn sign_node_heartbeat(key: &Ed25519SigningKey, mut value: NodeHeartbeat) -> NodeHeartbeat {
    value.signature = sign_ed25519(key, &node_heartbeat_preimage(&value));
    value
}

pub fn sign_relay_assignment(
    key: &Ed25519SigningKey,
    mut value: RelayAssignment,
) -> RelayAssignment {
    value.signature = sign_ed25519(key, &relay_assignment_preimage(&value));
    value
}

pub fn sign_network_message(key: &Ed25519SigningKey, mut value: NetworkMessage) -> NetworkMessage {
    value.payload_hash = blake3_hash(&value.payload);
    sign_network_message_with_payload_hash(key, value)
}

pub fn sign_network_message_with_payload_hash(
    key: &Ed25519SigningKey,
    mut value: NetworkMessage,
) -> NetworkMessage {
    debug_assert_eq!(value.payload_hash, blake3_hash(&value.payload));
    value.signature = sign_ed25519(key, &network_message_preimage(&value));
    value
}

#[allow(clippy::too_many_arguments)]
pub fn sign_network_message_payload(
    key: &Ed25519SigningKey,
    message_id: Hash,
    prev_hash: Hash,
    from: NodeId,
    to: NodeId,
    via_relay: NodeId,
    department: u16,
    work_type: u16,
    sequence: u64,
    payload: Vec<u8>,
) -> NetworkMessage {
    let payload_hash = blake3_hash(&payload);
    sign_network_message_with_payload_hash(
        key,
        NetworkMessage {
            abi_version: WORK_WIRE_ABI_VERSION,
            message_id,
            prev_hash,
            from,
            to,
            via_relay,
            department,
            work_type,
            sequence,
            payload_hash,
            payload,
            signature: empty_signature(),
        },
    )
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
        && verify_signature(
            &value.node,
            &value.signature,
            &node_available_preimage(value),
        )
}

pub fn verify_node_heartbeat(value: &NodeHeartbeat) -> bool {
    value.abi_version == WORK_WIRE_ABI_VERSION
        && verify_signature(
            &value.node,
            &value.signature,
            &node_heartbeat_preimage(value),
        )
}

pub fn verify_relay_assignment(value: &RelayAssignment) -> bool {
    value.abi_version == WORK_WIRE_ABI_VERSION
        && value.assigned_by.role == NODE_ROLE_ADMISSION
        && verify_signature(
            &value.assigned_by,
            &value.signature,
            &relay_assignment_preimage(value),
        )
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
        && verify_signature(
            &value.admission_node,
            &value.signature,
            &work_admission_preimage(value),
        )
}

pub fn verify_work_receipt(value: &WorkReceipt) -> bool {
    value.abi_version == WORK_WIRE_ABI_VERSION
        && verify_signature(
            &value.worker,
            &value.signature,
            &work_receipt_preimage(value),
        )
}
