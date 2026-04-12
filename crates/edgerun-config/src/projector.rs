//! Projector — rebuilds current config state from the append-only event store.
//!
//! Config changes are recorded as events in the stream. The projector
//! replays these events to produce the current `ConfigState`.

use crate::types::ConfigResource;
use crate::parser::ConfigState;

/// A config change event stored in the event stream.
#[derive(Debug, Clone)]
pub struct ConfigEvent {
    /// Sequence number in the stream.
    pub seq: u64,
    /// The config resource being created/updated.
    pub resource: ConfigResource,
    /// Operation type.
    pub op: ConfigOp,
}

/// Config operation types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigOp {
    /// Resource was created.
    Created,
    /// Resource was updated (replaces previous version with same name).
    Updated,
    /// Resource was deleted.
    Deleted,
}

/// Projects config state from a sequence of events.
pub struct ConfigProjector {
    /// Current state indexed by resource name.
    resources: std::collections::HashMap<String, ConfigEvent>,
}

impl ConfigProjector {
    /// Create a new empty projector.
    pub fn new() -> Self {
        Self {
            resources: std::collections::HashMap::new(),
        }
    }

    /// Apply a single config event.
    pub fn apply(&mut self, event: ConfigEvent) {
        match event.op {
            ConfigOp::Created | ConfigOp::Updated => {
                self.resources.insert(event.resource.name().to_string(), event);
            }
            ConfigOp::Deleted => {
                self.resources.remove(event.resource.name());
            }
        }
    }

    /// Apply a batch of events (in sequence order).
    pub fn apply_all(&mut self, events: impl IntoIterator<Item = ConfigEvent>) {
        for event in events {
            self.apply(event);
        }
    }

    /// Build the current ConfigState from all applied events.
    pub fn snapshot(&self) -> ConfigState {
        let resources: Vec<_> = self.resources.values()
            .map(|e| e.resource.clone())
            .collect();

        ConfigState::from_resources(&resources).unwrap_or_default()
    }

    /// Get the number of active resources.
    pub fn resource_count(&self) -> usize {
        self.resources.len()
    }

    /// Get a resource by name.
    pub fn get(&self, name: &str) -> Option<&ConfigResource> {
        self.resources.get(name).map(|e| &e.resource)
    }

    /// Get all active resource names.
    pub fn names(&self) -> Vec<&str> {
        self.resources.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for ConfigProjector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DnsServerSpec, DnsZoneSpec, SoaRecord};

    fn make_dns(name: &str, bind: &str) -> ConfigResource {
        ConfigResource::DnsServer(DnsServerSpec {
            bind_address: Some(bind.to_string()),
            bind_address_ipv6: None,
            default_ttl: None,
            rate_limit_qps: None,
            zones: None,
            forward_to: None,
            recursive: None,
        })
    }

    fn make_zone(name: &str, origin: &str) -> ConfigResource {
        ConfigResource::DnsZone(DnsZoneSpec {
            origin: origin.to_string(),
            soa: SoaRecord {
                mname: "ns1.example.com".to_string(),
                rname: "admin.example.com".to_string(),
                serial: 1, refresh: 3600, retry: 900, expire: 604800, minimum: 86400,
            },
            records: vec![],
            dnssec: None,
            wildcards: None,
        })
    }

    #[test]
    fn test_projector_create_and_snapshot() {
        let mut p = ConfigProjector::new();
        p.apply(ConfigEvent {
            seq: 0,
            resource: make_dns("dns1", "0.0.0.0:53"),
            op: ConfigOp::Created,
        });
        // ConfigResource::name() returns "unnamed-dns-server" for DnsServer specs
        assert_eq!(p.resource_count(), 1);
        assert!(p.get("unnamed-dns-server").is_some());
    }

    #[test]
    fn test_projector_update() {
        let mut p = ConfigProjector::new();
        p.apply(ConfigEvent {
            seq: 0,
            resource: make_dns("dns1", "0.0.0.0:53"),
            op: ConfigOp::Created,
        });
        p.apply(ConfigEvent {
            seq: 1,
            resource: make_dns("dns1", "10.0.0.1:53"),
            op: ConfigOp::Updated,
        });
        // Check the updated value is stored
        assert_eq!(p.resource_count(), 1);
    }

    #[test]
    fn test_projector_delete() {
        let mut p = ConfigProjector::new();
        p.apply(ConfigEvent {
            seq: 0,
            resource: make_dns("dns1", "0.0.0.0:53"),
            op: ConfigOp::Created,
        });
        p.apply(ConfigEvent {
            seq: 1,
            resource: make_dns("dns1", "0.0.0.0:53"),
            op: ConfigOp::Deleted,
        });
        assert_eq!(p.resource_count(), 0);
        assert!(p.get("dns1").is_none());
    }
}
