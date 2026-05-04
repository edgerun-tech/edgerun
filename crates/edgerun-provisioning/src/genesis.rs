use edgerun_core::protocol::{IdentityRef, Signature, Timestamp};

type Identity = IdentityRef;
use crate::contract::{hex_encode, sign, verify, KeyPair, ProvisioningContract, PublicKey};

#[derive(Debug, Clone)]
pub struct NodeGenesisClaim {
    pub node_identity: Identity,
    pub provisioning_contract_hash: String,
    pub created_at: Timestamp,
    pub signature: Option<Signature>,
}

impl NodeGenesisClaim {
    pub fn new(node_identity: Identity, contract: &ProvisioningContract) -> Self {
        Self {
            node_identity,
            provisioning_contract_hash: hex_encode(&contract.compute_hash()),
            created_at: Timestamp {
                seconds: 0,
                nanos: 0,
            },
            signature: None,
        }
    }

    pub fn sign(&mut self, keypair: &KeyPair) -> Result<Signature, String> {
        let payload = self.signing_payload();
        let sig = Signature {
            algorithm: 1,
            value: sign(keypair, payload.as_bytes()),
        };
        self.signature = Some(sig.clone());
        Ok(sig)
    }

    pub fn verify_signature(&self, node_public_key: &PublicKey) -> bool {
        let Some(sig) = self.signature.as_ref() else {
            return false;
        };
        let payload = self.signing_payload();
        verify(node_public_key, payload.as_bytes(), &sig.value)
    }

    pub fn commits_to_contract(&self, contract_hash: &str) -> bool {
        self.provisioning_contract_hash == contract_hash
    }

    fn signing_payload(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            hex_encode(&self.node_identity.identity_id),
            self.provisioning_contract_hash,
            self.created_at.seconds,
            self.created_at.nanos
        )
    }
}
