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
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::Duration;

// Re-export key types.
pub use edgerun_http::handler::Handler;
pub use edgerun_http::middleware::{Chain, Extensions, Middleware, Next, middleware_fn};
pub use edgerun_http::server::{BoundHttpServer, HttpServer, TlsCertificate};
pub use edgerun_http::{Request, Response, StatusCode};

/// Configuration for the DNS server component.
#[derive(Debug, Clone)]
pub struct DnsConfig {
    pub bind_addr: String,
    /// Optional IPv6 bind address for dual-stack DNS support.
    /// Set to "[::]:53" to listen on all IPv6 interfaces.
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

/// Configuration for the DHCP server component.
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

/// Configuration for the TFTP server component.
#[derive(Clone)]
pub struct TftpConfig {
    pub bind_addr: String,
    pub provider: Arc<dyn edgerun_tftp::server::FileProvider>,
    pub blksize: u16,
}

/// Configuration for the IMAP server component.
#[derive(Clone)]
pub struct ImapConfig {
    pub bind_addr: String,
    pub domain_name: String,
    /// Whether this is an IMAPS server (TLS from start, port 993).
    pub imaps: bool,
}

impl Default for ImapConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:143".to_string(),
            domain_name: "edgerun.mail".to_string(),
            imaps: false,
        }
    }
}

/// Configuration for the SMTP server component.
#[derive(Clone)]
pub struct SmtpConfig {
    pub bind_addr: String,
    pub domain_name: String,
    /// Maximum message size in bytes (0 = unlimited).
    pub max_message_size: usize,
    /// Whether this is an SMTPS server (TLS from start, port 465).
    pub smtps: bool,
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:25".to_string(),
            domain_name: "edgerun.mail".to_string(),
            max_message_size: 35_882_577,
            smtps: false,
        }
    }
}

/// Unified server builder.
pub struct Server {
    http: Option<HttpBuilder>,
    dns: Option<DnsConfig>,
    dhcp: Option<DhcpConfig>,
    tftp: Option<TftpConfig>,
    imap: Option<ImapConfig>,
    smtp: Option<SmtpConfig>,
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
            dns: None,
            dhcp: None,
            tftp: None,
            imap: None,
            smtp: None,
        }
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
    pub fn with_dns(mut self, config: DnsConfig) -> Self {
        self.dns = Some(config);
        self
    }

    /// Enable the DHCP server.
    pub fn with_dhcp(mut self, config: DhcpConfig) -> Self {
        self.dhcp = Some(config);
        self
    }

    /// Enable the TFTP server.
    pub fn with_tftp(mut self, config: TftpConfig) -> Self {
        self.tftp = Some(config);
        self
    }

    /// Enable the IMAP server.
    pub fn with_imap(mut self, config: ImapConfig) -> Self {
        self.imap = Some(config);
        self
    }

    /// Enable the SMTP server.
    pub fn with_smtp(mut self, config: SmtpConfig) -> Self {
        self.smtp = Some(config);
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

        let imap_server = if let Some(config) = self.imap {
            let imap_config = edgerun_imap::server::ImapServerConfig {
                bind_addr: config.bind_addr,
                domain_name: config.domain_name,
                imaps: config.imaps,
                ..Default::default()
            };
            let srv = edgerun_imap::ImapServer::new(imap_config)?;
            Some(srv)
        } else {
            None
        };

        let smtp_server = if let Some(config) = self.smtp {
            let smtp_config = edgerun_smtp::server::SmtpServerConfig {
                bind_addr: config.bind_addr,
                domain: config.domain_name,
                max_message_size: config.max_message_size,
                smtps: config.smtps,
            };
            let srv = edgerun_smtp::SmtpServer::with_memory_store(smtp_config)?;
            Some(srv)
        } else {
            None
        };

        Ok(BoundServer {
            http: http_bound.map(Arc::new),
            dns: dns_server.map(Arc::new),
            dhcp: dhcp_server,
            tftp: tftp_server,
            imap: imap_server,
            smtp: smtp_server,
        })
    }
}

impl Default for Server {
    fn default() -> Self { Self::new() }
}

/// A fully bound server with all protocol listeners ready.
pub struct BoundServer {
    http: Option<Arc<BoundHttpServer>>,
    dns: Option<Arc<edgerun_dns::DnsServer>>,
    dhcp: Option<edgerun_dhcp::DhcpServer>,
    tftp: Option<edgerun_tftp::TftpServer>,
    imap: Option<edgerun_imap::ImapServer>,
    smtp: Option<edgerun_smtp::SmtpServer>,
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

        // DHCP (now async!)
        if let Some(dhcp) = self.dhcp.take() {
            let token = shutdown.clone();
            tasks.push(edgerun_rt::spawn(async move {
                dhcp.run(token).await;
                Ok(())
            }));
        }

        // TFTP (now async!)
        if let Some(tftp) = self.tftp.take() {
            let token = shutdown.clone();
            tasks.push(edgerun_rt::spawn(async move {
                tftp.run(token).await;
                Ok(())
            }));
        }

        // IMAP (async TCP server)
        if let Some(imap) = self.imap.take() {
            let token = shutdown.clone();
            tasks.push(edgerun_rt::spawn(async move {
                imap.run(token).await
            }));
        }

        // SMTP (async TCP server)
        if let Some(smtp) = self.smtp.take() {
            let token = shutdown.clone();
            tasks.push(edgerun_rt::spawn(async move {
                smtp.run(token).await
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
