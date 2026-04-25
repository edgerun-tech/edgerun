//! Edgerun Scheduler

pub mod chain;
pub mod deployment;
pub mod error;
pub mod mesh_handler;
pub mod metrics;
pub mod provider;

pub use deployment::{DeploymentHandle, DeploymentManager};
pub use edgerun_solana::DeploymentStatus;
pub use error::SchedulerError;
pub use metrics::{MetricsReceiver, ProviderMetrics};
pub use provider::{ProviderInfo, ProviderManager};

use edgerun_rt::CancellationToken;
use std::collections::HashMap;
use std::sync::Arc;

pub struct Scheduler {
    provider_manager: ProviderManager,
    deployment_manager: DeploymentManager,
    metrics_receiver: MetricsReceiver,
    shutdown: Arc<CancellationToken>,
    provider_deployments: HashMap<[u8; 32], Vec<String>>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            provider_manager: ProviderManager::new(),
            deployment_manager: DeploymentManager::new(),
            metrics_receiver: MetricsReceiver::new(),
            shutdown: Arc::new(CancellationToken::new()),
            provider_deployments: HashMap::new(),
        }
    }

    pub fn with_solana_rpc(mut self, rpc_url: &str) -> Self {
        self.deployment_manager = DeploymentManager::new_with_rpc(rpc_url);
        self
    }

    pub fn add_provider(&mut self, info: ProviderInfo) {
        self.provider_manager.add(info);
    }

    pub fn remove_provider(&mut self, node_id: &[u8; 32]) {
        self.provider_manager.remove(node_id);
    }

    pub fn get_providers(&self) -> Vec<&ProviderInfo> {
        self.provider_manager.list_online()
    }

    pub fn select_provider(
        &self,
        required_cpu: u32,
        required_memory: u64,
    ) -> Option<&ProviderInfo> {
        self.provider_manager.select(required_cpu, required_memory)
    }

    pub fn register_provider_metrics(&mut self, metrics: ProviderMetrics) -> Result<(), String> {
        self.metrics_receiver.receive(metrics.clone())?;

        self.provider_manager.update_metrics(
            &metrics.node_id,
            metrics.cpu_cores_available,
            metrics.memory_bytes_available,
            metrics.storage_bytes_available,
            metrics.network_bytes_sent,
        );

        if let Some(p) = self.provider_manager.get_mut(&metrics.node_id) {
            p.cpu_cores_used = metrics.cpu_cores_used;
            p.memory_bytes_used = metrics.memory_bytes_used;
            p.is_online = true;
        }

        Ok(())
    }

    pub fn assign_deployment(
        &mut self,
        name: &str,
        provider_id: &[u8; 32],
        cpu_avail: u32,
        mem_avail: u64,
    ) -> Result<(), String> {
        if let Some(d) = self.deployment_manager.get(name) {
            if d.total_cpu_cores > cpu_avail || d.total_memory_bytes > mem_avail {
                return Err("Insufficient resources".to_string());
            }
            self.deployment_manager
                .assign_to_provider(name, provider_id);
            self.provider_deployments
                .entry(*provider_id)
                .or_default()
                .push(name.to_string());
            Ok(())
        } else {
            Err("Deployment not found".to_string())
        }
    }

    pub fn get_deployment_manager(&self) -> &DeploymentManager {
        &self.deployment_manager
    }

    pub fn is_shutdown(&self) -> bool {
        self.shutdown.is_cancelled()
    }

    pub fn set_shutdown(&self) {
        self.shutdown.cancel();
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::deployment::DeploymentHandle;
    use crate::metrics::{MetricsReceiver, ProviderMetrics};
    use crate::provider::ProviderInfo;
    use crate::{DeploymentStatus, Scheduler};

    #[test]
    fn test_scheduler_creation() {
        let scheduler = Scheduler::new();
        assert!(!scheduler.is_shutdown());
    }

    #[test]
    fn test_add_provider() {
        let mut scheduler = Scheduler::new();
        let mut info = ProviderInfo::new([1u8; 32], "test-provider");
        info.cpu_cores_available = 4;
        info.memory_bytes_available = 8_000_000_000;
        info.storage_bytes_available = 100_000_000_000;
        info.network_mbps = 1000;
        info.is_online = true;

        scheduler.add_provider(info);
        let providers = scheduler.get_providers();
        assert_eq!(providers.len(), 1);
    }

    #[test]
    fn test_select_provider() {
        let mut scheduler = Scheduler::new();
        let mut info = ProviderInfo::new([1u8; 32], "test-provider");
        info.cpu_cores_available = 4;
        info.memory_bytes_available = 8_000_000_000;
        info.storage_bytes_available = 100_000_000_000;
        info.network_mbps = 1000;
        info.is_online = true;

        scheduler.add_provider(info);

        let selected = scheduler.select_provider(2, 4_000_000_000);
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().cpu_cores_available, 4);
    }

    #[test]
    fn test_assign_deployment() {
        let mut scheduler = Scheduler::new();
        let mut info = ProviderInfo::new([1u8; 32], "test-provider");
        info.cpu_cores_available = 4;
        info.memory_bytes_available = 8_000_000_000;
        info.is_online = true;
        scheduler.add_provider(info.clone());

        let node_id: [u8; 32] = info.node_id;
        scheduler.deployment_manager.create_local(DeploymentHandle {
            on_chain_address: [0u8; 32],
            name: "test".to_string(),
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

        let result = scheduler.assign_deployment("test", &node_id, 2, 4_000_000_000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_register_metrics() {
        let mut scheduler = Scheduler::new();
        let metrics = ProviderMetrics {
            node_id: [1u8; 32],
            timestamp: 1000,
            cpu_cores_used: 2,
            cpu_cores_available: 4,
            memory_bytes_used: 4_000_000_000,
            memory_bytes_available: 8_000_000_000,
            storage_bytes_used: 5_000_000_000,
            storage_bytes_available: 10_000_000_000,
            network_bytes_sent: 1000,
            network_bytes_received: 500,
            container_count: 2,
            active_deployments: 1,
        };

        let result = scheduler.register_provider_metrics(metrics);
        assert!(result.is_ok());
    }

    #[test]
    fn test_metrics_aggregation() {
        let receiver = MetricsReceiver::new();
        let metrics = ProviderMetrics {
            node_id: [1u8; 32],
            timestamp: 1000,
            cpu_cores_used: 2,
            cpu_cores_available: 4,
            memory_bytes_used: 4_000_000_000,
            memory_bytes_available: 8_000_000_000,
            storage_bytes_used: 5_000_000_000,
            storage_bytes_available: 10_000_000_000,
            network_bytes_sent: 1000,
            network_bytes_received: 500,
            container_count: 2,
            active_deployments: 1,
        };

        receiver.receive(metrics.clone()).unwrap();

        let aggregated = receiver.aggregate_metrics(&[1u8; 32]);
        assert!(aggregated.is_some());
        assert_eq!(aggregated.unwrap().cpu_cores_used, 2);
    }
}
