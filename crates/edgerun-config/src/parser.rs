//! Config file parser — reads YAML and produces typed config resources.

use crate::types::ConfigResource;
use serde::Deserialize;
use edgerun_json::yaml::{YamlValue, YamlDeserializer};

#[derive(Debug, Clone, serde::Deserialize)]
struct RawDoc {
    kind: String,
    spec: YamlValue,
}

pub fn parse_config_file(yaml: &str) -> Result<Vec<ConfigResource>, ConfigError> {
    let mut resources = Vec::new();
    
    for doc_result in YamlDeserializer::from_str(yaml) {
        let doc = match doc_result {
            Ok(d) => d,
            Err(e) => return Err(ConfigError::ParseError(e.to_string())),
        };
        
        if doc.is_null() {
            continue;
        }
        
        let kind = doc.get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        let spec = doc.get("spec")
            .cloned()
            .unwrap_or(YamlValue::Null);
        
        let res = match kind {
            "DnsServer" => deserialize_resource::<crate::types::DnsServerSpec>(spec)
                .map(ConfigResource::DnsServer),
            "DnsZone" => deserialize_resource::<crate::types::DnsZoneSpec>(spec)
                .map(ConfigResource::DnsZone),
            "DnsForwarder" => deserialize_resource::<crate::types::DnsForwarderSpec>(spec)
                .map(ConfigResource::DnsForwarder),
            "ForwardingRule" => deserialize_resource::<crate::types::ForwardingRuleSpec>(spec)
                .map(ConfigResource::ForwardingRule),
            "TlsConfig" => deserialize_resource::<crate::types::TlsConfigSpec>(spec)
                .map(ConfigResource::TlsConfig),
            "RateLimit" => deserialize_resource::<crate::types::RateLimitSpec>(spec)
                .map(ConfigResource::RateLimit),
            "DhcpServer" => deserialize_resource::<crate::types::DhcpServerSpec>(spec)
                .map(ConfigResource::DhcpServer),
            "DhcpPool" => deserialize_resource::<crate::types::DhcpPoolSpec>(spec)
                .map(ConfigResource::DhcpPool),
            "TftpServer" => deserialize_resource::<crate::types::TftpServerSpec>(spec)
                .map(ConfigResource::TftpServer),
            _ => continue,
        };
        
        if let Ok(r) = res {
            resources.push(r);
        }
    }

    if resources.is_empty() {
        return Err(ConfigError::EmptyFile);
    }

    Ok(resources)
}

fn deserialize_resource<T: serde::de::DeserializeOwned>(spec: YamlValue) -> Result<T, ConfigError> {
    let json = yaml_to_json(spec);
    edgerun_json::from_value(json)
        .map_err(|e| ConfigError::ParseError(e.to_string()))
}

fn yaml_to_json(yaml: YamlValue) -> edgerun_json::JsonValue {
    match yaml {
        YamlValue::Null => edgerun_json::JsonValue::Null,
        YamlValue::Bool(b) => edgerun_json::JsonValue::Bool(b),
        YamlValue::Number(n) => edgerun_json::JsonValue::Number(n),
        YamlValue::String(s) => edgerun_json::JsonValue::String(s),
        YamlValue::Array(arr) => {
            edgerun_json::JsonValue::Array(arr.into_iter().map(yaml_to_json).collect())
        }
        YamlValue::Mapping(map) => {
            let mut obj = edgerun_json::Map::new();
            for (k, v) in map {
                obj.insert(k, yaml_to_json(v));
            }
            edgerun_json::JsonValue::Object(obj)
        }
        YamlValue::Tagged(tagged) => yaml_to_json(*tagged.value),
    }
}

pub fn to_yaml_all(resources: &[ConfigResource]) -> Result<String, ConfigError> {
    let mut out = String::new();
    for (i, res) in resources.iter().enumerate() {
        if i > 0 { out.push_str("---\n"); }
        
        let json = edgerun_json::to_value(res)
            .map_err(|e| ConfigError::ParseError(e.to_string()))?;
        let yaml = edgerun_json::yaml::json_to_yaml(json);
        let yaml_str = edgerun_json::yaml::to_yaml_string(&yaml)
            .map_err(|e| ConfigError::ParseError(e.to_string()))?;
        out.push_str(&yaml_str);
    }
    Ok(out)
}

