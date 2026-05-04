//! Assurance claim generation and recording.
//!
//! When a command is executed with an assurance requirement, this module generates
//! an `AssuranceClaim` that proves the command was executed at the requested
//! assurance level (software, hardware-backed, or attested runtime).

use edgerun_core::protocol::{canonical_bytes, ProtocolRecord};
use edgerun_core::util::{now_unix_millis_i64, system_time_to_prost};
use edgerun_hardware_signing::MeshSigner;
use edgerun_core::protocol::ObjectKind;
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
    command: &edgerun_core::protocol::CommandEnvelope,
    assurance_class: i32,
) -> Option<edgerun_core::protocol::ObjectRef> {
    use edgerun_core::protocol::{AssuranceClass, IdentityRef};

    let node_id = signer.node_id();
    let now = SystemTime::now();
    let expires = now.checked_add(std::time::Duration::from_secs(3600))?; // 1 hour validity

    let claim = AssuranceClaim {
        claim_version: 1,
        subject: Some(
            edgerun_proto::edgerun::v0::trust::assurance_claim::Subject::SubjectNode(
                edgerun_core::protocol::NodeRef {
                    node_id: node_id.0.to_vec(),
                },
            ),
        ),
        assurance_class,
        attester: Some(IdentityRef {
            identity_id: node_id.0.to_vec(),
            identity_kind: Some(2), // NODE
            key_hint: Some(node_id.0.to_vec()),
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
    signed_claim.signature = Some(edgerun_core::protocol::Signature {
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
    let Some(ref attester) = claim.attester else {
        return Err("no_attester");
    };
    let now_ms = now_unix_millis_i64();
    let validation = edgerun_core::validators_proto::validate_assurance_claim(claim, now_ms, &[]);
    if validation.verdict != edgerun_core::result::Verdict::Accept {
        return Err(match validation.reason_code {
            Some(edgerun_core::result::ReasonCode::CryptoInvalid) => "invalid_signature",
            Some(edgerun_core::result::ReasonCode::TimeInvalid) => "time_invalid",
            Some(edgerun_core::result::ReasonCode::AuthorityDenied) => "authority_denied",
            _ if attester.identity_id.is_empty() => "no_attester",
            _ => "invalid_assurance_claim",
        });
    }

    Ok(())
}
