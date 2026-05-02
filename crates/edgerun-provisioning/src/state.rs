use super::contract::ProvisioningContract;
use super::genesis::NodeGenesisClaim;
use super::errors::ProvisioningError;
use std::collections::HashMap;

/// State machine for provisioning contracts
/// Tracks consumed/revoked status to prevent replay
#[derive(Debug, Clone)]
pub struct ProvisioningState {
    // provisioning_id -> contract state
    contracts: HashMap<String, ContractState>,
}

#[derive(Debug, Clone)]
struct ContractState {
    pub contract: ProvisioningContract,
    pub status: super::contract::ProvisioningContractStatus,
    pub consumed_by: Option<NodeGenesisClaim>,
    pub consumed_at: Option<u64>,
}

impl ProvisioningState {
    pub fn new() -> Self {
        Self {
            contracts: HashMap::new(),
        }
    }
    
    /// Register a new provisioning contract (status: Issued)
    pub fn issue_contract(&mut self, contract: ProvisioningContract) {
        self.contracts.insert(
            contract.provisioning_id.clone(),
            ContractState {
                contract,
                status: super::contract::ProvisioningContractStatus::Issued,
                consumed_by: None,
                consumed_at: None,
            },
        );
    }
    
    /// Mark contract as baked into artifact
    pub fn mark_baked(&mut self, provisioning_id: &str) {
        if let Some(state) = self.contracts.get_mut(provisioning_id) {
            state.status = super::contract::ProvisioningContractStatus::BakedIntoArtifact;
        }
    }
    
    /// Process a node genesis claim
    /// Returns Ok(()) if accepted, Err(ProvisioningError) if rejected
    pub fn process_claim(
        &mut self,
        provisioning_id: &str,
        claim: NodeGenesisClaim,
    ) -> Result<(), ProvisioningError> {
        let state = self.contracts.get_mut(provisioning_id)
            .ok_or(ProvisioningError::ContractRevoked)?; // Treat missing as revoked
        
        match state.status {
            super::contract::ProvisioningContractStatus::Revoked => {
                return Err(ProvisioningError::ContractRevoked);
            }
            super::contract::ProvisioningContractStatus::Consumed
            | super::contract::ProvisioningContractStatus::Accepted
            | super::contract::ProvisioningContractStatus::ReplayRejected => {
                return Err(ProvisioningError::ContractAlreadyConsumed);
            }
            _ => {
                // OK to process
            }
        }
        
        // Mark as consumed
        state.status = super::contract::ProvisioningContractStatus::Consumed;
        state.consumed_by = Some(claim);
        state.consumed_at = Some(std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs());
        
        Ok(())
    }
    
    /// Accept a provisioning (after controller verification)
    pub fn accept_provisioning(&mut self, provisioning_id: &str) {
        if let Some(state) = self.contracts.get_mut(provisioning_id) {
            state.status = super::contract::ProvisioningContractStatus::Accepted;
        }
    }
    
    /// Reject a provisioning
    pub fn reject_provisioning(&mut self, provisioning_id: &str) {
        if let Some(state) = self.contracts.get_mut(provisioning_id) {
            state.status = super::contract::ProvisioningContractStatus::Rejected;
        }
    }
    
    /// Revoke a provisioning contract
    pub fn revoke_contract(&mut self, provisioning_id: &str) {
        if let Some(state) = self.contracts.get_mut(provisioning_id) {
            state.status = super::contract::ProvisioningContractStatus::Revoked;
        }
    }
}
