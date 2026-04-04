use crate::cbor::cbor_dumps;
use crate::crypto::domain_hash;
use crate::protocol::ProtocolRecord;

pub fn canonicalize_typed_record(
    record: &ProtocolRecord,
    signable: bool,
) -> Result<Vec<u8>, String> {
    cbor_dumps(&record.to_cvalue(signable))
}

pub fn record_hash_typed(record: &ProtocolRecord, domain_tag: &str) -> Result<Vec<u8>, String> {
    Ok(domain_hash(
        domain_tag,
        &canonicalize_typed_record(record, true)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{CommandEnvelope, IdentityRef, NodeRef, ProtocolRecord, Signature};

    fn sample_command(with_signature: bool) -> ProtocolRecord {
        ProtocolRecord::CommandEnvelope(CommandEnvelope {
            envelope_version: 1,
            command_id: Some(vec![1, 2, 3]),
            target_node: NodeRef { node_id: vec![4] },
            issuer: IdentityRef {
                identity_id: vec![5],
                identity_kind: Some("user".into()),
                key_hint: None,
            },
            command_type: Some("COMMAND_TYPE_QUERY".into()),
            command_version: 1,
            issued_at: None,
            not_before: None,
            expires_at: None,
            idempotency_key: None,
            payload_object: None,
            inline_payload: None,
            delegation_chain: vec![],
            requested_assurance: None,
            command_metadata: None,
            signature: with_signature.then(|| Signature {
                algorithm: "ed25519".into(),
                value: vec![9; 64],
            }),
        })
    }

    #[test]
    fn signable_canonicalization_ignores_signature_field() {
        let unsigned = sample_command(false);
        let signed = sample_command(true);
        let a = canonicalize_typed_record(&unsigned, true).unwrap();
        let b = canonicalize_typed_record(&signed, true).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn full_canonicalization_includes_signature_field() {
        let unsigned = sample_command(false);
        let signed = sample_command(true);
        let a = canonicalize_typed_record(&unsigned, false).unwrap();
        let b = canonicalize_typed_record(&signed, false).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn record_hash_uses_signable_form() {
        let unsigned = sample_command(false);
        let signed = sample_command(true);
        let a = record_hash_typed(&unsigned, "lifegraph:v0:hash:command-envelope").unwrap();
        let b = record_hash_typed(&signed, "lifegraph:v0:hash:command-envelope").unwrap();
        assert_eq!(a, b);
    }
}
