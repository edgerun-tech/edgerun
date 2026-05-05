use crate::prelude::v1::Vec;
use edgerun_sign::{ProtocolSignError, ProtocolSigner};
use edgerun_verify::ProtocolFamily;

#[derive(Clone)]
pub(crate) struct TestSigner {
    node_id: [u8; 64],
    key: edgerun_crypto::p256::ecdsa::SigningKey,
}

impl TestSigner {
    pub(crate) fn new() -> Self {
        let key = edgerun_crypto::random_p256_signing_key();
        let vk = key.verifying_key();
        let encoded = vk.to_encoded_point(false);
        let mut node_bytes = [0u8; 64];
        node_bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
        Self {
            node_id: node_bytes,
            key,
        }
    }

    pub(crate) fn node_id(&self) -> [u8; 64] {
        self.node_id
    }
}

impl ProtocolSigner for TestSigner {
    fn signature_algorithm(&self) -> i32 {
        edgerun_core::crypto::SIGNATURE_ALGORITHM_ECDSA_P256 as i32
    }

    fn sign_signature_input(
        &self,
        _family: ProtocolFamily,
        signature_input: &[u8],
    ) -> Result<Vec<u8>, ProtocolSignError> {
        use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashSigner;

        let sig: edgerun_crypto::p256::ecdsa::Signature = self
            .key
            .sign_prehash(signature_input)
            .map_err(|_| ProtocolSignError::SignerFailed)?;
        Ok(sig.to_bytes().to_vec())
    }
}
