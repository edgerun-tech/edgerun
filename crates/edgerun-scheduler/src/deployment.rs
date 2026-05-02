use crate::collections::HashMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::matches;

pub struct DeploymentManager {
    deployments: HashMap<String, DeploymentHandle>,
}

pub struct DeploymentHandle {
    pub name: String,
    pub provider: [u8; 32],
    pub container_count: u32,
    pub total_cpu_cores: u32,
    pub total_memory_bytes: u64,
    pub status: DeploymentStatus,
    pub assigned: bool,
}

impl Clone for DeploymentHandle {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            provider: self.provider,
            container_count: self.container_count,
            total_cpu_cores: self.total_cpu_cores,
            total_memory_bytes: self.total_memory_bytes,
            status: self.status,
            assigned: self.assigned,
        }
    }
}

impl DeploymentManager {
    pub fn new() -> Self {
        Self {
            deployments: HashMap::new(),
        }
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
    fn test_manager_creation() {
        let _ = DeploymentManager::new();
    }

    #[test]
    fn test_mark_running() {
        let mut dm = DeploymentManager::new();
        dm.create_local(DeploymentHandle {
            name: "test-deployment".to_string(),
            provider: [0u8; 32],
            container_count: 1,
            total_cpu_cores: 2,
            total_memory_bytes: 4_000_000_000,
            status: DeploymentStatus::Created,
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
            name: "test-deployment".to_string(),
            provider: [0u8; 32],
            container_count: 1,
            total_cpu_cores: 2,
            total_memory_bytes: 4_000_000_000,
            status: DeploymentStatus::Running,
            assigned: false,
        });

        dm.set_error("test-deployment", "Container crashed");
        let d = dm.get("test-deployment").unwrap();
        assert!(matches!(d.status, DeploymentStatus::Disputed));
    }
}
