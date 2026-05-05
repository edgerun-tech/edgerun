use crate::prelude::v1::Vec;
use edgerun_keygen::generate_ephemeral_node_identity;
use edgerun_sign::{ProtocolSignError, ProtocolSigner};
use edgerun_sign_p256::P256ProtocolSigner;
use edgerun_verify::ProtocolFamily;

#[derive(Clone)]
pub(crate) struct TestSigner {
    node_id: [u8; 64],
    signer: P256ProtocolSigner,
}

impl TestSigner {
    pub(crate) fn new() -> Self {
        let identity = generate_ephemeral_node_identity();
        Self {
            node_id: identity.node_id,
            signer: identity.signer,
        }
    }

    pub(crate) fn node_id(&self) -> [u8; 64] {
        self.node_id
    }
}

impl ProtocolSigner for TestSigner {
    fn signature_algorithm(&self) -> i32 {
        self.signer.signature_algorithm()
    }

    fn sign_signature_input(
        &self,
        family: ProtocolFamily,
        signature_input: &[u8],
    ) -> Result<Vec<u8>, ProtocolSignError> {
        self.signer.sign_signature_input(family, signature_input)
    }
}
