use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

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

pub struct MetricsReceiver;

impl MetricsReceiver {
    pub fn new() -> Self {
        Self
    }
    
    pub fn receive(&self, metrics: ProviderMetrics) -> Result<(), String> {
        if metrics.cpu_cores_available == 0 {
            return Err("Invalid metrics: cpu_cores_available is 0".to_string());
        }
        if metrics.memory_bytes_available == 0 {
            return Err("Invalid metrics: memory_bytes_available is 0".to_string());
        }
        Ok(())
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