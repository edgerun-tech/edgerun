use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use edgerun_protocols::wire::{
    RuntimeAppInstall, RuntimeCapabilityDeclaration, RuntimeDeploymentConfig, RuntimeDomainConfig,
    RuntimeProtocolBinding, ROUTE_SCHEME_HTTPS, RUNTIME_PROTOCOL_ACME, RUNTIME_PROTOCOL_DNS_TCP,
    RUNTIME_PROTOCOL_DNS_UDP, SDK_WIRE_ABI_VERSION,
};

use crate::network::{binding_intents, NodeTransportSurface, ServiceBindingIntent};

use super::{
    default_protocol_bindings, runtime_app_install, runtime_deployment_config, runtime_http_route,
    sha256,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeServicePlan {
    pub runtime_id: [u8; 32],
    pub public_ipv4: [u8; 4],
    pub hostname: Vec<u8>,
    pub origin: Vec<u8>,
    /// Requested protocol bindings.
    ///
    /// These are not necessarily native listeners. The node uses them as
    /// routing/resource intent and chooses the host-specific realization.
    pub listeners: Vec<RuntimeProtocolBinding>,
    pub domains: Vec<RuntimeDomainConfig>,
    pub apps: Vec<RuntimeAppInstall>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeBootstrapPolicy {
    pub node_label: &'static str,
    pub stream_id: [u8; 32],
    pub controller_id: [u8; 64],
    pub bootstrap_relays: &'static [&'static str],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeMailboxSpec {
    pub address: &'static str,
    pub target: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeAliasSpec {
    pub address: &'static str,
    pub target: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeWebsiteSpec {
    pub domain: &'static str,
    pub repo: &'static str,
    pub commit: &'static str,
    pub path: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeDomainSpec {
    pub domain: &'static str,
    pub authoritative_dns: bool,
    pub mailboxes: &'static [RuntimeMailboxSpec],
    pub aliases: &'static [RuntimeAliasSpec],
    pub website: Option<RuntimeWebsiteSpec>,
}

impl RuntimeDomainSpec {
    pub fn mail_enabled(&self) -> bool {
        !self.mailboxes.is_empty() || !self.aliases.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeDeploymentSpec {
    pub policy: RuntimeBootstrapPolicy,
    pub public_ipv4: [u8; 4],
    pub hostname: &'static str,
    pub origin: &'static str,
    pub mail_host_prefix: &'static str,
    pub maildir_root: &'static str,
    pub queue_data_root: &'static str,
    pub dkim_domain: &'static str,
    pub dkim_selector: &'static str,
    pub dkim_key_path: &'static str,
    pub tls_cert_path: &'static str,
    pub tls_key_path: &'static str,
    pub acme_account_key_path: &'static str,
    pub runtime_root: &'static str,
    pub derived_db_path: &'static str,
    pub acme_contact: &'static str,
    pub domains: &'static [RuntimeDomainSpec],
    pub external_cnames: &'static [(&'static str, &'static str)],
}

impl RuntimeDeploymentSpec {
    pub fn certificate_domains(&self) -> Vec<String> {
        let mut out = Vec::new();
        for domain in self.domains {
            if domain.mail_enabled() {
                out.push(self.mail_host_for_domain(domain.domain));
            }
            if let Some(site) = domain.website {
                out.push(site.domain.to_string());
            }
            out.push(format_bytes(b"mta-sts.", domain.domain.as_bytes()));
        }
        out.sort();
        out.dedup();
        out
    }

    pub fn local_mail_domains(&self) -> Vec<String> {
        let mut domains = Vec::new();
        for domain in self.domains {
            if domain.mail_enabled() || domain.domain == self.origin {
                domains.push(domain.domain.to_string());
            }
        }
        domains.sort();
        domains.dedup();
        domains
    }

    pub fn mail_host_for_domain(&self, domain: &str) -> String {
        if self.mail_host_prefix.is_empty() {
            domain.to_string()
        } else {
            let mut out = String::new();
            out.push_str(self.mail_host_prefix);
            out.push('.');
            out.push_str(domain);
            out
        }
    }

    pub fn to_wire_config(&self) -> RuntimeDeploymentConfig {
        let mut domains = Vec::new();
        let mut apps = Vec::new();
        for domain in self.domains {
            let mut routes = Vec::new();
            if let Some(site) = domain.website {
                let app_id = sha256(site.domain.as_bytes());
                let release_id = sha256(site.commit.as_bytes());
                routes.push(runtime_http_route(
                    app_id,
                    release_id,
                    ROUTE_SCHEME_HTTPS,
                    site.domain.as_bytes().to_vec(),
                    site.path.as_bytes().to_vec(),
                ));
                apps.push(runtime_app_install(
                    app_id,
                    release_id,
                    sha256(site.repo.as_bytes()),
                    self.policy.stream_id,
                    sha256(site.path.as_bytes()),
                    routes.clone(),
                    vec![site.domain.as_bytes().to_vec()],
                ));
            }
            let mut mailboxes = Vec::new();
            for mailbox in domain.mailboxes {
                mailboxes.push(edgerun_protocols::wire::RuntimeMailbox {
                    abi_version: SDK_WIRE_ABI_VERSION,
                    flags: 1,
                    address: mailbox.address.as_bytes().to_vec(),
                    target_app_id: sha256(mailbox.target.as_bytes()),
                });
            }
            domains.push(RuntimeDomainConfig {
                abi_version: SDK_WIRE_ABI_VERSION,
                flags: 1,
                domain: domain.domain.as_bytes().to_vec(),
                authoritative_dns: domain.authoritative_dns,
                mail_enabled: domain.mail_enabled(),
                acme_enabled: domain.authoritative_dns,
                mailboxes,
                routes,
            });
        }
        runtime_deployment_config(
            self.policy.stream_id,
            self.public_ipv4,
            self.hostname.as_bytes().to_vec(),
            self.origin.as_bytes().to_vec(),
            self.acme_contact.as_bytes().to_vec(),
            default_protocol_bindings([0, 0, 0, 0]),
            domains,
            apps,
        )
    }
}

impl RuntimeServicePlan {
    pub fn from_deployment(config: &RuntimeDeploymentConfig) -> Self {
        Self::from_deployment_with_apps(config, Vec::new())
    }

    pub fn from_deployment_with_apps(
        config: &RuntimeDeploymentConfig,
        extra_apps: Vec<RuntimeAppInstall>,
    ) -> Self {
        let mut listeners = config.protocol_bindings.clone();
        if listeners.is_empty() {
            listeners = default_protocol_bindings([0, 0, 0, 0]);
        }
        listeners.sort_by_key(|binding| (binding.protocol, binding.port, binding.bind_ipv4));
        listeners.dedup_by(|a, b| {
            a.protocol == b.protocol
                && a.port == b.port
                && a.bind_ipv4 == b.bind_ipv4
                && a.host == b.host
        });
        let mut apps = config.apps.clone();
        for app in extra_apps {
            if !apps.iter().any(|existing| existing.app_id == app.app_id) {
                apps.push(app);
            }
        }
        Self {
            runtime_id: config.runtime_id,
            public_ipv4: config.public_ipv4,
            hostname: config.hostname.clone(),
            origin: config.origin.clone(),
            listeners,
            domains: config.domains.clone(),
            apps,
        }
    }

    pub fn requires_dns(&self) -> bool {
        self.listeners.iter().any(|binding| {
            binding.protocol == RUNTIME_PROTOCOL_DNS_UDP
                || binding.protocol == RUNTIME_PROTOCOL_DNS_TCP
        })
    }

    pub fn requires_acme(&self) -> bool {
        self.domains.iter().any(|domain| domain.acme_enabled)
            || self
                .listeners
                .iter()
                .any(|binding| binding.protocol == RUNTIME_PROTOCOL_ACME)
    }

    pub fn mail_domains(&self) -> Vec<Vec<u8>> {
        let mut domains: Vec<Vec<u8>> = self
            .domains
            .iter()
            .filter(|domain| domain.mail_enabled)
            .map(|domain| domain.domain.clone())
            .collect();
        domains.sort();
        domains.dedup();
        domains
    }

    pub fn requested_bindings(&self, surface: NodeTransportSurface) -> Vec<ServiceBindingIntent> {
        binding_intents(&self.listeners, surface)
    }

    pub fn provided_capabilities(&self) -> Vec<RuntimeCapabilityDeclaration> {
        let mut capabilities = Vec::new();
        for app in &self.apps {
            capabilities.extend(app.provided_capabilities.iter().cloned());
        }
        capabilities
    }

    pub fn required_capabilities(&self) -> Vec<RuntimeCapabilityDeclaration> {
        let mut capabilities = Vec::new();
        for app in &self.apps {
            capabilities.extend(app.required_capabilities.iter().cloned());
        }
        capabilities
    }
}

fn format_bytes(prefix: &[u8], suffix: &[u8]) -> String {
    let mut out = String::new();
    out.push_str(core::str::from_utf8(prefix).unwrap_or(""));
    out.push_str(core::str::from_utf8(suffix).unwrap_or(""));
    out
}
