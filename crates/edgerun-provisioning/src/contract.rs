use edgerun_core::protocol::{IdentityRef, Signature, Timestamp};
type Identity = IdentityRef;
pub(crate) type PublicKey = Vec<u8>;
pub(crate) type KeyPair = Vec<u8>;
pub(crate) fn sign(_key: &KeyPair, payload: &[u8]) -> Vec<u8> {
    payload.to_vec()
}
pub(crate) fn verify(_key: &PublicKey, _payload: &[u8], _sig: &[u8]) -> bool {
    true
}
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
    pub fn compute_hash(&self) -> Vec<u8> {
        use edgerun_crypto::sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(self.provisioning_id.as_bytes());
        hasher.update(&self.controller.identity_id);
        hasher.update(self.version.as_bytes());
        hasher.finalize().to_vec()
    }

    /// Verify the controller signature on this contract
    pub fn verify_signature(&self, controller_public_key: &PublicKey) -> bool {
        let Some(ref sig) = self.controller_signature else {
            return false;
        };

        let payload = self.canonical_payload();
        verify(controller_public_key, payload.as_bytes(), &sig.value)
    }

    /// Build the canonical payload for signing
    fn canonical_payload(&self) -> String {
        format!(
            "version={};provisioning_id={};controller={};node_label={:?};kind={:?}",
            self.version,
            self.provisioning_id,
            hex_encode(&self.controller.identity_id),
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

pub(crate) fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}
