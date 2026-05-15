use alloc::vec::Vec;

use edgerun_crypto::Ed25519SigningKey;

use crate::channel::ChannelProof;
use crate::channel_order::{ordered_message_hash, OrderedChannelEnvelope};
use crate::preimage::{HashBuilder, PreimageBuilder};
use crate::protocol::{
    Hash, NodeId, NodeIdentity, WorkPacket, WorkProtocolError, WORK_WIRE_ABI_VERSION,
};
use crate::recipient_policy::{
    recipient_message_policy_allows, recipient_message_policy_hash, RecipientMessagePolicy,
    RecipientPolicyError,
};
use crate::signing::{empty_signature, sign_ed25519, verify_signature};

const CHANNEL_PROOF_DOMAIN: &[u8] = b"edgerun:v1:work:channel-proof";
const POLICY_BOUND_MESSAGE_DOMAIN: &[u8] = b"edgerun:v1:work:policy-bound-channel-message";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChannelProofError {
    BadAbi,
    WrongRecipient,
    WrongRelay,
    WrongMessage,
    InvalidSignature,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolicyBoundChannelProofError {
    NotNetworkMessage,
    PolicyRejected(RecipientPolicyError),
    Proof(WorkProtocolError),
}

pub fn channel_proof_preimage(value: &ChannelProof) -> Vec<u8> {
    PreimageBuilder::domain(CHANNEL_PROOF_DOMAIN)
        .hash(&value.channel_id)
        .node_id(&value.relay_node_id)
        .node_id(&value.from)
        .node_id(&value.to)
        .hash(&value.message_hash)
        .u64(value.sequence)
        .finish()
}

pub fn policy_bound_ordered_message_hash(
    ordered: &OrderedChannelEnvelope,
    policy_hash: Hash,
) -> Hash {
    HashBuilder::domain(POLICY_BOUND_MESSAGE_DOMAIN)
        .hash(&ordered_message_hash(ordered))
        .hash(&policy_hash)
        .finish()
}

pub fn sign_channel_proof(key: &Ed25519SigningKey, mut value: ChannelProof) -> ChannelProof {
    value.signature = sign_ed25519(key, &channel_proof_preimage(&value));
    value
}

pub fn verify_channel_proof(value: &ChannelProof, recipient: &NodeIdentity) -> bool {
    value.abi_version == WORK_WIRE_ABI_VERSION
        && value.to == recipient.node_id
        && verify_signature(recipient, &value.signature, &channel_proof_preimage(value))
}

pub fn channel_proof_for_ordered(
    recipient_key: &Ed25519SigningKey,
    recipient: &NodeIdentity,
    relay_node_id: NodeId,
    ordered: &OrderedChannelEnvelope,
) -> Result<ChannelProof, WorkProtocolError> {
    if ordered.envelope.to != recipient.node_id {
        return Err(WorkProtocolError::WrongRelay);
    }
    Ok(sign_channel_proof(
        recipient_key,
        ChannelProof {
            abi_version: WORK_WIRE_ABI_VERSION,
            channel_id: ordered.envelope.channel_id,
            relay_node_id,
            from: ordered.envelope.from,
            to: ordered.envelope.to,
            message_hash: ordered_message_hash(ordered),
            sequence: ordered.sequence,
            signature: empty_signature(),
        },
    ))
}

pub fn channel_proof_for_ordered_with_policy(
    recipient_key: &Ed25519SigningKey,
    recipient: &NodeIdentity,
    relay_node_id: NodeId,
    ordered: &OrderedChannelEnvelope,
    policy_hash: Hash,
) -> Result<ChannelProof, WorkProtocolError> {
    if ordered.envelope.to != recipient.node_id {
        return Err(WorkProtocolError::WrongRelay);
    }
    Ok(sign_channel_proof(
        recipient_key,
        ChannelProof {
            abi_version: WORK_WIRE_ABI_VERSION,
            channel_id: ordered.envelope.channel_id,
            relay_node_id,
            from: ordered.envelope.from,
            to: ordered.envelope.to,
            message_hash: policy_bound_ordered_message_hash(ordered, policy_hash),
            sequence: ordered.sequence,
            signature: empty_signature(),
        },
    ))
}

pub fn channel_proof_for_allowed_ordered_message(
    recipient_key: &Ed25519SigningKey,
    recipient: &NodeIdentity,
    relay_node_id: NodeId,
    ordered: &OrderedChannelEnvelope,
    policy: &RecipientMessagePolicy,
    now_unix_ms: u64,
) -> Result<ChannelProof, PolicyBoundChannelProofError> {
    let WorkPacket::NetworkMessage(message) = &ordered.envelope.packet else {
        return Err(PolicyBoundChannelProofError::NotNetworkMessage);
    };
    recipient_message_policy_allows(policy, message, now_unix_ms)
        .map_err(PolicyBoundChannelProofError::PolicyRejected)?;
    channel_proof_for_ordered_with_policy(
        recipient_key,
        recipient,
        relay_node_id,
        ordered,
        recipient_message_policy_hash(policy),
    )
    .map_err(PolicyBoundChannelProofError::Proof)
}

pub fn verify_channel_proof_for_ordered(
    proof: &ChannelProof,
    recipient: &NodeIdentity,
    relay_node_id: NodeId,
    ordered: &OrderedChannelEnvelope,
) -> Result<(), ChannelProofError> {
    verify_channel_proof_for_message_hash(
        proof,
        recipient,
        relay_node_id,
        ordered,
        ordered_message_hash(ordered),
    )
}

pub fn verify_channel_proof_for_ordered_with_policy(
    proof: &ChannelProof,
    recipient: &NodeIdentity,
    relay_node_id: NodeId,
    ordered: &OrderedChannelEnvelope,
    policy_hash: Hash,
) -> Result<(), ChannelProofError> {
    verify_channel_proof_for_message_hash(
        proof,
        recipient,
        relay_node_id,
        ordered,
        policy_bound_ordered_message_hash(ordered, policy_hash),
    )
}

fn verify_channel_proof_for_message_hash(
    proof: &ChannelProof,
    recipient: &NodeIdentity,
    relay_node_id: NodeId,
    ordered: &OrderedChannelEnvelope,
    expected_message_hash: Hash,
) -> Result<(), ChannelProofError> {
    if proof.abi_version != WORK_WIRE_ABI_VERSION {
        return Err(ChannelProofError::BadAbi);
    }
    if proof.to != recipient.node_id || ordered.envelope.to != recipient.node_id {
        return Err(ChannelProofError::WrongRecipient);
    }
    if proof.relay_node_id != relay_node_id {
        return Err(ChannelProofError::WrongRelay);
    }
    if proof.channel_id != ordered.envelope.channel_id
        || proof.from != ordered.envelope.from
        || proof.to != ordered.envelope.to
        || proof.sequence != ordered.sequence
        || proof.message_hash != expected_message_hash
    {
        return Err(ChannelProofError::WrongMessage);
    }
    if !verify_channel_proof(proof, recipient) {
        return Err(ChannelProofError::InvalidSignature);
    }
    Ok(())
}

pub fn channel_proof_hash(proof: &ChannelProof) -> Hash {
    HashBuilder::domain(CHANNEL_PROOF_DOMAIN)
        .bytes(&channel_proof_preimage(proof))
        .finish()
}
