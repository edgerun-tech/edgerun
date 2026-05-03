//! Compile-time deployment policy for the standalone edgerun-server binary.
//!
//! This file is intentionally static. The deployed server must not need YAML to
//! decide who controls it or what the initial DNS/mail/HTTP surface is. A build
//! pipeline can replace these constants and produce a single-use binary for a
//! specific node.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompiledBootstrapPolicy {
    pub node_label: &'static str,
    pub stream_id: [u8; 32],
    pub controller_id: [u8; 64],
    pub bootstrap_relays: &'static [&'static str],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompiledMailbox {
    pub address: &'static str,
    pub target: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompiledAlias {
    pub address: &'static str,
    pub target: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompiledWebsite {
    pub domain: &'static str,
    pub repo: &'static str,
    pub commit: &'static str,
    pub path: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompiledDomain {
    pub domain: &'static str,
    pub authoritative_dns: bool,
    pub mailboxes: &'static [CompiledMailbox],
    pub aliases: &'static [CompiledAlias],
    pub website: Option<CompiledWebsite>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompiledDeployment {
    pub policy: CompiledBootstrapPolicy,
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
    pub acme_contact: &'static str,
    pub domains: &'static [CompiledDomain],
    pub external_cnames: &'static [(&'static str, &'static str)],
}

impl CompiledDeployment {
    pub fn certificate_domains(&self) -> Vec<String> {
        let mut out = Vec::new();
        for domain in self.domains {
            if domain.mail_enabled() {
                out.push(format!("{}.{}", self.mail_host_prefix, domain.domain));
            }
            if let Some(site) = domain.website {
                out.push(site.domain.to_string());
            }
            out.push(format!("mta-sts.{}", domain.domain));
        }
        out.sort();
        out.dedup();
        out
    }
}

impl CompiledDomain {
    pub fn mail_enabled(&self) -> bool {
        !self.mailboxes.is_empty() || !self.aliases.is_empty()
    }
}

pub const CONTROLLER_PUBLIC_KEY_PLACEHOLDER: [u8; 64] = [0; 64];

pub const DEPLOYMENT: CompiledDeployment = CompiledDeployment {
    policy: CompiledBootstrapPolicy {
        node_label: "edgerun-tech-main-server",
        stream_id: [0xED; 32],
        // Replace at build/provision time with the controller public key. The
        // type and path are now fixed; the value is the only remaining input.
        controller_id: CONTROLLER_PUBLIC_KEY_PLACEHOLDER,
        bootstrap_relays: &["wss://edgerun.tech/.well-known/edgerun/bootstrap"],
    },
    public_ipv4: [172, 245, 67, 49],
    hostname: "mail.edgerun.tech",
    origin: "edgerun.tech",
    mail_host_prefix: "mail",
    maildir_root: "/var/lib/edgerun/mail/maildirs",
    queue_data_root: "/var/lib/edgerun/mail/queue",
    dkim_domain: "edgerun.tech",
    dkim_selector: "mail",
    dkim_key_path: "/etc/edgerun/server/dkim-mail.private.pem",
    tls_cert_path: "/etc/edgerun/server/tls/fullchain.pem",
    tls_key_path: "/etc/edgerun/server/tls/privkey.pem",
    acme_account_key_path: "/etc/edgerun/server/acme-account.pem",
    acme_contact: "ken@edgerun.tech",
    domains: &[
        CompiledDomain {
            domain: "edgerun.tech",
            authoritative_dns: true,
            mailboxes: &[
                CompiledMailbox { address: "ken@edgerun.tech", target: "ken@edgerun.tech" },
                CompiledMailbox { address: "admin@edgerun.tech", target: "admin@edgerun.tech" },
                CompiledMailbox { address: "dmarc-reports@edgerun.tech", target: "dmarc-reports@edgerun.tech" },
                CompiledMailbox { address: "tls-reports@edgerun.tech", target: "tls-reports@edgerun.tech" },
            ],
            aliases: &[
                CompiledAlias { address: "postmaster@edgerun.tech", target: "ken@edgerun.tech" },
                CompiledAlias { address: "abuse@edgerun.tech", target: "ken@edgerun.tech" },
            ],
            website: Some(CompiledWebsite {
                domain: "edgerun.tech",
                repo: "Sylchi/edge-vercel-front",
                commit: "main",
                path: "/",
            }),
        },
        CompiledDomain {
            domain: "nodes.edgerun.tech",
            authoritative_dns: true,
            mailboxes: &[],
            aliases: &[],
            website: None,
        },
    ],
    external_cnames: &[
        ("blog.edgerun.tech", "sylchi.github.io"),
    ],
};
