//! Command processing for the Lifegraph protocol.
//!
//! A command is an external signed request directed at a node.
//! The node validates the command and decides whether to commit or reject it.
//!
//! ## Protocol invariants (§5)
//!
//! - A command is not authoritative until the target node validates it
//!   and records the outcome in its own stream
//! - Delivery alone has no effect on authoritative node state
//! - If rejected, the node records a `command_rejected` event
//! - If committed, the node records a `command_committed` event

use crate::crypto::node_id_to_verifying_key;
use crate::protocol::{canonical_bytes, CommandEnvelope, IdentityRef, ProtocolRecord};
use p256::ecdsa::signature::hazmat::PrehashVerifier;
use sha2::{Digest, Sha256};

/// Result of processing a command.
#[derive(Debug)]
pub enum CommandOutcome {
    Valid,
    MissingSignature,
    InvalidPublicKey,
    InvalidSignature,
}

/// Validates a command's signature using canonical protobuf encoding.
pub fn validate_command(command: &CommandEnvelope) -> CommandOutcome {
    let Some(sig) = &command.signature else {
        return CommandOutcome::MissingSignature;
    };

    let Some(public_key) = extract_public_key(&command.issuer) else {
        return CommandOutcome::InvalidPublicKey;
    };

    if sig.algorithm != 2 {
        // SIGNATURE_ALGORITHM_ECDSA_P256_SHA256 = 2
        return CommandOutcome::InvalidSignature;
    }

    // Hash the canonical protobuf form (signable = clears signature field)
    let record = ProtocolRecord::CommandEnvelope(command.clone());
    let canonical = canonical_bytes(&record, true);
    let digest = Sha256::digest(&canonical);

    if !verify_ecdsa_p256(&public_key, &digest, &sig.value) {
        return CommandOutcome::InvalidSignature;
    }

    CommandOutcome::Valid
}

fn extract_public_key(identity: &Option<IdentityRef>) -> Option<[u8; 64]> {
    let identity = identity.as_ref()?;
    let hint = identity.key_hint.as_ref()?;
    if hint.len() == 64 {
        let mut bytes = [0u8; 64];
        bytes.copy_from_slice(hint);
        Some(bytes)
    } else {
        None
    }
}

fn verify_ecdsa_p256(public_key: &[u8; 64], digest: &[u8], signature: &[u8]) -> bool {
    let vk = match node_id_to_verifying_key(public_key) {
        Some(vk) => vk,
        None => return false,
    };

    if signature.len() != 64 {
        return false;
    }

    let digest_array: [u8; 32] = match digest.try_into() {
        Ok(d) => d,
        Err(_) => return false,
    };

    // Convert r||s to DER encoding for verification
    let r_bytes = &signature[..32];
    let s_bytes = &signature[32..];
    let r_der = integer_to_der(r_bytes);
    let s_der = integer_to_der(s_bytes);

    let mut der = Vec::new();
    der.push(0x30);
    der.push(r_der.len() as u8);
    der.extend_from_slice(&r_der);
    der.push(s_der.len() as u8);
    der.extend_from_slice(&s_der);

    let len_byte = der.len() - 2;
    if len_byte > 127 {
        return false;
    }
    let mut final_der = Vec::new();
    final_der.push(0x30);
    final_der.push(len_byte as u8);
    final_der.extend_from_slice(&der[2..]);

    match p256::ecdsa::Signature::from_der(&final_der) {
        Ok(sig) => vk.verify_prehash(&digest_array, &sig).is_ok(),
        Err(_) => false,
    }
}

fn integer_to_der(bytes: &[u8]) -> Vec<u8> {
    let mut trimmed = bytes.iter().skip_while(|&&b| b == 0).copied().collect::<Vec<_>>();
    if trimmed.is_empty() {
        trimmed = vec![0];
    } else if trimmed[0] & 0x80 != 0 {
        trimmed.insert(0, 0);
    }
    let mut result = vec![0x02, trimmed.len() as u8];
    result.extend_from_slice(&trimmed);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{CommandEnvelope, IdentityRef, NodeRef, Signature};
    use lifegraph_proto::lifegraph::v0::common::Signature as ProtoSignature;

    fn make_unsigned_command() -> CommandEnvelope {
        CommandEnvelope {
            envelope_version: 1,
            command_id: vec![1, 2, 3],
            target_node: Some(NodeRef {
                node_id: vec![4, 5, 6],
            }),
            issuer: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: Some(1), // USER
                key_hint: None,
            }),
            command_type: 7, // QUERY
            command_version: 1,
            issued_at: None,
            not_before: None,
            expires_at: None,
            idempotency_key: Vec::new(),
            payload: None,
            delegation_chain: vec![],
            requested_assurance: None,
            command_metadata: None,
            signature: None,
        }
    }

    #[test]
    fn command_without_signature_is_rejected() {
        let command = make_unsigned_command();
        assert!(matches!(
            validate_command(&command),
            CommandOutcome::MissingSignature
        ));
    }

    #[test]
    fn command_with_invalid_signature_is_rejected() {
        let mut command = make_unsigned_command();
        command.signature = Some(ProtoSignature {
            algorithm: 2, // ECDSA_P256_SHA256
            value: vec![0u8; 64],
        });
        let outcome = validate_command(&command);
        assert!(matches!(
            outcome,
            CommandOutcome::InvalidSignature | CommandOutcome::InvalidPublicKey
        ));
    }

    #[test]
    fn canonical_command_deterministic() {
        let cmd = make_unsigned_command();
        let record = ProtocolRecord::CommandEnvelope(cmd.clone());
        let a = canonical_bytes(&record, true);
        let b = canonical_bytes(&record, true);
        assert_eq!(a, b);
    }

    #[test]
    fn canonical_command_signable_vs_full() {
        let mut cmd = make_unsigned_command();
        cmd.signature = Some(ProtoSignature {
            algorithm: 2,
            value: vec![1; 64],
        });
        let record = ProtocolRecord::CommandEnvelope(cmd.clone());
        let signable = canonical_bytes(&record, true);
        let full = canonical_bytes(&record, false);
        assert_ne!(signable, full);
        assert!(signable.len() < full.len());
    }
}
