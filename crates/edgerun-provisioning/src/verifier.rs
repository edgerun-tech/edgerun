use super::contract::ProvisioningContract;
use super::errors::ProvisioningError;
use super::genesis::NodeGenesisClaim;
use edgerun_crypto::PublicKey;

/// Verifier for provisioning contracts and genesis claims
pub struct ProvisioningVerifier;

impl ProvisioningVerifier {
    /// Verify a provisioning contract
    /// Returns Ok(()) if valid, Err(ProvisioningError) otherwise
    pub fn verify_contract(
        contract: &ProvisioningContract,
        controller_public_key: &PublicKey,
    ) -> Result<(), ProvisioningError> {
        // Check kind (enum, not bool)
        match contract.kind {
            super::contract::ProvisioningKind::Unspecified => {
                return Err(ProvisioningError::UnspecifiedKind);
            }
            super::contract::ProvisioningKind::Template => {
                // Templates not supported yet
                return Err(ProvisioningError::ContractKindInvalid);
            }
            super::contract::ProvisioningKind::SingleNode => {
                // OK
            }
        }

        // Check expiry
        if contract.is_expired() {
            return Err(ProvisioningError::ContractExpired);
        }

        // Verify controller signature
        if !contract.verify_signature(controller_public_key) {
            return Err(ProvisioningError::ContractSignatureInvalid);
        }

        Ok(())
    }

    /// Verify a node genesis claim against a contract
    pub fn verify_genesis_claim(
        contract: &ProvisioningContract,
        claim: &NodeGenesisClaim,
        node_public_key: &PublicKey,
    ) -> Result<(), ProvisioningError> {
        // Check that claim commits to this contract
        if !claim.commits_to_contract(&contract.compute_hash()) {
            return Err(ProvisioningError::GenesisNotCommittedToContract);
        }

        // Verify node signature
        if !claim.verify_signature(node_public_key) {
            return Err(ProvisioningError::GenesisSignatureInvalid);
        }

        // Check build hash
        if !claim.build_artifact_hash.is_empty()
            && !contract.build_artifact_hash.is_empty()
            && claim.build_artifact_hash != contract.build_artifact_hash
        {
            return Err(ProvisioningError::BuildHashMismatch);
        }

        // Check config hash
        if !claim.config_hash.is_empty()
            && !contract.config_hash.is_empty()
            && claim.config_hash != contract.config_hash
        {
            return Err(ProvisioningError::ConfigHashMismatch);
        }

        Ok(())
    }
}
