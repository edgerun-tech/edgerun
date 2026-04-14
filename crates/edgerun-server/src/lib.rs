//! Unified multi-protocol server.
//!
//! Supports HTTP/1.1, HTTP/2, HTTP/3, DNS, DHCP, and TFTP — all with
//! shared graceful shutdown via [`CancellationToken`].
//!
//! # Example
//! ```no_run
//! use edgerun_server::Server;
//! use edgerun_http::{Response, StatusCode, into_handler};
//! use edgerun_tls::certificate_gen::generate_self_signed;
//!
//! # async fn example() -> std::io::Result<()> {
//! let handler = into_handler(|_req| {
//!     Response::text(StatusCode::new(200).unwrap(), "Hello!")
//! });
//!
//! let cert = generate_self_signed(&["localhost"]);
//! let server = Server::new()
//!     .with_http(handler, "127.0.0.1:8443")
//!     .with_tls(cert)
//!     .with_http3()
//!     .build()
//!     .await?;
//!
//! let shutdown = edgerun_rt::CancellationToken::new();
//! server.run(shutdown).await
//! # }
//! ```

use edgerun_rt::CancellationToken;
use std::sync::Arc;
use std::time::Duration;

// Re-export key types.
pub use edgerun_http::handler::Handler;
pub use edgerun_http::middleware::{Chain, Extensions, Middleware, Next, middleware_fn};
pub use edgerun_http::server::{BoundHttpServer, HttpServer, TlsCertificate};
pub use edgerun_http::{Request, Response, StatusCode};

pub mod middleware;
pub use middleware::{
    ConnectionChain, ConnectionHandler, ConnectionMiddleware, NextConnection,
    connection_fn, FnConnectionMiddleware, PassThroughHandler, MiddlewareAdapter,
    IpFilter, ConnectionLogger, ConnectionRateLimit,
};

// ---------------------------------------------------------------------------
// Optional protocol configs (gated by feature flags)
// ---------------------------------------------------------------------------

#[cfg(feature = "dns")]
mod dns_config {
    

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
pub use dns_config::DnsConfig;

#[cfg(feature = "dhcp")]
mod dhcp_config {
    
    use std::net::Ipv4Addr;

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
pub use dhcp_config::DhcpConfig;

#[cfg(feature = "tftp")]
mod tftp_config {
    use super::*;

    #[derive(Clone)]
    pub struct TftpConfig {
        pub bind_addr: String,
        pub provider: Arc<dyn edgerun_tftp::server::FileProvider>,
        pub blksize: u16,
    }
}
#[cfg(feature = "tftp")]
pub use tftp_config::TftpConfig;

#[cfg(feature = "imap")]
mod imap_config {
    use std::path::PathBuf;

    #[derive(Clone)]
    pub struct ImapConfig {
        pub bind_addr: String,
        pub domain_name: String,
        pub imaps: bool,
        /// If set, uses MaildirImapStore for persistent local mailbox storage.
        /// Must point to the same Maildir root that SMTP writes to.
        pub maildir_root: Option<PathBuf>,
    }

    impl Default for ImapConfig {
        fn default() -> Self {
            Self {
                bind_addr: "0.0.0.0:143".to_string(),
                domain_name: "edgerun.mail".to_string(),
                imaps: false,
                maildir_root: None,
            }
        }
    }
}
#[cfg(feature = "imap")]
pub use imap_config::ImapConfig;

#[cfg(feature = "smtp")]
mod smtp_config {
    use std::path::PathBuf;

    #[derive(Clone)]
    pub struct SmtpConfig {
        pub bind_addr: String,
        pub domain_name: String,
        pub max_message_size: usize,
        pub smtps: bool,
        /// Local domains for mail delivery routing.
        pub local_domains: Vec<String>,
        /// If set, enables outbound relay with persistent queue at this path.
        pub queue_data_root: Option<PathBuf>,
        /// DNS server for MX lookups in outbound relay.
        pub relay_dns_server: String,
        /// If set, uses MaildirStore for persistent local mailbox storage.
        pub maildir_root: Option<PathBuf>,
    }

