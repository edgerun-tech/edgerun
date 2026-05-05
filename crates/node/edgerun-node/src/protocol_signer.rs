//! Node-edge adapters between legacy mesh signers and protocol signing.

use alloc::sync::Arc;
use alloc::vec::Vec;

use edgerun_hardware_signing::{MeshSigner, NodeID};
use edgerun_sign::{ProtocolSignError, ProtocolSigner, SignableProtocolFamily as ProtocolFamily};

/// Adapter kept at the node edge while mesh transport still owns `MeshSigner`.
#[derive(Clone)]
pub struct MeshProtocolSigner {
    signer: Arc<dyn MeshSigner>,
}

impl MeshProtocolSigner {
    pub fn new(signer: Arc<dyn MeshSigner>) -> Self {
        Self { signer }
    }

    pub fn node_id(&self) -> NodeID {
        self.signer.node_id()
    }

    pub fn mesh_signer(&self) -> &dyn MeshSigner {
        self.signer.as_ref()
    }
}

impl ProtocolSigner for MeshProtocolSigner {
    fn signature_algorithm(&self) -> i32 {
        edgerun_core::crypto::SIGNATURE_ALGORITHM_ECDSA_P256 as i32
    }

    fn sign_signature_input(
        &self,
        _family: ProtocolFamily,
        signature_input: &[u8],
    ) -> Result<Vec<u8>, ProtocolSignError> {
        self.signer
            .sign_message_var(signature_input)
            .map(|signature| signature.to_vec())
            .map_err(|_| ProtocolSignError::SignerFailed)
    }
}

pub struct BorrowedMeshProtocolSigner<'a> {
    signer: &'a dyn MeshSigner,
}

impl<'a> BorrowedMeshProtocolSigner<'a> {
    pub fn new(signer: &'a dyn MeshSigner) -> Self {
        Self { signer }
    }
}

impl ProtocolSigner for BorrowedMeshProtocolSigner<'_> {
    fn signature_algorithm(&self) -> i32 {
        edgerun_core::crypto::SIGNATURE_ALGORITHM_ECDSA_P256 as i32
    }

    fn sign_signature_input(
        &self,
        _family: ProtocolFamily,
        signature_input: &[u8],
    ) -> Result<Vec<u8>, ProtocolSignError> {
        self.signer
            .sign_message_var(signature_input)
            .map(|signature| signature.to_vec())
            .map_err(|_| ProtocolSignError::SignerFailed)
    }
}
