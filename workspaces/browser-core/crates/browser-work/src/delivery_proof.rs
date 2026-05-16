use alloc::vec::Vec;

use edgerun_crypto::Ed25519SigningKey;

use crate::channel::ChannelProof;
use crate::channel_order::{ordered_message_hash, OrderedChannelEnvelope};
use crate::preimage::{HashBuilder, PreimageBuilder};
use crate::protocol::{Hash, NodeId, NodeIdentity, WORK_WIRE_ABI_VERSION};
use crate::signing::{empty_signature, sign_ed25519, verify_signature};

const CHANNEL_PROOF_DOMAIN: &[u8] = b"edgerun:v1:work:channel-proof";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChannelProofError {
    BadAbi,
    WrongRecipient,
    WrongRelay,
    WrongMessage,
    InvalidSignature,
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
) -> Result<ChannelProof, ChannelProofError> {
    if ordered.envelope.to != recipient.node_id {
        return Err(ChannelProofError::WrongRecipient);
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

pub fn verify_channel_proof_for_ordered(
    proof: &ChannelProof,
    recipient: &NodeIdentity,
    relay_node_id: NodeId,
    ordered: &OrderedChannelEnvelope,
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
        || proof.message_hash != ordered_message_hash(ordered)
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
