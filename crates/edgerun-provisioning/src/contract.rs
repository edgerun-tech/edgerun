use edgerun_crypto::{sign, verify, KeyPair, PublicKey};
use edgerun_proto::edgerun::v0::common::{Identity, Signature, Timestamp};
use std::time::{SystemTime, UNIX_EPOCH};

/// Provisioning contract kind — enum, not bool
/// Mirrors proto enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvisioningKind {
    Unspecified, // Rejected
    SingleNode,  // Normal path
    Template,    // Reserved — not supported yet
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
/// Uses edgerun-proto types for Identity, Signature, Timestamp
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
    pub fn compute_hash(&self) -> Vec<u8> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(self.provisioning_id.as_bytes());
        hasher.update(&self.controller.fingerprint);
        hasher.update(self.version.as_bytes());
        hasher.finalize().to_vec()
    }

    /// Verify the controller signature on this contract
    pub fn verify_signature(&self, controller_public_key: &PublicKey) -> bool {
        let Some(ref sig) = self.controller_signature else {
            return false;
        };

        let payload = self.canonical_payload();
        verify(controller_public_key, payload.as_bytes(), sig)
    }

    /// Build the canonical payload for signing
    fn canonical_payload(&self) -> String {
        format!(
            "version={};provisioning_id={};controller={};node_label={:?};kind={:?}",
            self.version,
            self.provisioning_id,
            hex::encode(&self.controller.fingerprint),
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
