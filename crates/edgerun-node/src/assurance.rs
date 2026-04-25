//! Assurance claim generation and recording.
//!
//! When a command is executed with an assurance requirement, this module generates
//! an `AssuranceClaim` that proves the command was executed at the requested
//! assurance level (software, hardware-backed, or attested runtime).

use edgerun_core::protocol::{canonical_bytes, ProtocolRecord};
use edgerun_core::util::system_time_to_prost;
use edgerun_hardware_signing::MeshSigner;
use edgerun_proto::edgerun::v0::common::ObjectKind;
use edgerun_proto::edgerun::v0::trust::AssuranceClaim;
use edgerun_storage::NodeStore;
use std::time::SystemTime;

const KIND_PROOF: i32 = ObjectKind::Proof as i32; // 7

/// Generates and records an AssuranceClaim for the given command execution.
///
/// The claim is signed by the local node's key using domain-separated
/// canonical signing (spec §17) and stored as an encrypted object.
/// Returns the ObjectRef to the stored claim.
pub fn generate_and_record_assurance_claim(
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    command: &edgerun_proto::edgerun::v0::stream::CommandEnvelope,
    assurance_class: i32,
) -> Option<edgerun_proto::edgerun::v0::common::ObjectRef> {
    use edgerun_proto::edgerun::v0::common::{AssuranceClass, IdentityRef};

    let node_id = signer.node_id();
    let now = SystemTime::now();
    let expires = now.checked_add(std::time::Duration::from_secs(3600))?; // 1 hour validity

    let claim = AssuranceClaim {
        claim_version: 1,
        subject: Some(
            edgerun_proto::edgerun::v0::trust::assurance_claim::Subject::SubjectNode(
                edgerun_proto::edgerun::v0::common::NodeRef {
                    node_id: node_id.0.to_vec(),
                },
            ),
        ),
        assurance_class,
        attester: Some(IdentityRef {
            identity_id: node_id.0.to_vec(),
            identity_kind: Some(2), // NODE
            key_hint: None,
        }),
        issued_at: Some(system_time_to_prost(now)),
        expires_at: Some(system_time_to_prost(expires)),
        evidence_object: None,
        claim_note: format!(
            "Command {} executed at assurance class {}",
            edgerun_core::util::bytes_to_hex(&command.command_id),
            assurance_class
        ),
        signature: None,
    };

    // Sign using domain-separated canonical path (spec §17)
    let record = ProtocolRecord::AssuranceClaim(claim.clone());
    let canonical = canonical_bytes(&record, true);
    let sig_bytes =
        match signer.sign_record(edgerun_core::crypto::SIG_DOMAIN_ASSURANCE_CLAIM, &canonical) {
            Ok(sig) => sig,
            Err(e) => {
                edgerun_log::warn!("failed to sign assurance claim: {:?}", e);
                return None;
            }
        };

    let mut signed_claim = claim;
    signed_claim.signature = Some(edgerun_proto::edgerun::v0::common::Signature {
        algorithm: 1,
        value: sig_bytes.to_vec(),
    });

    let claim_bytes = prost::Message::encode_to_vec(&signed_claim);
    let obj_ref = store
        .put_object(&claim_bytes, KIND_PROOF, &[stream_id.to_vec()])
        .ok()?;

    edgerun_log::info!(
        "assurance claim recorded: class={}, object_id={}",
        assurance_class,
        edgerun_core::util::bytes_to_hex(&obj_ref.object_id)
    );

    Some(obj_ref)
}

/// Verifies an AssuranceClaim's signature using domain-separated canonical
/// verification (spec §17).
pub fn verify_assurance_claim(claim: &AssuranceClaim) -> Result<(), &'static str> {
    let Some(ref sig) = claim.signature else {
        return Err("missing_signature");
    };
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

    let mut vk_sec1 = [0u8; 65];
    vk_sec1[0] = 0x04;
    vk_sec1[1..].copy_from_slice(key_hint);
    let vk = match edgerun_crypto::p256::ecdsa::VerifyingKey::from_sec1_bytes(&vk_sec1) {
        Ok(v) => v,
        Err(_) => return Err("bad_public_key"),
    };

    let record = ProtocolRecord::AssuranceClaim(claim.clone());
    let canonical = canonical_bytes(&record, true);

    if !edgerun_core::crypto::verify_canonical_record(
        &vk,
        edgerun_core::crypto::SIG_DOMAIN_ASSURANCE_CLAIM,
        &canonical,
        &sig.value,
    ) {
        return Err("invalid_signature");
    }

    Ok(())
}