pub fn parse_and_validate(yaml: &str) -> Result<ConfigState, ConfigError> {
    let resources = parse_config_file(yaml)?;
    let state = ConfigState::from_resources(&resources)?;
    Ok(state)
}

#[derive(Debug, Clone, Default)]
pub struct ConfigState {
    pub dns_servers: Vec<crate::types::DnsServerSpec>,
    pub dns_zones: Vec<crate::types::DnsZoneSpec>,
    pub dns_forwarders: Vec<crate::types::DnsForwarderSpec>,
    pub forwarding_rules: Vec<crate::types::ForwardingRuleSpec>,
    pub tls_configs: Vec<crate::types::TlsConfigSpec>,
    pub rate_limits: Vec<crate::types::RateLimitSpec>,
    pub dhcp_servers: Vec<crate::types::DhcpServerSpec>,
    pub dhcp_pools: Vec<crate::types::DhcpPoolSpec>,
    pub dhcpv6_servers: Vec<crate::types::Dhcpv6ServerSpec>,
    pub dhcpv6_pools: Vec<crate::types::Dhcpv6PoolSpec>,
    pub tftp_servers: Vec<crate::types::TftpServerSpec>,
    pub smtp_servers: Vec<crate::types::SmtpServerSpec>,
    pub imap_servers: Vec<crate::types::ImapServerSpec>,
}

impl ConfigState {
    pub fn from_resources(resources: &[ConfigResource]) -> Result<Self, ConfigError> {
        let mut state = ConfigState::default();
        for res in resources {
            match res {
                ConfigResource::DnsServer(spec) => state.dns_servers.push(spec.clone()),
                ConfigResource::DnsZone(spec) => state.dns_zones.push(spec.clone()),
                ConfigResource::DnsForwarder(spec) => state.dns_forwarders.push(spec.clone()),
                ConfigResource::ForwardingRule(spec) => state.forwarding_rules.push(spec.clone()),
                ConfigResource::TlsConfig(spec) => state.tls_configs.push(spec.clone()),
                ConfigResource::RateLimit(spec) => state.rate_limits.push(spec.clone()),
                ConfigResource::DhcpServer(spec) => state.dhcp_servers.push(spec.clone()),
                ConfigResource::DhcpPool(spec) => state.dhcp_pools.push(spec.clone()),
                ConfigResource::Dhcpv6Server(spec) => state.dhcpv6_servers.push(spec.clone()),
                ConfigResource::Dhcpv6Pool(spec) => state.dhcpv6_pools.push(spec.clone()),
                ConfigResource::TftpServer(spec) => state.tftp_servers.push(spec.clone()),
                ConfigResource::SmtpServer(spec) => state.smtp_servers.push(spec.clone()),
                ConfigResource::ImapServer(spec) => state.imap_servers.push(spec.clone()),
            }
        }
        state.validate()?;
        Ok(state)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        for (i, dns) in self.dns_servers.iter().enumerate() {
            if let Some(zones) = &dns.zones {
                for zone_name in zones {
                    if !self.dns_zones.iter().any(|z| z.origin == *zone_name) {
                        return Err(ConfigError::ValidationError(format!(
                            "dns_servers[{}]: references zone '{}' which does not exist", i, zone_name
                        )));
                    }
                }
            }
        }
        for (i, dhcp) in self.dhcp_servers.iter().enumerate() {
            for pool_name in &dhcp.pools {
                if !self.dhcp_pools.iter().any(|p| p.name == *pool_name) {
                    return Err(ConfigError::ValidationError(format!(
                        "dhcp_servers[{}]: references pool '{}' which does not exist", i, pool_name
                    )));
                }
            }
        }
        for (i, zone) in self.dns_zones.iter().enumerate() {
            if !zone.records.iter().any(|r| r.record_type == "NS") {
                return Err(ConfigError::ValidationError(format!(
                    "dns_zones[{}]: zone '{}' has no NS record", i, zone.origin
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ConfigError {
    #[error("config file is empty")]
    EmptyFile,
    #[error("parse error: {0}")]
    ParseError(String),
    #[error("validation error: {0}")]
    ValidationError(String),
}