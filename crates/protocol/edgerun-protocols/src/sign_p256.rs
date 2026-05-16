extern crate alloc;

use alloc::vec::Vec;

use crate::sign::{ProtocolSignError, ProtocolSigner, ProtocolSigningOutput};
use crate::verify::ProtocolFamily;
use edgerun_core::crypto::{self, SigningKey};
use edgerun_core::protocol::{ProtocolRecord, Signature};

#[derive(Clone)]
pub struct P256ProtocolSigner {
    signing_key: SigningKey,
}

impl P256ProtocolSigner {
    pub const fn new(signing_key: SigningKey) -> Self {
        Self { signing_key }
    }

    pub fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }

    pub fn verifying_key(&self) -> crypto::VerifyingKey {
        self.signing_key.verifying_key()
    }

    pub fn raw_public_key(&self) -> [u8; crypto::ECDSA_P256_PUBLIC_KEY_LEN] {
        crypto::verifying_key_to_node_id(&self.verifying_key())
    }

    pub fn sign_record(
        &self,
        record: &ProtocolRecord,
        family: ProtocolFamily,
    ) -> Result<ProtocolSigningOutput, ProtocolSignError> {
        <Self as ProtocolSigner>::sign_protocol_record(self, record, family)
    }
}

impl From<SigningKey> for P256ProtocolSigner {
    fn from(signing_key: SigningKey) -> Self {
        Self::new(signing_key)
    }
}

impl ProtocolSigner for P256ProtocolSigner {
    fn signature_algorithm(&self) -> i32 {
        crypto::SIGNATURE_ALGORITHM_ECDSA_P256 as i32
    }

    fn sign_signature_input(
        &self,
        _family: ProtocolFamily,
        signature_input: &[u8],
    ) -> Result<Vec<u8>, ProtocolSignError> {
        self.signing_key
            .sign_prehash_fixed(signature_input)
            .map(|signature| signature.to_vec())
            .map_err(|_| ProtocolSignError::SignerFailed)
    }
}

pub fn sign_protocol_record_p256(
    signing_key: &SigningKey,
    record: &ProtocolRecord,
    family: ProtocolFamily,
) -> Result<ProtocolSigningOutput, ProtocolSignError> {
    P256ProtocolSigner::new(signing_key.clone()).sign_record(record, family)
}

pub fn signature_for_record_p256(
    signing_key: &SigningKey,
    record: &ProtocolRecord,
    family: ProtocolFamily,
) -> Result<Signature, ProtocolSignError> {
    Ok(sign_protocol_record_p256(signing_key, record, family)?.signature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verify::{ProtocolSignerRef, verify_event_envelope};
    use edgerun_core::protocol::{EventEnvelope, ProtocolRecord};

    fn test_signing_key() -> SigningKey {
        let bytes: [u8; 32] = [9u8; 32];
        SigningKey::from_bytes(&bytes).unwrap()
    }

    #[test]
    fn signs_event_envelope_that_verifier_accepts() {
        let signing_key = test_signing_key();
        let signer = P256ProtocolSigner::new(signing_key);
        let writer = signer.raw_public_key();
        let mut event = EventEnvelope {
            envelope_version: 1,
            stream_id: writer.to_vec(),
            seq: 1,
            event_version: 1,
            ..EventEnvelope::default()
        };

        let signed = signer
            .sign_record(
                &ProtocolRecord::EventEnvelope(event.clone()),
                ProtocolFamily::EventEnvelope,
            )
            .unwrap();
        event.signature = Some(signed.signature);

        let verified =
            verify_event_envelope(&event, ProtocolSignerRef::P256Raw64(&writer)).unwrap();
        assert_eq!(verified.family, ProtocolFamily::EventEnvelope);
        assert_eq!(verified.record_hash, signed.record_hash);
    }
}
