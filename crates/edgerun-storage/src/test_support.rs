use crate::prelude::v1::Vec;
use edgerun_hardware_signing::{HardwareSigningError, MeshSigner, NodeID};
use edgerun_sign::{ProtocolSignError, ProtocolSigner};
use edgerun_verify::ProtocolFamily;

#[derive(Clone)]
pub(crate) struct TestSigner {
    node_id: NodeID,
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
            node_id: NodeID(node_bytes),
            key,
        }
    }
}

impl MeshSigner for TestSigner {
    fn node_id(&self) -> NodeID {
        self.node_id
    }

    fn sign_digest(&self, digest: &[u8; 32]) -> Result<[u8; 64], HardwareSigningError> {
        use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashSigner;

        let sig: edgerun_crypto::p256::ecdsa::Signature = self.key.sign_prehash(digest).unwrap();
        let mut bytes = [0u8; 64];
        bytes.copy_from_slice(&sig.to_bytes());
        Ok(bytes)
    }

    fn sign_message_var(&self, message: &[u8]) -> Result<[u8; 64], HardwareSigningError> {
        use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashSigner;

        let sig: edgerun_crypto::p256::ecdsa::Signature = self.key.sign_prehash(message).unwrap();
        let mut bytes = [0u8; 64];
        bytes.copy_from_slice(&sig.to_bytes());
        Ok(bytes)
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
