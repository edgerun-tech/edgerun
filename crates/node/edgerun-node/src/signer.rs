use edgerun_hardware_signing::{MeshSigner, NodeID};
use edgerun_protocols::keygen::{
    NodeSigningKey, node_id_from_signing_key, node_signing_key_from_bytes,
};
use edgerun_protocols::sign::{ProtocolSigner, SignableProtocolFamily};
use edgerun_protocols::sign_p256::P256ProtocolSigner;

/// A software signer that is Send + Sync.
pub struct SyncSoftwareSigner {
    node_id: NodeID,
    signer: std::sync::Mutex<P256ProtocolSigner>,
}

impl SyncSoftwareSigner {
    pub fn new(key: NodeSigningKey) -> Self {
        let node_bytes = node_id_from_signing_key(&key);
        Self {
            node_id: NodeID(node_bytes),
            signer: std::sync::Mutex::new(P256ProtocolSigner::new(key)),
        }
    }
}

impl MeshSigner for SyncSoftwareSigner {
    fn node_id(&self) -> NodeID {
        self.node_id
    }

    fn sign_digest(
        &self,
        digest: &[u8; 32],
    ) -> Result<[u8; 64], edgerun_hardware_signing::HardwareSigningError> {
        let signer = self
            .signer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let sig = signer
            .sign_signature_input(SignableProtocolFamily::EventEnvelope, digest)
            .map_err(|e| {
                edgerun_hardware_signing::HardwareSigningError::Provider(format!("{e:?}"))
            })?;
        let mut bytes = [0u8; 64];
        bytes.copy_from_slice(&sig);
        Ok(bytes)
    }

    fn sign_message_var(
        &self,
        message: &[u8],
    ) -> Result<[u8; 64], edgerun_hardware_signing::HardwareSigningError> {
        let signer = self
            .signer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let sig = signer
            .sign_signature_input(SignableProtocolFamily::EventEnvelope, message)
            .map_err(|e| {
                edgerun_hardware_signing::HardwareSigningError::Provider(format!("{e:?}"))
            })?;
        let mut bytes = [0u8; 64];
        bytes.copy_from_slice(&sig);
        Ok(bytes)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub fn parse_signing_key_hex(key_hex: &str) -> NodeSigningKey {
    let hex_str = key_hex.trim();
    let bytes = edgerun_protocols::core_protocol::util::hex_to_bytes(hex_str).unwrap_or_else(|e| {
        eprintln!("error: invalid key hex: {}", e);
        std::process::exit(1);
    });
    let bytes: [u8; 32] = bytes.try_into().unwrap_or_else(|_| {
        eprintln!("error: key must be 32 bytes (64 hex chars)");
        std::process::exit(1);
    });
    node_signing_key_from_bytes(bytes).unwrap_or_else(|e| {
        eprintln!("error: invalid key: {:?}", e);
        std::process::exit(1);
    })
}
