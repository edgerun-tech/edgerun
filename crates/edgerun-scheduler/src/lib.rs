//! Edgerun Scheduler

pub mod provider;
pub mod deployment;
pub mod metrics;
pub mod chain;
pub mod error;
pub mod mesh_handler;

pub use provider::{ProviderManager, ProviderInfo};
pub use deployment::{DeploymentManager, DeploymentHandle};
pub use metrics::{MetricsReceiver, ProviderMetrics};
pub use error::SchedulerError;

use edgerun_rt::CancellationToken;
use std::sync::Arc;
use std::collections::HashMap;

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
    
    pub fn select_provider(&self, required_cpu: u32, required_memory: u64) -> Option<&ProviderInfo> {
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
    
    pub fn assign_deployment(&mut self, name: &str, provider_id: &[u8; 32], cpu_avail: u32, mem_avail: u64) -> Result<(), String> {
        if let Some(d) = self.deployment_manager.get(name) {
            if d.total_cpu_cores > cpu_avail || d.total_memory_bytes > mem_avail {
                return Err("Insufficient resources".to_string());
            }
            self.deployment_manager.assign_to_provider(name, provider_id);
            self.provider_deployments
                .entry(*provider_id)
                .or_insert_with(Vec::new)
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
    use super::*;
    
    #[test]
    fn test_scheduler_creation() {
        let _ = Scheduler::new();
    }
}