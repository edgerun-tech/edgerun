use super::contract::ProvisioningContract;
use super::errors::ProvisioningError;
use super::genesis::NodeGenesisClaim;
type PublicKey = Vec<u8>;

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
        if !claim.commits_to_contract(&crate::contract::hex_encode(&contract.compute_hash())) {
            return Err(ProvisioningError::GenesisNotCommittedToContract);
        }

        // Verify node signature
        if !claim.verify_signature(node_public_key) {
            return Err(ProvisioningError::GenesisSignatureInvalid);
        }

        // Check build hash
        // (build_artifact_hash and config_hash not in generated type yet)

        Ok(())
    }
}