    impl Default for SmtpConfig {
        fn default() -> Self {
            Self {
                bind_addr: "0.0.0.0:25".to_string(),
                domain_name: "edgerun.mail".to_string(),
                max_message_size: 35_882_577,
                smtps: false,
                local_domains: vec!["edgerun.mail".to_string()],
                queue_data_root: None,
                relay_dns_server: "8.8.8.8:53".to_string(),
                maildir_root: None,
            }
        }
    }
}
#[cfg(feature = "smtp")]
pub use smtp_config::SmtpConfig;

#[cfg(feature = "lmtp")]
mod lmtp_config {
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
pub use lmtp_config::LmtpConfig;

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------

/// Unified server builder.
pub struct Server {
    http: Option<HttpBuilder>,
    #[cfg(feature = "dns")]
    dns: Option<DnsConfig>,
    #[cfg(feature = "dhcp")]
    dhcp: Option<DhcpConfig>,
    #[cfg(feature = "tftp")]
    tftp: Option<TftpConfig>,
    #[cfg(feature = "imap")]
    imap: Option<ImapConfig>,
    #[cfg(feature = "smtp")]
    smtp: Option<SmtpConfig>,
    #[cfg(feature = "lmtp")]
    lmtp: Option<LmtpConfig>,
    connection_middleware: Vec<Arc<dyn ConnectionMiddleware>>,
}

struct HttpBuilder {
    handler: Arc<dyn Handler>,
    bind_addr: String,
    tls: Option<TlsCertificate>,
    http3: bool,
    keep_alive: Option<Duration>,
    max_request_size: usize,
}

impl Server {
    pub fn new() -> Self {
        Self {
            http: None,
            #[cfg(feature = "dns")]
            dns: None,
            #[cfg(feature = "dhcp")]
            dhcp: None,
            #[cfg(feature = "tftp")]
            tftp: None,
            #[cfg(feature = "imap")]
            imap: None,
            #[cfg(feature = "smtp")]
            smtp: None,
            #[cfg(feature = "lmtp")]
            lmtp: None,
            connection_middleware: Vec::new(),
        }
    }

    /// Add connection-level middleware that runs on every TCP connection
    /// before protocol parsing.
    ///
    /// Middleware is applied in order: first `.with_connection_middleware()`
    /// = outermost (runs first on connect).
    pub fn with_connection_middleware<M: ConnectionMiddleware>(mut self, mw: M) -> Self {
        self.connection_middleware.push(Arc::new(mw));
        self
    }

    /// Enable HTTP with the given handler and bind address.
    pub fn with_http<H: Handler>(mut self, handler: H, addr: impl std::fmt::Display) -> Self {
        self.http = Some(HttpBuilder {
            handler: Arc::new(handler),
            bind_addr: addr.to_string(),
            tls: None,
            http3: false,
            keep_alive: Some(Duration::from_secs(5)),
            max_request_size: 10 * 1024 * 1024,
        });
        self
    }

    /// Enable TLS for HTTP (required for HTTP/3).
    pub fn with_tls(mut self, cert: TlsCertificate) -> Self {
        if let Some(ref mut h) = self.http {
            h.tls = Some(cert);
        }
        self
    }

    /// Enable HTTP/3 on the same port as the TCP listener.
    pub fn with_http3(mut self) -> Self {
        if let Some(ref mut h) = self.http {
            h.http3 = true;
        }
        self
    }

    /// Enable the DNS server.
    #[cfg(feature = "dns")]
    pub fn with_dns(mut self, config: DnsConfig) -> Self {
        self.dns = Some(config);
        self
    }

    /// Enable the DHCP server.
    #[cfg(feature = "dhcp")]
    pub fn with_dhcp(mut self, config: DhcpConfig) -> Self {
        self.dhcp = Some(config);
        self
    }

    /// Enable the TFTP server.
    #[cfg(feature = "tftp")]
    pub fn with_tftp(mut self, config: TftpConfig) -> Self {
        self.tftp = Some(config);
        self
    }

    /// Enable the IMAP server.
    #[cfg(feature = "imap")]
    pub fn with_imap(mut self, config: ImapConfig) -> Self {
        self.imap = Some(config);
        self
    }

    /// Enable the SMTP server.
    #[cfg(feature = "smtp")]
    pub fn with_smtp(mut self, config: SmtpConfig) -> Self {
        self.smtp = Some(config);
        self
    }

    /// Enable the LMTP server.
    #[cfg(feature = "lmtp")]
    pub fn with_lmtp(mut self, config: LmtpConfig) -> Self {
        self.lmtp = Some(config);
        self
    }

