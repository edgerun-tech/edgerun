use edgerun_core::{Identity, StreamId, EventId, Signature, Timestamp};
use edgerun_crypto::{sign, verify, KeyPair, PublicKey};
use std::time::{SystemTime, UNIX_EPOCH};

/// Provisioning contract kind — enum, not bool
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvisioningKind {
    Unspecified,  // Rejected
    SingleNode,   // Normal path
    Template,     // Reserved — not supported yet
}

impl ProvisioningKind {
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => ProvisioningKind::SingleNode,
            2 => ProvisioningKind::Template,
            _ => ProvisioningKind::Unspecified,
        }
    }
}

/// Provisioning contract status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvisioningContractStatus {
    Unspecified,
    Issued,
    BakedIntoArtifact,
    FirstBootClaimReceived,
    Accepted,
    Rejected,
    Consumed,
    ReplayRejected,
    Revoked,
}

/// Provisioning contract
/// Single-use by default (contains node-specific settings)
#[derive(Debug, Clone)]
pub struct ProvisioningContract {
    pub version: String,
    pub provisioning_id: String,
    pub controller: Identity,
    pub node_label: String,
    pub node_role: String,
    pub scheduler_tags: Vec<String>,
    pub bootstrap_coordinator_url: String,
    pub network_profile_id: String,
    pub resource_advertisement_policy: String,
    pub required_assurance: String,
    pub build_artifact_hash: String,
    pub config_hash: String,
    pub issued_at: Timestamp,
    pub expires_at: Timestamp,
    pub kind: ProvisioningKind,
    pub controller_signature: Option<Signature>,
}

impl ProvisioningContract {
    /// Compute the hash of this contract (for commitment in genesis)
    pub fn compute_hash(&self) -> String {
        // TODO: Use proper canonical serialization + hash
        // For now, use a simple approach
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut s = DefaultHasher::new();
        self.provisioning_id.hash(&mut s);
        self.controller.to_string().hash(&mut s);
        self.version.hash(&mut s);
        format!("{:x}", s.finish())
    }
    
    /// Verify the controller signature on this contract
    pub fn verify_signature(&self, controller_public_key: &PublicKey) -> bool {
        let Some(ref sig) = self.controller_signature else {
            return false;
        };
        
        // TODO: Use proper signature verification with canonical payload
        verify(
            controller_public_key,
            self.canonical_payload().as_bytes(),
            sig,
        )
    }
    
    /// Build the canonical payload for signing
    fn canonical_payload(&self) -> String {
        format!(
            "version={};provisioning_id={};controller={};node_label={};kind={:?}",
            self.version,
            self.provisioning_id,
            self.controller.to_string(),
            self.node_label,
            self.kind,
        )
    }
    
    /// Check if this contract has expired
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now > self.expires_at.seconds as u64
    }
    
    /// Check if this contract is single-use (not template)
    pub fn is_single_use(&self) -> bool {
        matches!(self.kind, ProvisioningKind::SingleNode)
    }
}
