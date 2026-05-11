use alloc::vec;
use alloc::vec::Vec;

use edgerun_crypto::Ed25519SigningKey;
use rkyv::{Archive, Deserialize, Serialize};

use crate::codec::{blake3_hash, empty_signature, sign_ed25519, verify_signature};
use crate::protocol::*;

const RECIPIENT_MESSAGE_POLICY_DOMAIN: &[u8] = b"edgerun:v1:work:recipient-message-policy";

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RecipientMessagePolicy {
    pub abi_version: u16,
    pub recipient: NodeIdentity,
    pub sequence: u64,
    pub valid_until_unix_ms: u64,
    pub allow_unknown_senders: bool,
    pub allowed_senders: Vec<NodeId>,
    pub blocked_senders: Vec<NodeId>,
    pub allowed_relays: Vec<NodeId>,
    pub allowed_departments: Vec<u16>,
    pub allowed_work_types: Vec<u16>,
    pub max_payload_bytes: u64,
    pub policy_hash: Hash,
    pub signature: WorkSignature,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecipientPolicyError {
    InvalidPolicy,
    ExpiredPolicy,
    WrongRecipient,
    SenderBlocked,
    SenderNotAllowed,
    RelayNotAllowed,
    DepartmentNotAllowed,
    WorkTypeNotAllowed,
    PayloadTooLarge,
}

pub fn recipient_message_policy_preimage(value: &RecipientMessagePolicy) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(RECIPIENT_MESSAGE_POLICY_DOMAIN);
    out.push(0);
    out.extend_from_slice(&value.recipient.node_id);
    out.extend_from_slice(&value.recipient.role.to_be_bytes());
    out.extend_from_slice(&value.recipient.public_key);
    out.extend_from_slice(&value.sequence.to_be_bytes());
    out.extend_from_slice(&value.valid_until_unix_ms.to_be_bytes());
    out.push(value.allow_unknown_senders as u8);
    encode_node_id_list(&mut out, &value.allowed_senders);
    encode_node_id_list(&mut out, &value.blocked_senders);
    encode_node_id_list(&mut out, &value.allowed_relays);
    encode_u16_list(&mut out, &value.allowed_departments);
    encode_u16_list(&mut out, &value.allowed_work_types);
    out.extend_from_slice(&value.max_payload_bytes.to_be_bytes());
    out.extend_from_slice(&value.policy_hash);
    out
}

pub fn sign_recipient_message_policy(
    key: &Ed25519SigningKey,
    mut value: RecipientMessagePolicy,
) -> RecipientMessagePolicy {
    value.signature = sign_ed25519(key, &recipient_message_policy_preimage(&value));
    value
}

pub fn verify_recipient_message_policy(value: &RecipientMessagePolicy) -> bool {
    value.abi_version == WORK_WIRE_ABI_VERSION
        && value.recipient.role == NODE_ROLE_MESSAGE
        && verify_signature(
            &value.recipient,
            &value.signature,
            &recipient_message_policy_preimage(value),
        )
}

pub fn recipient_message_policy_hash(value: &RecipientMessagePolicy) -> Hash {
    blake3_hash(&recipient_message_policy_preimage(value))
}

pub fn recipient_message_policy_allows(
    policy: &RecipientMessagePolicy,
    message: &NetworkMessage,
    now_unix_ms: u64,
) -> Result<(), RecipientPolicyError> {
    if !verify_recipient_message_policy(policy) {
        return Err(RecipientPolicyError::InvalidPolicy);
    }
    if policy.valid_until_unix_ms < now_unix_ms {
        return Err(RecipientPolicyError::ExpiredPolicy);
    }
    if message.to != policy.recipient.node_id {
        return Err(RecipientPolicyError::WrongRecipient);
    }
    if contains_node_id(&policy.blocked_senders, &message.from) {
        return Err(RecipientPolicyError::SenderBlocked);
    }
    if !policy.allow_unknown_senders && !contains_node_id(&policy.allowed_senders, &message.from) {
        return Err(RecipientPolicyError::SenderNotAllowed);
    }
    if !policy.allowed_relays.is_empty() && !contains_node_id(&policy.allowed_relays, &message.via_relay) {
        return Err(RecipientPolicyError::RelayNotAllowed);
    }
    if !policy.allowed_departments.is_empty() && !policy.allowed_departments.contains(&message.department) {
        return Err(RecipientPolicyError::DepartmentNotAllowed);
    }
    if !policy.allowed_work_types.is_empty() && !policy.allowed_work_types.contains(&message.work_type) {
        return Err(RecipientPolicyError::WorkTypeNotAllowed);
    }
    if message.payload.len() as u64 > policy.max_payload_bytes {
        return Err(RecipientPolicyError::PayloadTooLarge);
    }
    Ok(())
}

pub fn open_recipient_message_policy(
    recipient: NodeIdentity,
    sequence: u64,
    valid_until_unix_ms: u64,
) -> RecipientMessagePolicy {
    RecipientMessagePolicy {
        abi_version: WORK_WIRE_ABI_VERSION,
        recipient,
        sequence,
        valid_until_unix_ms,
        allow_unknown_senders: true,
        allowed_senders: Vec::new(),
        blocked_senders: Vec::new(),
        allowed_relays: Vec::new(),
        allowed_departments: vec![DEPARTMENT_MESSAGE],
        allowed_work_types: vec![WORK_TYPE_MESSAGE_DELIVER],
        max_payload_bytes: MAX_WORK_FRAME_LEN as u64,
        policy_hash: [0u8; 32],
        signature: empty_signature(),
    }
}

pub fn allowlist_recipient_message_policy(
    recipient: NodeIdentity,
    allowed_senders: Vec<NodeId>,
    sequence: u64,
    valid_until_unix_ms: u64,
) -> RecipientMessagePolicy {
    RecipientMessagePolicy {
        abi_version: WORK_WIRE_ABI_VERSION,
        recipient,
        sequence,
        valid_until_unix_ms,
        allow_unknown_senders: false,
        allowed_senders,
        blocked_senders: Vec::new(),
        allowed_relays: Vec::new(),
        allowed_departments: vec![DEPARTMENT_MESSAGE],
        allowed_work_types: vec![WORK_TYPE_MESSAGE_DELIVER],
        max_payload_bytes: MAX_WORK_FRAME_LEN as u64,
        policy_hash: [0u8; 32],
        signature: empty_signature(),
    }
}

fn contains_node_id(values: &[NodeId], value: &NodeId) -> bool {
    values.iter().any(|candidate| candidate == value)
}

fn encode_node_id_list(out: &mut Vec<u8>, values: &[NodeId]) {
    out.extend_from_slice(&(values.len() as u64).to_be_bytes());
    for value in values {
        out.extend_from_slice(value);
    }
}

fn encode_u16_list(out: &mut Vec<u8>, values: &[u16]) {
    out.extend_from_slice(&(values.len() as u64).to_be_bytes());
    for value in values {
        out.extend_from_slice(&value.to_be_bytes());
    }
}