    /// Build and bind all protocol listeners.
    pub async fn build(self) -> std::io::Result<BoundServer> {
        let http_bound = if let Some(h) = self.http {
            let mut server = HttpServer::new(h.handler);
            if let Some(ka) = h.keep_alive { server = server.keep_alive(Some(ka)); }
            server = server.max_request_size(h.max_request_size);
            if let Some(cert) = h.tls {
                server = server.with_tls(cert);
            }
            if h.http3 {
                server = server.with_http3();
            }
            Some(server.bind(&h.bind_addr).await?)
        } else {
            None
        };

        #[cfg(feature = "dns")]
        let dns_server = if let Some(config) = self.dns {
            let dns_config = edgerun_dns::DnsServerConfig {
                bind_addr: config.bind_addr,
                default_ttl: config.default_ttl,
                rate_limit_qps: config.rate_limit_qps,
                bind_addr_ipv6: config.bind_addr_ipv6,
            };
            let srv = edgerun_dns::DnsServer::new(dns_config)?;
            Some(srv)
        } else {
            None
        };

        #[cfg(feature = "dhcp")]
        let dhcp_server = if let Some(config) = self.dhcp {
            let dhcp_config = edgerun_dhcp::server::DhcpServerConfig {
                server_ip: config.server_ip,
                subnet_mask: config.subnet_mask,
                router: config.router,
                dns_servers: config.dns_servers,
                lease_time: config.lease_time,
                tftp_server: None,
                default_bootfile: None,
                bootfile_by_arch: std::collections::HashMap::new(),
            };
            let srv = edgerun_dhcp::DhcpServer::new(dhcp_config, config.pool_start, config.pool_end)?;
            Some(srv)
        } else {
            None
        };

        #[cfg(feature = "tftp")]
        let tftp_server = if let Some(config) = self.tftp {
            let tftp_config = edgerun_tftp::server::TftpServerConfig {
                bind_addr: config.bind_addr,
                default_blksize: config.blksize,
                timeout_secs: 5,
            };
            let srv = edgerun_tftp::TftpServer::new(tftp_config, config.provider)?;
            Some(srv)
        } else {
            None
        };

        #[cfg(feature = "imap")]
        let imap_server = if let Some(config) = self.imap {
            let imap_config = edgerun_email::imap::server::ImapServerConfig {
                bind_addr: config.bind_addr,
                domain_name: config.domain_name,
                imaps: config.imaps,
                ..Default::default()
            };
            let srv = if let Some(ref maildir_root) = config.maildir_root {
                // Use MaildirImapStore — reads same Maildir that SMTP writes to
                let store = edgerun_email::imap::MaildirImapStore::new(maildir_root)?;
                edgerun_email::imap::ImapServer::with_store(imap_config, std::sync::Arc::new(store))?
            } else {
                // Fallback to in-memory store
                edgerun_email::imap::ImapServer::new(imap_config)?
            };
            Some(srv)
        } else {
            None
        };

        #[cfg(feature = "smtp")]
        let smtp_server = if let Some(config) = self.smtp {
            let smtp_config = edgerun_email::smtp::server::SmtpServerConfig {
                bind_addr: config.bind_addr,
                domain: config.domain_name,
                limits: edgerun_email::smtp::ServerLimits {
                    max_message_size: config.max_message_size,
                    ..Default::default()
                },
                smtps: config.smtps,
                local_domains: config.local_domains,
                queue_data_root: config.queue_data_root,
                relay_dns_server: config.relay_dns_server,
                ..Default::default()
            };

            let srv = if let Some(ref maildir_root) = config.maildir_root {
                // Use persistent MaildirStore
                let store = edgerun_email::smtp::server::MaildirStore::new(maildir_root)?;
                let handler = std::sync::Arc::new(store);
                edgerun_email::smtp::server::SmtpServer::new(smtp_config, handler)?
            } else {
                // Fallback to in-memory store
                edgerun_email::smtp::SmtpServer::with_memory_store(smtp_config)?
            };
            Some(srv)
        } else {
            None
        };

        #[cfg(feature = "lmtp")]
        let lmtp_server = if let Some(config) = self.lmtp {
            let lmtp_config = edgerun_email::lmtp::server::LmtpServerConfig {
                bind_addr: config.bind_addr,
                domain: config.domain_name,
                limits: edgerun_email::smtp::ServerLimits {
                    max_message_size: config.max_message_size,
                    ..Default::default()
                },
            };
            let srv = edgerun_email::lmtp::LmtpServer::with_memory_store(lmtp_config)?;
            Some(srv)
        } else {
            None
        };

        // Build connection middleware chain
        let connection_middleware = if self.connection_middleware.is_empty() {
            // No middleware — use a pass-through handler
            Arc::new(PassThroughHandler) as Arc<dyn ConnectionHandler>
        } else {
            let chain = self.connection_middleware.into_iter().fold(
                ConnectionChain::new(PassThroughHandler),
                |chain, mw| chain.with(MiddlewareAdapter(mw)),
            );
            Arc::new(chain.build()) as Arc<dyn ConnectionHandler>
        };

        Ok(BoundServer {
            http: http_bound.map(Arc::new),
            #[cfg(feature = "dns")]
            dns: dns_server.map(Arc::new),
            #[cfg(feature = "dhcp")]
            dhcp: dhcp_server,
            #[cfg(feature = "tftp")]
            tftp: tftp_server,
            #[cfg(feature = "imap")]
            imap: imap_server,
            #[cfg(feature = "smtp")]
            smtp: smtp_server,
            #[cfg(feature = "lmtp")]
            lmtp: lmtp_server,
            connection_middleware,
        })
    }
}

impl Default for Server {
    fn default() -> Self { Self::new() }
}

/// A fully bound server with all protocol listeners ready.
pub struct BoundServer {
    http: Option<Arc<BoundHttpServer>>,
    #[cfg(feature = "dns")]
    dns: Option<Arc<edgerun_dns::DnsServer>>,
    #[cfg(feature = "dhcp")]
    dhcp: Option<edgerun_dhcp::DhcpServer>,
    #[cfg(feature = "tftp")]
    tftp: Option<edgerun_tftp::TftpServer>,
    #[cfg(feature = "imap")]
    imap: Option<edgerun_email::imap::ImapServer>,
    #[cfg(feature = "smtp")]
    smtp: Option<edgerun_email::smtp::SmtpServer>,
    #[cfg(feature = "lmtp")]
    lmtp: Option<edgerun_email::lmtp::LmtpServer>,
    /// Compiled connection middleware chain.
    /// If empty, connections go directly to protocol handlers.
    connection_middleware: Arc<dyn ConnectionHandler>,
}

impl BoundServer {
    /// Run all protocol listeners until `shutdown` is cancelled.
    pub async fn run(&mut self, shutdown: CancellationToken) -> std::io::Result<()> {
        let mut tasks: Vec<edgerun_rt::JoinHandle<std::io::Result<()>>> = Vec::new();

        // HTTP (TCP + optional HTTP/3 UDP)
        if let Some(ref http) = self.http {
            let token = shutdown.clone();
            let http = Arc::clone(http);
            tasks.push(edgerun_rt::spawn(async move {
                http.serve_with_shutdown(token).await
            }));
        }

        // DNS
        #[cfg(feature = "dns")]
        if let Some(ref dns) = self.dns {
            let dns_run = Arc::clone(dns);
            tasks.push(edgerun_rt::spawn(async move {
                let _ = dns_run.run().await;
                Ok(())
            }));
            let dns_shutdown = Arc::clone(dns);
            let token = shutdown.clone();
            tasks.push(edgerun_rt::spawn(async move {
                while !token.is_cancelled() {
                    edgerun_rt::sleep(Duration::from_millis(100)).await;
                }
                dns_shutdown.shutdown().await;
                Ok(())
            }));
        }

        // DHCP
        #[cfg(feature = "dhcp")]
        if let Some(dhcp) = self.dhcp.take() {
            let token = shutdown.clone();
            tasks.push(edgerun_rt::spawn(async move {
                dhcp.run(token).await;
                Ok(())
            }));
        }

        // TFTP
        #[cfg(feature = "tftp")]
        if let Some(tftp) = self.tftp.take() {
            let token = shutdown.clone();
            tasks.push(edgerun_rt::spawn(async move {
                tftp.run(token).await;
                Ok(())
            }));
        }

        // IMAP
        #[cfg(feature = "imap")]
        if let Some(imap) = self.imap.take() {
            let token = shutdown.clone();
            tasks.push(edgerun_rt::spawn(async move {
                imap.run(token).await
            }));
        }

        // SMTP
        #[cfg(feature = "smtp")]
        if let Some(smtp) = self.smtp.take() {
            let token = shutdown.clone();
            tasks.push(edgerun_rt::spawn(async move {
                smtp.run(token).await
            }));
        }

        // LMTP
        #[cfg(feature = "lmtp")]
        if let Some(lmtp) = self.lmtp.take() {
            let token = shutdown.clone();
            tasks.push(edgerun_rt::spawn(async move {
                lmtp.run(token).await
            }));
        }

        // Wait for all tasks
        for task in tasks {
            let _ = task.await;
        }

        edgerun_log::info!("All server protocols shut down");
        Ok(())
    }
}
