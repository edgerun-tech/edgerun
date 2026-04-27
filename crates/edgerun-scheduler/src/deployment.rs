use crate::collections::HashMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::matches;
use edgerun_solana::{solana_types::Pubkey, DeploymentClient, DeploymentStatus};

pub struct DeploymentManager {
    rpc_url: Option<String>,
    deployments: HashMap<String, DeploymentHandle>,
    deployment_client: Option<DeploymentClient>,
}

pub struct DeploymentHandle {
    pub on_chain_address: [u8; 32],
    pub name: String,
    pub provider: [u8; 32],
    pub container_count: u32,
    pub total_cpu_cores: u32,
    pub total_memory_bytes: u64,
    pub status: DeploymentStatus,
    pub deposit: u64,
    pub burn_rate: u64,
    pub spent: u64,
    pub assigned: bool,
}

impl Clone for DeploymentHandle {
    fn clone(&self) -> Self {
        Self {
            on_chain_address: self.on_chain_address,
            name: self.name.clone(),
            provider: self.provider,
            container_count: self.container_count,
            total_cpu_cores: self.total_cpu_cores,
            total_memory_bytes: self.total_memory_bytes,
            status: self.status,
            deposit: self.deposit,
            burn_rate: self.burn_rate,
            spent: self.spent,
            assigned: self.assigned,
        }
    }
}

impl DeploymentManager {
    pub fn new() -> Self {
        Self {
            rpc_url: None,
            deployments: HashMap::new(),
            deployment_client: None,
        }
    }

    pub fn new_with_rpc(rpc_url: &str) -> Self {
        let mut this = Self::new();
        this.rpc_url = Some(rpc_url.to_string());
        if let Ok(client) = DeploymentClient::new(rpc_url) {
            this.deployment_client = Some(client);
        }
        this
    }

    pub fn create_local(&mut self, handle: DeploymentHandle) {
        self.deployments.insert(handle.name.clone(), handle);
    }

    pub fn get(&self, name: &str) -> Option<&DeploymentHandle> {
        self.deployments.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut DeploymentHandle> {
        self.deployments.get_mut(name)
    }

    pub fn remove(&mut self, name: &str) -> Option<DeploymentHandle> {
        self.deployments.remove(name)
    }

    pub fn list(&self) -> Vec<&DeploymentHandle> {
        self.deployments.values().collect()
    }

    pub fn iter_mut(&mut self) -> &mut HashMap<String, DeploymentHandle> {
        &mut self.deployments
    }

    pub fn list_active(&self) -> Vec<&DeploymentHandle> {
        self.deployments
            .values()
            .filter(|d| matches!(d.status, DeploymentStatus::Running))
            .collect()
    }

    pub fn list_unassigned(&self) -> Vec<DeploymentHandle> {
        self.deployments
            .values()
            .filter(|d| !d.assigned && matches!(d.status, DeploymentStatus::Running))
            .cloned()
            .collect()
    }

    pub fn assign_to_provider(&mut self, name: &str, provider_id: &[u8; 32]) -> bool {
        if let Some(d) = self.deployments.get_mut(name) {
            d.provider = *provider_id;
            d.assigned = true;
            return true;
        }
        false
    }

    pub fn unassign(&mut self, name: &str) -> bool {
        if let Some(d) = self.deployments.get_mut(name) {
            d.assigned = false;
            return true;
        }
        false
    }

    pub fn sync_all(&mut self) {
        if let Some(client) = &self.deployment_client {
            for (_, handle) in self.deployments.iter_mut() {
                let pubkey = Pubkey::new_from_array(handle.on_chain_address);
                if let Ok(deployment) = client.get_deployment(&pubkey) {
                    handle.status = deployment.status;
                    handle.spent = deployment.spent;
                    handle.total_cpu_cores = deployment.total_cpu_cores;
                    handle.total_memory_bytes = deployment.total_memory_bytes;
                }
            }
        }
    }

    pub fn calculate_burn_rate(
        _container_count: u32,
        cpu: u32,
        memory: u64,
        storage: u64,
        network: u32,
    ) -> u64 {
        use edgerun_solana::types::pricing;

        let cpu_cost = cpu as u64 * pricing::CORE_HOUR;
        let memory_gib = memory.div_ceil(1024 * 1024 * 1024);
        let memory_cost = memory_gib * pricing::RAM_GIB_HOUR;
        let storage_gib = storage.div_ceil(1024 * 1024 * 1024);
        let storage_cost = storage_gib * pricing::STORAGE_GIB_HOUR;
        let network_cost = network as u64 * pricing::NETWORK_MBIT_HOUR;

        (cpu_cost + memory_cost + storage_cost + network_cost) / 3600
    }

    pub fn mark_running(&mut self, name: &str) {
        if let Some(d) = self.deployments.get_mut(name) {
            d.status = DeploymentStatus::Running;
        }
    }

    pub fn set_error(&mut self, name: &str, _error: &str) {
        if let Some(d) = self.deployments.get_mut(name) {
            d.status = DeploymentStatus::Disputed;
        }
    }
}

impl Default for DeploymentManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_burn_rate_calculation() {
        let rate = DeploymentManager::calculate_burn_rate(1, 4, 8_000_000_000, 10_000_000_000, 100);
        assert!(rate > 0);
    }

    #[test]
    fn test_manager_creation() {
        let _ = DeploymentManager::new();
    }

    #[test]
    fn test_mark_running() {
        let mut dm = DeploymentManager::new();
        dm.create_local(DeploymentHandle {
            on_chain_address: [0u8; 32],
            name: "test-deployment".to_string(),
            provider: [0u8; 32],
            container_count: 1,
            total_cpu_cores: 2,
            total_memory_bytes: 4_000_000_000,
            status: DeploymentStatus::Created,
            deposit: 1_000_000_000,
            burn_rate: 100,
            spent: 0,
            assigned: false,
        });

        dm.mark_running("test-deployment");
        let d = dm.get("test-deployment").unwrap();
        assert!(matches!(d.status, DeploymentStatus::Running));
    }

    #[test]
    fn test_set_error() {
        let mut dm = DeploymentManager::new();
        dm.create_local(DeploymentHandle {
            on_chain_address: [0u8; 32],
            name: "test-deployment".to_string(),
            provider: [0u8; 32],
            container_count: 1,
            total_cpu_cores: 2,
            total_memory_bytes: 4_000_000_000,
            status: DeploymentStatus::Running,
            deposit: 1_000_000_000,
            burn_rate: 100,
            spent: 0,
            assigned: false,
        });

        dm.set_error("test-deployment", "Container crashed");
        let d = dm.get("test-deployment").unwrap();
        assert!(matches!(d.status, DeploymentStatus::Disputed));
    }
}
