//! Node address handling for edgerun email.
//!
//! Address format: `{node_id_hex}@nodes.edgerun.tech`
//!
//! Two delivery paths:
//! - Local: stored in maildir, validated by node identity
//! - Mesh: signed delivery via mesh channels

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};

type HashMap<K, V> = BTreeMap<K, V>;

/// Node address domain
pub const NODES_DOMAIN: &str = "nodes.edgerun.tech";

/// Parse a node address into (node_id_hex, domain)
pub fn parse_node_address(address: &str) -> Option<(&str, &str)> {
    if let Some((local_part, domain)) = address.split_once('@') {
        if domain == NODES_DOMAIN && local_part.len() == 64 {
            return Some((local_part, domain));
        }
    }
    None
}

/// Check if address is a node address
pub fn is_node_address(address: &str) -> bool {
    parse_node_address(address).is_some()
}

/// Extract node ID from address (hex string)
pub fn node_id_from_address(address: &str) -> Option<String> {
    parse_node_address(address).map(|(node_id, _)| node_id.to_string())
}

/// Validate a node address exists locally
///
/// In production, this checks if we have a channel/credential for this node.
/// For now, we keep a local registry of known nodes.
pub struct NodeRegistry {
    known_nodes: HashMap<String, NodeInfo>,
}

#[derive(Clone, Debug)]
pub struct NodeInfo {
    pub node_id: String,
    pub channel_address: Option<String>,
    pub verified: bool,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self {
            known_nodes: HashMap::new(),
        }
    }

    /// Register a known node
    pub fn register(&mut self, node_id_hex: String, channel_address: Option<String>) {
        self.known_nodes.insert(
            node_id_hex.clone(),
            NodeInfo {
                node_id: node_id_hex,
                channel_address,
                verified: false,
            },
        );
    }

    /// Check if a node is known/registered
    pub fn is_known(&self, node_id_hex: &str) -> bool {
        self.known_nodes.contains_key(node_id_hex)
    }

    /// Get node info
    pub fn get(&self, node_id_hex: &str) -> Option<&NodeInfo> {
        self.known_nodes.get(node_id_hex)
    }

    /// Mark a node as verified (proved ownership via signing)
    pub fn mark_verified(&mut self, node_id_hex: &str) {
        if let Some(info) = self.known_nodes.get_mut(node_id_hex) {
            info.verified = true;
        }
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_node_address() {
        let node_id = "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2";
        let addr = format!("{}@nodes.edgerun.tech", node_id);

        let result = parse_node_address(&addr);
        assert!(result.is_some());
        let (id, domain) = result.unwrap();
        assert_eq!(id, node_id);
        assert_eq!(domain, "nodes.edgerun.tech");

        // Invalid addresses
        assert!(parse_node_address("user@external.com").is_none());
        assert!(parse_node_address("short@nodes.edgerun.tech").is_none());
    }

    #[test]
    fn test_node_registry() {
        let mut registry = NodeRegistry::new();

        let node_id = "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2";

        assert!(!registry.is_known(node_id));

        registry.register(
            node_id.to_string(),
            Some("/ipc/channels/abc123".to_string()),
        );

        assert!(registry.is_known(node_id));

        let info = registry.get(node_id).unwrap();
        assert!(!info.verified);

        registry.mark_verified(node_id);

        let info = registry.get(node_id).unwrap();
        assert!(info.verified);
    }
}
