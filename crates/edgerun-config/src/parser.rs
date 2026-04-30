//! Config file parser — reads YAML and produces typed config resources.

use crate::collections::HashMap;
use crate::prelude::v1::*;
use crate::types::ConfigResource;
use edgerun_json::yaml::{YamlDeserializer, YamlValue};
use edgerun_json::FromJson;

#[derive(Debug, Clone)]
struct RawDoc {
    kind: String,
    spec: YamlValue,
}

pub fn parse_config_file(yaml: &str) -> Result<Vec<ConfigResource>, ConfigError> {
    let mut resources = Vec::new();

    for doc_result in YamlDeserializer::parse(yaml) {
        let doc = match doc_result {
            Ok(d) => d,
            Err(e) => return Err(ConfigError::ParseError(e.to_string())),
        };

        if doc.is_null() {
            continue;
        }

        let kind = doc.get("kind").and_then(|v| v.as_str()).unwrap_or("");

        let spec = doc.get("spec").cloned().unwrap_or(YamlValue::Null);

        let res =
            match kind {
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
                "Dhcpv6Server" => deserialize_resource::<crate::types::Dhcpv6ServerSpec>(spec)
                    .map(ConfigResource::Dhcpv6Server),
                "Dhcpv6Pool" => deserialize_resource::<crate::types::Dhcpv6PoolSpec>(spec)
                    .map(ConfigResource::Dhcpv6Pool),
                "TftpServer" => deserialize_resource::<crate::types::TftpServerSpec>(spec)
                    .map(ConfigResource::TftpServer),
                "SmtpServer" => deserialize_resource::<crate::types::SmtpServerSpec>(spec)
                    .map(ConfigResource::SmtpServer),
                "ImapServer" => deserialize_resource::<crate::types::ImapServerSpec>(spec)
                    .map(ConfigResource::ImapServer),
                "Node" => {
                    deserialize_resource::<crate::types::NodeSpec>(spec).map(ConfigResource::Node)
                }
                "BrowserApp" => deserialize_resource::<crate::types::BrowserAppSpec>(spec)
                    .map(ConfigResource::BrowserApp),
                "BrowserNodePolicy" => {
                    deserialize_resource::<crate::types::BrowserNodePolicySpec>(spec)
                        .map(ConfigResource::BrowserNodePolicy)
                }
                "Container" => deserialize_resource::<crate::types::ContainerSpec>(spec)
                    .map(ConfigResource::Container),
                "Deployment" => deserialize_resource::<crate::types::DeploymentSpec>(spec)
                    .map(ConfigResource::Deployment),
                "Secret" => deserialize_resource::<crate::types::SecretSpec>(spec)
                    .map(ConfigResource::Secret),
                "Peer" => {
                    deserialize_resource::<crate::types::PeerSpec>(spec).map(ConfigResource::Peer)
                }
                "Gateway" => deserialize_resource::<crate::types::GatewaySpec>(spec)
                    .map(ConfigResource::Gateway),
                "Service" => deserialize_resource::<crate::types::ServiceSpec>(spec)
                    .map(ConfigResource::Service),
                "HttpRoute" => deserialize_resource::<crate::types::HttpRouteSpec>(spec)
                    .map(ConfigResource::HttpRoute),
                "TcpRoute" => deserialize_resource::<crate::types::TcpRouteSpec>(spec)
                    .map(ConfigResource::TcpRoute),
                "TlsRoute" => deserialize_resource::<crate::types::TlsRouteSpec>(spec)
                    .map(ConfigResource::TlsRoute),
                _ => continue,
            };

        resources.push(res.map_err(|err| ConfigError::ParseError(format!("{kind}: {err}")))?);
    }

    if resources.is_empty() {
        return Err(ConfigError::EmptyFile);
    }

    Ok(resources)
}

fn deserialize_resource<T: FromJson>(spec: YamlValue) -> Result<T, ConfigError> {
    let json = yaml_to_json(spec);
    T::from_json(json).map_err(|e| ConfigError::ParseError(e.to_string()))
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
        if i > 0 {
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("---\n");
        }

        let json = resource_to_json(res);
        let yaml = edgerun_json::yaml::json_to_yaml(json);
        let yaml_str = edgerun_json::yaml::to_yaml_string(&yaml)
            .map_err(|e| ConfigError::ParseError(e.to_string()))?;
        out.push_str(&yaml_str);
    }
    Ok(out)
}

