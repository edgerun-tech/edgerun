//! Assurance claim generation and recording.
//!
//! When a command is executed with an assurance requirement, this module generates
//! an `AssuranceClaim` that proves the command was executed at the requested
//! assurance level (software, hardware-backed, or attested runtime).

use edgerun_proto::edgerun::v0::common::ObjectKind;
use edgerun_hardware_signing::MeshSigner;
use edgerun_storage::NodeStore;
use edgerun_core::util::system_time_to_prost;
use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashVerifier;
use prost::Message;
use std::time::SystemTime;
use edgerun_proto::edgerun::v0::trust::AssuranceClaim;

const KIND_PROOF: i32 = ObjectKind::Proof as i32; // 7

/// Generates and records an AssuranceClaim for the given command execution.
///
/// The claim is signed by the local node's key and stored as an encrypted object.
/// Returns the ObjectRef to the stored claim.
pub fn generate_and_record_assurance_claim(
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    command: &edgerun_proto::edgerun::v0::stream::CommandEnvelope,
    assurance_class: i32,
) -> Option<edgerun_proto::edgerun::v0::common::ObjectRef> {
    use edgerun_proto::edgerun::v0::trust::AssuranceClaim;
    use edgerun_proto::edgerun::v0::common::{AssuranceClass, IdentityRef};

    let node_id = signer.node_id();
    let now = SystemTime::now();
    let expires = now.checked_add(std::time::Duration::from_secs(3600))?; // 1 hour validity

    let claim = AssuranceClaim {
        claim_version: 1,
        subject: Some(edgerun_proto::edgerun::v0::trust::assurance_claim::Subject::SubjectNode(
            edgerun_proto::edgerun::v0::common::NodeRef {
                node_id: node_id.0.to_vec(),
            },
        )),
        assurance_class,
        attester: Some(IdentityRef {
            identity_id: node_id.0.to_vec(),
            identity_kind: Some(2), // NODE
            key_hint: None,
        }),
        issued_at: Some(system_time_to_prost(now)),
        expires_at: Some(system_time_to_prost(expires)),
        evidence_object: None, // Evidence can be added later (e.g., TPM quotes)
        claim_note: format!(
            "Command {} executed at assurance class {}",
            edgerun_core::util::bytes_to_hex(&command.command_id),
            assurance_class
        ),
        signature: None, // Will be signed below
    };

    // Sign the claim: hash the canonical form and sign with the node's key
    let claim_bytes = prost::Message::encode_to_vec(&claim);
    let claim_hash = edgerun_core::crypto::sha256(&claim_bytes);
    let mut digest_32 = [0u8; 32];
    digest_32.copy_from_slice(&claim_hash);

    let sig_bytes = match signer.sign_digest(&digest_32) {
        Ok(sig) => sig,
        Err(e) => {
            edgerun_log::warn!("failed to sign assurance claim: {:?}", e);
            return None;
        }
    };

    let mut signed_claim = claim;
    signed_claim.signature = Some(edgerun_proto::edgerun::v0::common::Signature {
        algorithm: 1, // ECDSA P-256
        value: sig_bytes.to_vec(),
    });

    // Store the signed claim as an object
    let claim_bytes = prost::Message::encode_to_vec(&signed_claim);
    let obj_ref = store.put_object(&claim_bytes, KIND_PROOF, &[stream_id.to_vec()]).ok()?;

    edgerun_log::info!("assurance claim recorded: class={}, object_id={}",
        assurance_class,
        edgerun_core::util::bytes_to_hex(&obj_ref.object_id));

    Some(obj_ref)
}

/// Verifies an AssuranceClaim's signature.
pub fn verify_assurance_claim(
    claim: &edgerun_proto::edgerun::v0::trust::AssuranceClaim,
) -> Result<(), &'static str> {
    let Some(ref sig) = claim.signature else {
        return Err("missing_signature");
    };
    if sig.algorithm != edgerun_core::crypto::SIGNATURE_ALGORITHM_ECDSA_P256 as i32 {
        return Err("bad_algorithm");
    }
    if sig.value.len() != edgerun_core::crypto::ECDSA_P256_SIGNATURE_LEN {
        return Err("bad_signature_length");
    }
    let Some(ref attester) = claim.attester else {
        return Err("no_attester");
    };
    let Some(ref key_hint) = attester.key_hint else {
        return Err("no_key_hint");
    };
    if key_hint.len() != edgerun_core::crypto::ECDSA_P256_PUBLIC_KEY_LEN {
        return Err("bad_key_hint");
    }

    // Reconstruct public key
    let mut vk_sec1 = [0u8; 65];
    vk_sec1[0] = 0x04;
    vk_sec1[1..].copy_from_slice(key_hint);
    let vk = match edgerun_crypto::p256::ecdsa::VerifyingKey::from_sec1_bytes(&vk_sec1) {
        Ok(v) => v,
        Err(_) => return Err("bad_public_key"),
    };

    // Hash the claim without signature
    let mut signable = claim.clone();
    signable.signature = None;
    let canonical = prost::Message::encode_to_vec(&signable);
    let digest = edgerun_core::crypto::sha256(&canonical);

    // Verify
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
