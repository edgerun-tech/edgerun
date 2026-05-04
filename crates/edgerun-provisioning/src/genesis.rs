use super::contract::ProvisioningContract;
use edgerun_core::{EventId, Identity, Signature, StreamId, Timestamp};
use edgerun_crypto::{sign, KeyPair};

/// Node genesis claim
/// Sent by node to controller via bootstrap coordinator
#[derive(Debug, Clone)]
pub struct NodeGenesisClaim {
    pub version: String,
    pub provisioning_id: String,
    pub provisioning_contract_hash: String,
    pub node_identity: Identity,
    pub controller: Identity,
    pub node_stream_id: StreamId,
    pub node_genesis_event_hash: String,
    pub node_label: String,
    pub build_artifact_hash: String,
    pub config_hash: String,
    pub boot_measurement_hash: Option<String>,
    pub created_at: Timestamp,
    pub node_signature: Option<Signature>,
}

impl NodeGenesisClaim {
    /// Create a new genesis claim
    pub fn new(
        contract: &ProvisioningContract,
        node_identity: Identity,
        node_stream_id: StreamId,
        genesis_event_hash: String,
    ) -> Self {
        Self {
            version: contract.version.clone(),
            provisioning_id: contract.provisioning_id.clone(),
            provisioning_contract_hash: contract.compute_hash(),
            node_identity,
            controller: contract.controller.clone(),
            node_stream_id,
            node_genesis_event_hash: genesis_event_hash,
            node_label: contract.node_label.clone(),
            build_artifact_hash: contract.build_artifact_hash.clone(),
            config_hash: contract.config_hash.clone(),
            boot_measurement_hash: None,
            created_at: Timestamp::now(),
            node_signature: None,
        }
    }

    /// Sign this claim with the node's keypair
    pub fn sign(&mut self, keypair: &KeyPair) -> Result<(), String> {
        let payload = self.canonical_payload();
        match sign(keypair, payload.as_bytes()) {
            Ok(sig) => {
                self.node_signature = Some(sig);
                Ok(())
            }
            Err(e) => Err(format!("Failed to sign genesis claim: {:?}", e)),
        }
    }

    /// Verify the node signature on this claim
    pub fn verify_signature(&self, node_public_key: &edgerun_crypto::PublicKey) -> bool {
        let Some(ref sig) = self.node_signature else {
            return false;
        };

        let payload = self.canonical_payload();
        edgerun_crypto::verify(node_public_key, payload.as_bytes(), sig)
    }

    /// Build canonical payload for signing/verification
    fn canonical_payload(&self) -> String {
        format!(
            "version={};provisioning_id={};node_identity={};contract_hash={};genesis_hash={}",
            self.version,
            self.provisioning_id,
            self.node_identity.to_string(),
            self.provisioning_contract_hash,
            self.node_genesis_event_hash,
        )
    }

    /// Check if this claim commits to the given contract hash
    pub fn commits_to_contract(&self, contract_hash: &str) -> bool {
        self.provisioning_contract_hash == contract_hash
    }
}
