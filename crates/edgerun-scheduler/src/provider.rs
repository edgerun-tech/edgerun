use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub node_id: [u8; 32],
    pub name: String,
    pub cpu_cores_available: u32,
    pub cpu_cores_used: u32,
    pub memory_bytes_available: u64,
    pub memory_bytes_used: u64,
    pub storage_bytes_available: u64,
    pub storage_bytes_used: u64,
    pub network_mbps: u32,
    pub uptime_percent: u32,
    pub slash_count: u32,
    pub is_online: bool,
}

impl ProviderInfo {
    pub fn new(node_id: [u8; 32], name: &str) -> Self {
        Self {
            node_id,
            name: name.to_string(),
            cpu_cores_available: 0,
            cpu_cores_used: 0,
            memory_bytes_available: 0,
            memory_bytes_used: 0,
            storage_bytes_available: 0,
            storage_bytes_used: 0,
            network_mbps: 0,
            uptime_percent: 10000,
            slash_count: 0,
            is_online: false,
        }
    }

    pub fn can_fit(&self, cpu: u32, memory: u64) -> bool {
        self.cpu_cores_available >= cpu && self.memory_bytes_available >= memory
    }
}

pub struct ProviderManager {
    providers: HashMap<[u8; 32], ProviderInfo>,
}

impl ProviderManager {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    pub fn add(&mut self, info: ProviderInfo) {
        self.providers.insert(info.node_id, info);
    }

    pub fn remove(&mut self, node_id: &[u8; 32]) {
        self.providers.remove(node_id);
    }

    pub fn get(&self, node_id: &[u8; 32]) -> Option<&ProviderInfo> {
        self.providers.get(node_id)
    }

    pub fn get_mut(&mut self, node_id: &[u8; 32]) -> Option<&mut ProviderInfo> {
        self.providers.get_mut(node_id)
    }

    pub fn list(&self) -> Vec<&ProviderInfo> {
        self.providers.values().collect()
    }

    pub fn list_online(&self) -> Vec<&ProviderInfo> {
        self.providers.values().filter(|p| p.is_online).collect()
    }

    pub fn select(&self, required_cpu: u32, required_memory: u64) -> Option<&ProviderInfo> {
        self.providers
            .values()
            .filter(|p| p.is_online && p.can_fit(required_cpu, required_memory))
            .max_by_key(|p| p.uptime_percent.saturating_sub(p.slash_count * 100))
    }

    pub fn set_online(&mut self, node_id: &[u8; 32], online: bool) {
        if let Some(p) = self.providers.get_mut(node_id) {
            p.is_online = online;
        }
    }

    pub fn update_metrics(
        &mut self,
        node_id: &[u8; 32],
        cpu_available: u32,
        memory_available: u64,
        storage_available: u64,
        _network_sent: u64,
    ) {
        if let Some(p) = self.providers.get_mut(node_id) {
            p.cpu_cores_available = cpu_available;
            p.memory_bytes_available = memory_available;
            p.storage_bytes_available = storage_available;
        }
    }
}

impl Default for ProviderManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_info() {
        let info = ProviderInfo::new([0u8; 32], "test-provider");
        assert_eq!(info.uptime_percent, 10000);
    }

    #[test]
    fn test_provider_selection() {
        let mut mgr = ProviderManager::new();

        let mut p1 = ProviderInfo::new([1u8; 32], "p1");
        p1.cpu_cores_available = 4;
        p1.memory_bytes_available = 8_000_000_000;
        p1.is_online = true;
        mgr.add(p1);

        let mut p2 = ProviderInfo::new([2u8; 32], "p2");
        p2.cpu_cores_available = 8;
        p2.memory_bytes_available = 16_000_000_000;
        p2.is_online = true;
        p2.uptime_percent = 9500;
        mgr.add(p2);

        let selected = mgr.select(4, 4_000_000_000);
        assert!(selected.is_some());
    }
}
