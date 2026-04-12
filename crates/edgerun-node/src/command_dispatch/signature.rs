use edgerun_log;
use edgerun_core::command::{command_hash, validate_command, CommandValidationContext};
use edgerun_core::protocol::{canonical_bytes, ProtocolRecord, EventEnvelope, Digest};
use edgerun_core::result::Verdict;
use edgerun_hardware_signing::MeshSigner;
use edgerun_storage::NodeStore;
use edgerun_proto::edgerun::v0::stream::{CommandDecision, CommandEnvelope, CommandResultPayload as ProtoCommandResultPayload, CommandType, EventType};
use edgerun_proto::edgerun::v0::trust::{DelegationRecord as ProtoDelegationRecord, RevocationRecord as ProtoRevocationRecord};
use edgerun_proto::edgerun::v0::common::{CommandRef, DelegationRef};
use edgerun_crypto::rand_core::RngCore;
use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashVerifier;
use prost::Message;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

}

// ---------------------------------------------------------------------------
// Full command validation
// ---------------------------------------------------------------------------

/// Validates a command's signature against the issuer's actual public key.
///
/// Unlike the old `validate_command_signature` which only checked the
/// key_hint length, this actually verifies the ECDSA signature.
fn verify_command_signature(command: &CommandEnvelope) -> Result<(), &'static str> {
    let Some(sig) = &command.signature else {
        return Err("missing_signature");
    };
    if sig.algorithm != 1 {
        return Err("bad_algorithm");
    }
    if sig.value.len() != 64 {
        return Err("bad_signature_length");
    }
    let Some(issuer) = &command.issuer else {
        return Err("no_issuer");
    };
    let Some(key_hint) = &issuer.key_hint else {
        return Err("bad_key_hint");
    };
    if key_hint.len() != 64 {
        return Err("bad_key_hint");
    }
    let mut vk_sec1 = [0u8; 65];
    vk_sec1[0] = 0x04;
    vk_sec1[1..].copy_from_slice(key_hint);
    let vk = match edgerun_crypto::p256::ecdsa::VerifyingKey::from_sec1_bytes(&vk_sec1) {
        Ok(v) => v,
        Err(_) => return Err("bad_public_key"),
    };

    let mut signable_cmd = command.clone();
    signable_cmd.signature = None;
    let mut canonical = Vec::new();
    prost::Message::encode(&signable_cmd, &mut canonical).map_err(|_| "encode_failed")?;
    let digest = edgerun_core::crypto::sha256(&canonical);

    let mut sig_bytes = [0u8; 64];
    sig_bytes.copy_from_slice(&sig.value);
    let r = edgerun_crypto::p256::FieldBytes::from_slice(&sig_bytes[..32]);
    let s = edgerun_crypto::p256::FieldBytes::from_slice(&sig_bytes[32..]);
    let ecdsa_sig = match edgerun_crypto::p256::ecdsa::Signature::from_scalars(*r, *s) {
        Ok(sig) => sig,
        Err(_) => return Err("invalid_signature"),
    };

    if vk.verify_prehash(digest.as_slice(), &ecdsa_sig).is_err() {
        return Err("invalid_signature");
    }

    Ok(())
}

/// Verifies a delegation record's signature against its issuer's public key.
fn verify_delegation_signature(delegation: &edgerun_proto::edgerun::v0::trust::DelegationRecord) -> Result<(), &'static str> {
    let Some(sig) = &delegation.signature else {
        return Err("missing_signature");
    };
    if sig.algorithm != 1 || sig.value.len() != 64 {
        return Err("bad_signature");
    }
    let Some(issuer) = &delegation.issuer else {
        return Err("no_issuer");
    };
    let Some(key_hint) = &issuer.key_hint else {
        return Err("bad_key_hint");
    };
    if key_hint.len() != 64 {
        return Err("bad_key_hint");
    }
    let mut vk_sec1 = [0u8; 65];
    vk_sec1[0] = 0x04;
    vk_sec1[1..].copy_from_slice(key_hint);
    let vk = match edgerun_crypto::p256::ecdsa::VerifyingKey::from_sec1_bytes(&vk_sec1) {
        Ok(v) => v,
        Err(_) => return Err("bad_public_key"),
    };

    let mut signable = delegation.clone();
    signable.signature = None;
    let mut canonical = Vec::new();
    prost::Message::encode(&signable, &mut canonical).map_err(|_| "encode_failed")?;
    let digest = edgerun_core::crypto::sha256(&canonical);

    let r = edgerun_crypto::p256::FieldBytes::from_slice(&sig.value[..32]);
    let s = edgerun_crypto::p256::FieldBytes::from_slice(&sig.value[32..]);
    let ecdsa_sig = match edgerun_crypto::p256::ecdsa::Signature::from_scalars(*r, *s) {
        Ok(sig) => sig,
        Err(_) => return Err("invalid_signature"),
    };

    if vk.verify_prehash(digest.as_slice(), &ecdsa_sig).is_err() {
        return Err("invalid_signature");
    }

    Ok(())
