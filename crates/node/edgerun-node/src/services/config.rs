//! Service configuration types for node-owned protocol runtimes.

#[cfg(feature = "dns")]
mod dns {
    use alloc::string::{String, ToString};

    #[derive(Debug, Clone)]
    pub struct DnsConfig {
        pub bind_addr: String,
        pub bind_addr_ipv6: Option<String>,
        pub default_ttl: u32,
        pub rate_limit_qps: u32,
    }

    impl Default for DnsConfig {
        fn default() -> Self {
            Self {
                bind_addr: "127.0.0.1:53".to_string(),
                bind_addr_ipv6: None,
                default_ttl: 300,
                rate_limit_qps: 100,
            }
        }
    }
}
#[cfg(feature = "dns")]
pub use dns::DnsConfig;

#[cfg(feature = "dhcp")]
mod dhcp {
    use alloc::vec;
    use alloc::vec::Vec;
    use core::net::Ipv4Addr;

    #[derive(Debug, Clone)]
    pub struct DhcpConfig {
        pub server_ip: Ipv4Addr,
        pub subnet_mask: Ipv4Addr,
        pub router: Ipv4Addr,
        pub dns_servers: Vec<Ipv4Addr>,
        pub lease_time: u32,
        pub pool_start: Ipv4Addr,
        pub pool_end: Ipv4Addr,
    }

    impl DhcpConfig {
        pub fn new(
            server_ip: Ipv4Addr,
            subnet_mask: Ipv4Addr,
            router: Ipv4Addr,
            pool_start: Ipv4Addr,
            pool_end: Ipv4Addr,
        ) -> Self {
            Self {
                server_ip,
                subnet_mask,
                router,
                dns_servers: vec![server_ip],
                lease_time: 3600,
                pool_start,
                pool_end,
            }
        }
    }
}
#[cfg(feature = "dhcp")]
pub use dhcp::DhcpConfig;

#[cfg(feature = "tftp")]
mod tftp {
    use alloc::string::{String, ToString};
    use alloc::sync::Arc;

    #[derive(Clone)]
    pub struct TftpConfig {
        pub bind_addr: String,
        pub provider: Arc<dyn crate::services::tftp_runtime::FileProvider>,
        pub blksize: u16,
    }

    impl TftpConfig {
        pub fn new(provider: Arc<dyn crate::services::tftp_runtime::FileProvider>) -> Self {
            Self {
                bind_addr: "0.0.0.0:69".to_string(),
                provider,
                blksize: 512,
            }
        }
    }
}
#[cfg(feature = "tftp")]
pub use tftp::TftpConfig;

#[cfg(feature = "imap")]
mod imap {
    use alloc::string::{String, ToString};
    #[cfg(target_os = "none")]
    type PathBuf = String;
    #[cfg(not(target_os = "none"))]
    use std::path::PathBuf;

    #[derive(Clone)]
    pub struct ImapConfig {
        pub bind_addr: String,
        pub domain_name: String,
        pub imaps: bool,
        /// If set, uses MaildirImapStore for persistent local mailbox storage.
        /// Must point to the same Maildir root that SMTP writes to.
        pub maildir_root: Option<PathBuf>,
        /// TLS certificate and key for IMAPS/STARTTLS.
        #[cfg(feature = "tls")]
        pub tls_cert: Option<edgerun_protocols::tls::CertificateAndKey>,
    }

    impl Default for ImapConfig {
        fn default() -> Self {
            Self {
                bind_addr: "0.0.0.0:143".to_string(),
                domain_name: "edgerun.mail".to_string(),
                imaps: false,
                maildir_root: None,
                #[cfg(feature = "tls")]
                tls_cert: None,
            }
        }
    }
}
#[cfg(feature = "imap")]
pub use imap::ImapConfig;

#[cfg(feature = "smtp")]
mod smtp {
    use alloc::string::{String, ToString};
    use alloc::vec;
    use alloc::vec::Vec;
    #[cfg(target_os = "none")]
    type PathBuf = String;
    #[cfg(not(target_os = "none"))]
    use std::path::PathBuf;

    #[derive(Clone)]
    pub struct SmtpConfig {
        pub bind_addr: String,
        pub domain_name: String,
        pub max_message_size: usize,
        pub smtps: bool,
        pub starttls: bool,
        /// Local domains for mail delivery routing.
        pub local_domains: Vec<String>,
        /// If set, enables outbound relay with persistent queue at this path.
        pub queue_data_root: Option<PathBuf>,
        /// DNS server for MX lookups in outbound relay.
        pub relay_dns_server: Option<String>,
        /// If set, uses MaildirStore for persistent local mailbox storage.
        pub maildir_root: Option<PathBuf>,
        /// DKIM signing domain.
        pub dkim_domain: Option<String>,
        /// DKIM selector.
        pub dkim_selector: Option<String>,
        /// Path to DKIM private key.
        pub dkim_key_path: Option<PathBuf>,
        /// TLS certificate and key for SMTPS/STARTTLS.
        #[cfg(feature = "tls")]
        pub tls_cert: Option<edgerun_protocols::tls::CertificateAndKey>,
    }

    impl Default for SmtpConfig {
        fn default() -> Self {
            Self {
                bind_addr: "0.0.0.0:25".to_string(),
                domain_name: "edgerun.mail".to_string(),
                max_message_size: 35_882_577,
                smtps: false,
                starttls: true,
                local_domains: vec!["edgerun.mail".to_string()],
                queue_data_root: None,
                relay_dns_server: None,
                maildir_root: None,
                dkim_domain: None,
                dkim_selector: None,
                dkim_key_path: None,
                #[cfg(feature = "tls")]
                tls_cert: None,
            }
        }
    }
}
#[cfg(feature = "smtp")]
pub use smtp::SmtpConfig;

#[cfg(feature = "lmtp")]
mod lmtp {
    use alloc::string::{String, ToString};

    #[derive(Clone)]
    pub struct LmtpConfig {
        pub bind_addr: String,
        pub domain_name: String,
        pub max_message_size: usize,
    }

    impl Default for LmtpConfig {
        fn default() -> Self {
            Self {
                bind_addr: "0.0.0.0:24".to_string(),
                domain_name: "edgerun.mail".to_string(),
                max_message_size: 35_882_577,
            }
        }
    }
}
#[cfg(feature = "lmtp")]
pub use lmtp::LmtpConfig;

#[cfg(feature = "proxy")]
mod proxy {
    use alloc::string::{String, ToString};
    use core::time::Duration;

    #[derive(Clone)]
    pub struct ProxyConfig {
        pub bind_addr: String,
        pub socks5_bind_addr: Option<String>,
        pub upstream_proxy: Option<String>,
        pub connect_timeout: Duration,
        pub tunnel_buffer_size: usize,
        pub tunnel_read_timeout: Duration,
        pub tunnel_write_timeout: Duration,
        pub max_connections: Option<usize>,
    }

    impl Default for ProxyConfig {
        fn default() -> Self {
            Self {
                bind_addr: "0.0.0.0:8080".to_string(),
                socks5_bind_addr: None,
                upstream_proxy: None,
                connect_timeout: Duration::from_secs(30),
                tunnel_buffer_size: 64 * 1024,
                tunnel_read_timeout: Duration::from_secs(60),
                tunnel_write_timeout: Duration::from_secs(60),
                max_connections: None,
            }
        }
    }
}
#[cfg(feature = "proxy")]
pub use proxy::ProxyConfig;