fn resource_to_json(res: &ConfigResource) -> edgerun_json::JsonValue {
    let spec = match res {
        ConfigResource::DnsServer(spec) => edgerun_json::ToJson::to_json(spec),
        ConfigResource::DnsZone(spec) => edgerun_json::ToJson::to_json(spec),
        ConfigResource::DnsForwarder(spec) => edgerun_json::ToJson::to_json(spec),
        ConfigResource::ForwardingRule(spec) => edgerun_json::ToJson::to_json(spec),
        ConfigResource::TlsConfig(spec) => edgerun_json::ToJson::to_json(spec),
        ConfigResource::RateLimit(spec) => edgerun_json::ToJson::to_json(spec),
        ConfigResource::DhcpServer(spec) => edgerun_json::ToJson::to_json(spec),
        ConfigResource::DhcpPool(spec) => edgerun_json::ToJson::to_json(spec),
        ConfigResource::Dhcpv6Server(spec) => edgerun_json::ToJson::to_json(spec),
        ConfigResource::Dhcpv6Pool(spec) => edgerun_json::ToJson::to_json(spec),
        ConfigResource::TftpServer(spec) => edgerun_json::ToJson::to_json(spec),
        ConfigResource::SmtpServer(spec) => edgerun_json::ToJson::to_json(spec),
        ConfigResource::ImapServer(spec) => edgerun_json::ToJson::to_json(spec),
        ConfigResource::Node(spec) => edgerun_json::ToJson::to_json(spec),
        ConfigResource::BrowserApp(spec) => edgerun_json::ToJson::to_json(spec),
        ConfigResource::BrowserNodePolicy(spec) => edgerun_json::ToJson::to_json(spec),
        ConfigResource::Deployment(spec) => edgerun_json::ToJson::to_json(spec),
        ConfigResource::Container(spec) => edgerun_json::ToJson::to_json(spec),
        ConfigResource::Secret(spec) => edgerun_json::ToJson::to_json(spec),
        ConfigResource::Peer(spec) => edgerun_json::ToJson::to_json(spec),
        ConfigResource::Gateway(spec) => edgerun_json::ToJson::to_json(spec),
        ConfigResource::Service(spec) => edgerun_json::ToJson::to_json(spec),
        ConfigResource::HttpRoute(spec) => edgerun_json::ToJson::to_json(spec),
        ConfigResource::TcpRoute(spec) => edgerun_json::ToJson::to_json(spec),
        ConfigResource::TlsRoute(spec) => edgerun_json::ToJson::to_json(spec),
    };
    let mut object = edgerun_json::Map::new();
    object.push_field("kind", res.kind());
    object.push_field("spec", spec);
    object.into()
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
    pub nodes: Vec<crate::types::NodeSpec>,
    pub browser_apps: Vec<crate::types::BrowserAppSpec>,
    pub browser_node_policies: Vec<crate::types::BrowserNodePolicySpec>,
    pub containers: Vec<crate::types::ContainerSpec>,
    pub deployments: Vec<crate::types::DeploymentSpec>,
    pub secrets: Vec<crate::types::SecretSpec>,
    pub peers: Vec<crate::types::PeerSpec>,
    pub gateways: Vec<crate::types::GatewaySpec>,
    pub services: Vec<crate::types::ServiceSpec>,
    pub http_routes: Vec<crate::types::HttpRouteSpec>,
    pub tcp_routes: Vec<crate::types::TcpRouteSpec>,
    pub tls_routes: Vec<crate::types::TlsRouteSpec>,
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
                ConfigResource::Node(spec) => state.nodes.push(spec.clone()),
                ConfigResource::BrowserApp(spec) => state.browser_apps.push(spec.clone()),
                ConfigResource::BrowserNodePolicy(spec) => {
                    state.browser_node_policies.push(spec.clone())
                }
                ConfigResource::Container(spec) => state.containers.push(spec.clone()),
                ConfigResource::Deployment(spec) => state.deployments.push(spec.clone()),
                ConfigResource::Secret(spec) => state.secrets.push(spec.clone()),
                ConfigResource::Peer(spec) => state.peers.push(spec.clone()),
                ConfigResource::Gateway(spec) => state.gateways.push(spec.clone()),
                ConfigResource::Service(spec) => state.services.push(spec.clone()),
                ConfigResource::HttpRoute(spec) => state.http_routes.push(spec.clone()),
                ConfigResource::TcpRoute(spec) => state.tcp_routes.push(spec.clone()),
                ConfigResource::TlsRoute(spec) => state.tls_routes.push(spec.clone()),
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
                            "dns_servers[{}]: references zone '{}' which does not exist",
                            i, zone_name
                        )));
                    }
                }
            }
        }
        for (i, dhcp) in self.dhcp_servers.iter().enumerate() {
            for pool_name in &dhcp.pools {
                if !self.dhcp_pools.iter().any(|p| p.name == *pool_name) {
                    return Err(ConfigError::ValidationError(format!(
                        "dhcp_servers[{}]: references pool '{}' which does not exist",
                        i, pool_name
                    )));
                }
            }
        }
        for (i, zone) in self.dns_zones.iter().enumerate() {
            if !zone.records.iter().any(|r| r.record_type == "NS") {
                return Err(ConfigError::ValidationError(format!(
                    "dns_zones[{}]: zone '{}' has no NS record",
                    i, zone.origin
                )));
            }
        }
        Ok(())
    }

    pub fn build_dhcp_scopes(
        &self,
        server_idx: usize,
    ) -> Result<HashMap<String, crate::types::DhcpPoolSpec>, ConfigError> {
        let mut scopes = HashMap::new();
        if server_idx >= self.dhcp_servers.len() {
            return Ok(scopes);
        }
        let server = &self.dhcp_servers[server_idx];
        for pool_name in &server.pools {
            if let Some(pool) = self.dhcp_pools.iter().find(|p| p.name == *pool_name) {
                scopes.insert(pool.name.clone(), pool.clone());
            }
        }
        Ok(scopes)
    }

    pub fn build_dhcpv6_scopes(
        &self,
        server_idx: usize,
    ) -> Result<HashMap<String, crate::types::Dhcpv6PoolSpec>, ConfigError> {
        let mut scopes = HashMap::new();
        if server_idx >= self.dhcpv6_servers.len() {
            return Ok(scopes);
        }
        let server = &self.dhcpv6_servers[server_idx];
        for pool_name in &server.pools {
            if let Some(pool) = self.dhcpv6_pools.iter().find(|p| p.name == *pool_name) {
                scopes.insert(pool.name.clone(), pool.clone());
            }
        }
        Ok(scopes)
    }
}

