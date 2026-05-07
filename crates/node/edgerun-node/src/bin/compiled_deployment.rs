//! Compile-time deployment policy for the standalone edgerun-server binary.
//!
//! This file is intentionally static. The deployed server must not need YAML to
//! decide who controls it or what the initial DNS/mail/HTTP surface is. A build
//! pipeline can replace these constants and produce a single-use binary for a
//! specific node.

pub use edgerun_node::runtime::{
    RuntimeAliasSpec as CompiledAlias, RuntimeBootstrapPolicy as CompiledBootstrapPolicy,
    RuntimeDeploymentSpec as CompiledDeployment, RuntimeDomainSpec as CompiledDomain,
    RuntimeMailboxSpec as CompiledMailbox, RuntimeWebsiteSpec as CompiledWebsite,
};

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
    runtime_root: "/var/lib/edgerun/.edgerun",
    derived_db_path: "/var/lib/edgerun/.edgerun/runtime.edb",
    acme_contact: "ken@edgerun.tech",
    domains: &[
        CompiledDomain {
            domain: "edgerun.tech",
            authoritative_dns: true,
            mailboxes: &[
                CompiledMailbox {
                    address: "ken@edgerun.tech",
                    target: "ken@edgerun.tech",
                },
                CompiledMailbox {
                    address: "admin@edgerun.tech",
                    target: "admin@edgerun.tech",
                },
                CompiledMailbox {
                    address: "dmarc-reports@edgerun.tech",
                    target: "dmarc-reports@edgerun.tech",
                },
                CompiledMailbox {
                    address: "tls-reports@edgerun.tech",
                    target: "tls-reports@edgerun.tech",
                },
            ],
            aliases: &[
                CompiledAlias {
                    address: "postmaster@edgerun.tech",
                    target: "ken@edgerun.tech",
                },
                CompiledAlias {
                    address: "abuse@edgerun.tech",
                    target: "ken@edgerun.tech",
                },
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
    external_cnames: &[("blog.edgerun.tech", "sylchi.github.io")],
};

fn main() {}
