use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use edgerun_mesh::mesh_payload::{MetricsReportPayload, DeploymentMetrics as MeshDeploymentMetrics};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMetrics {
    pub node_id: [u8; 32],
    pub timestamp: u64,
    pub cpu_cores_used: u32,
    pub cpu_cores_available: u32,
    pub memory_bytes_used: u64,
    pub memory_bytes_available: u64,
    pub storage_bytes_used: u64,
    pub storage_bytes_available: u64,
    pub network_bytes_sent: u64,
    pub network_bytes_received: u64,
    pub container_count: u32,
    pub active_deployments: u32,
}

impl ProviderMetrics {
    pub fn new(node_id: [u8; 32]) -> Self {
        Self {
            node_id,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            cpu_cores_used: 0,
            cpu_cores_available: 0,
            memory_bytes_used: 0,
            memory_bytes_available: 0,
            storage_bytes_used: 0,
            storage_bytes_available: 0,
            network_bytes_sent: 0,
            network_bytes_received: 0,
            container_count: 0,
            active_deployments: 0,
        }
    }
    
    pub fn from_mesh_payload(node_id: [u8; 32], payload: &MetricsReportPayload) -> Self {
        Self {
            node_id,
            timestamp: payload.timestamp,
            cpu_cores_used: payload.cpu_cores_used,
            cpu_cores_available: payload.cpu_cores_available,
            memory_bytes_used: payload.memory_bytes_used,
            memory_bytes_available: payload.memory_bytes_available,
            storage_bytes_used: payload.storage_bytes_used,
            storage_bytes_available: payload.storage_bytes_available,
            network_bytes_sent: payload.network_bytes_sent,
            network_bytes_received: payload.network_bytes_received,
            container_count: payload.container_count,
            active_deployments: payload.deployments.len() as u32,
        }
    }
    
    pub fn cpu_utilization(&self) -> f64 {
        if self.cpu_cores_available == 0 {
            return 0.0;
        }
        (self.cpu_cores_used as f64 / self.cpu_cores_available as f64) * 100.0
    }
    
    pub fn memory_utilization(&self) -> f64 {
        if self.memory_bytes_available == 0 {
            return 0.0;
        }
        (self.memory_bytes_used as f64 / self.memory_bytes_available as f64) * 100.0
    }
}

pub struct MetricsReceiver {
    history: parking_lot::RwLock<Vec<ProviderMetrics>>,
}

impl MetricsReceiver {
    pub fn new() -> Self {
        Self {
            history: parking_lot::RwLock::new(Vec::new()),
        }
    }
    
    pub fn receive(&self, metrics: ProviderMetrics) -> Result<(), String> {
        if metrics.cpu_cores_available == 0 {
            return Err("Invalid metrics: cpu_cores_available is 0".to_string());
        }
        if metrics.memory_bytes_available == 0 {
            return Err("Invalid metrics: memory_bytes_available is 0".to_string());
        }
        self.history.write().push(metrics);
        Ok(())
    }
    
    pub fn receive_from_mesh(&self, node_id: [u8; 32], payload: &MetricsReportPayload) -> Result<ProviderMetrics, String> {
        let metrics = ProviderMetrics::from_mesh_payload(node_id, payload);
        self.receive(metrics.clone())?;
        Ok(metrics)
    }
    
    pub fn get_history(&self, node_id: &[u8; 32]) -> Vec<ProviderMetrics> {
        self.history.read()
            .iter()
            .filter(|m| &m.node_id == node_id)
            .cloned()
            .collect()
    }
    
    pub fn aggregate_metrics(&self, node_id: &[u8; 32]) -> Option<ProviderMetrics> {
        let history = self.history.read();
        let samples: Vec<_> = history.iter()
            .filter(|m| &m.node_id == node_id)
            .collect();
        
        if samples.is_empty() {
            return None;
        }
        
        let avg_cpu = samples.iter().map(|m| m.cpu_cores_used as f64).sum::<f64>() / samples.len() as f64;
        let avg_mem = samples.iter().map(|m| m.memory_bytes_used as f64).sum::<f64>() / samples.len() as f64;
        let avg_net = samples.iter().map(|m| m.network_bytes_sent as f64).sum::<f64>() / samples.len() as f64;
        
        let latest = samples.last()?;
        Some(ProviderMetrics {
            node_id: *node_id,
            timestamp: latest.timestamp,
            cpu_cores_used: avg_cpu as u32,
            cpu_cores_available: latest.cpu_cores_available,
            memory_bytes_used: avg_mem as u64,
            memory_bytes_available: latest.memory_bytes_available,
            storage_bytes_used: latest.storage_bytes_used,
            storage_bytes_available: latest.storage_bytes_available,
            network_bytes_sent: avg_net as u64,
            network_bytes_received: latest.network_bytes_received,
            container_count: latest.container_count,
            active_deployments: latest.active_deployments,
        })
    }
}

impl Default for MetricsReceiver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metrics_creation() {
        let metrics = ProviderMetrics::new([0u8; 32]);
        assert!(metrics.timestamp > 0);
    }
}