#[derive(Debug, Clone, edgerun_error::Error)]
pub enum ConfigError {
    EmptyFile,
    ParseError(String),
    ValidationError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_NATIVE_RESOURCE_KINDS: &str = r#"
kind: DnsServer
spec:
  bind_address: "0.0.0.0:53"
---
kind: DnsZone
spec:
  origin: example.com
  soa:
    mname: ns1.example.com
    rname: admin.example.com
---
kind: DnsForwarder
spec:
  upstreams: [1.1.1.1:53, 8.8.8.8:53]
---
kind: ForwardingRule
spec:
  zone: cluster.local
  upstreams: [10.0.0.10:53]
---
kind: TlsConfig
spec:
  dot_enabled: true
---
kind: RateLimit
spec:
  qps: 25
---
kind: DhcpServer
spec:
  interface: eth0
  pools: [lan]
---
kind: DhcpPool
spec:
  name: lan
  range_start: 192.168.1.10
  range_end: 192.168.1.100
  subnet_mask: 255.255.255.0
---
kind: Dhcpv6Server
spec:
  interface: eth0
  pools: [lan6]
---
kind: Dhcpv6Pool
spec:
  name: lan6
  range_start: "fd00::10"
  range_end: "fd00::ff"
  prefix_length: 64
---
kind: TftpServer
spec:
  root_dir: /srv/tftp
---
kind: SmtpServer
spec:
  hostname: mail.example.com
---
kind: ImapServer
spec:
  hostname: mail.example.com
---
kind: Node
spec:
  roles: [worker]
---
kind: BrowserApp
spec:
  app_id: edgerun.mail
  title: Mail
  module:
    url: /apps/mail/app.wasm
  surfaces: [mail]
---
kind: BrowserNodePolicy
spec:
  name: default
  allowed_apps: [edgerun.mail]
  prompt_capabilities: [mail://edgerun.tech/*]
---
kind: Container
spec:
  image: hello-world:latest
---
kind: Deployment
spec:
  name: hello
---
kind: Secret
spec:
  secret_type: Opaque
---
kind: Peer
spec:
  node_id: node-1
---
kind: Gateway
spec:
  listeners: []
---
kind: Service
spec:
  ports: []
---
kind: HttpRoute
spec:
  hostnames: [example.com]
---
kind: TcpRoute
spec:
  port: 443
---
kind: TlsRoute
spec:
  sni_hostnames: [example.com]
"#;

    #[test]
    fn parses_all_native_config_resource_kinds_without_serde() {
        let resources = parse_config_file(ALL_NATIVE_RESOURCE_KINDS).unwrap();
        let parsed_kinds: Vec<&str> = resources.iter().map(ConfigResource::kind).collect();
        assert_eq!(resources.len(), 25, "parsed kinds: {parsed_kinds:?}");

        assert!(matches!(resources[0], ConfigResource::DnsServer(_)));
        assert!(matches!(resources[1], ConfigResource::DnsZone(_)));
        assert!(matches!(resources[2], ConfigResource::DnsForwarder(_)));
        assert!(matches!(resources[3], ConfigResource::ForwardingRule(_)));
        assert!(matches!(resources[4], ConfigResource::TlsConfig(_)));
        assert!(matches!(resources[5], ConfigResource::RateLimit(_)));
        assert!(matches!(resources[6], ConfigResource::DhcpServer(_)));
        assert!(matches!(resources[7], ConfigResource::DhcpPool(_)));
        assert!(matches!(resources[8], ConfigResource::Dhcpv6Server(_)));
        assert!(matches!(resources[9], ConfigResource::Dhcpv6Pool(_)));
        assert!(matches!(resources[10], ConfigResource::TftpServer(_)));
        assert!(matches!(resources[11], ConfigResource::SmtpServer(_)));
        assert!(matches!(resources[12], ConfigResource::ImapServer(_)));
        assert!(matches!(resources[13], ConfigResource::Node(_)));
        assert!(matches!(resources[14], ConfigResource::BrowserApp(_)));
        assert!(matches!(
            resources[15],
            ConfigResource::BrowserNodePolicy(_)
        ));
        assert!(matches!(resources[16], ConfigResource::Container(_)));
        assert!(matches!(resources[17], ConfigResource::Deployment(_)));
        assert!(matches!(resources[18], ConfigResource::Secret(_)));
        assert!(matches!(resources[19], ConfigResource::Peer(_)));
        assert!(matches!(resources[20], ConfigResource::Gateway(_)));
        assert!(matches!(resources[21], ConfigResource::Service(_)));
        assert!(matches!(resources[22], ConfigResource::HttpRoute(_)));
        assert!(matches!(resources[23], ConfigResource::TcpRoute(_)));
        assert!(matches!(resources[24], ConfigResource::TlsRoute(_)));
    }

    #[test]
    fn native_config_resources_roundtrip_through_yaml_envelopes() {
        let resources = parse_config_file(ALL_NATIVE_RESOURCE_KINDS).unwrap();
        let yaml = to_yaml_all(&resources).unwrap();
        assert!(yaml.contains("kind: DnsServer"));
        assert!(yaml.contains("spec:"));

        let reparsed = parse_config_file(&yaml).unwrap();
        assert_eq!(reparsed.len(), resources.len());
        for (original, roundtripped) in resources.iter().zip(reparsed.iter()) {
            assert_eq!(original.kind(), roundtripped.kind());
        }
    }

    #[test]
    fn parse_and_validate_uses_native_models_without_serde() {
        let state = parse_and_validate(
            r#"
kind: DhcpPool
spec:
  name: lan
  range_start: 192.168.1.10
  range_end: 192.168.1.100
  subnet_mask: 255.255.255.0
---
kind: DhcpServer
spec:
  interface: eth0
  pools: [lan]
"#,
        )
        .unwrap();

        assert_eq!(state.dhcp_pools.len(), 1);
        assert_eq!(state.dhcp_servers.len(), 1);
    }

    #[test]
    fn parses_browser_app_capability_requests() {
        let resources = parse_config_file(
            r#"
kind: BrowserApp
spec:
  app_id: edgerun.mail
  title: Mail
  module:
    url: /apps/mail/app.wasm
    sha256: abc123
  surfaces: [mail]
  required_capabilities:
    - selector: mail://edgerun.tech/*
      operations: [query, read, send]
      constraints: [require-user-presence]
"#,
        )
        .unwrap();

        let ConfigResource::BrowserApp(app) = &resources[0] else {
            panic!("expected BrowserApp");
        };
        assert_eq!(app.app_id, "edgerun.mail");
        assert_eq!(app.module.url, "/apps/mail/app.wasm");
        assert_eq!(app.module.sha256.as_deref(), Some("abc123"));
        assert_eq!(app.required_capabilities.len(), 1);
        assert_eq!(
            app.required_capabilities[0].selector,
            "mail://edgerun.tech/*"
        );
        assert_eq!(app.required_capabilities[0].operations.len(), 3);
        assert_eq!(app.required_capabilities[0].constraints.len(), 1);
    }
}
