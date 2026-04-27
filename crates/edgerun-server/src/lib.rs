//! Unified multi-protocol server.
//!
//! Supports HTTP/1.1, HTTP/2, HTTP/3, DNS, DHCP, SMTP, IMAP, LMTP, TFTP, and Proxy
//! — all with shared graceful shutdown via [`CancellationToken`].
//!
//! For HTTP types ([`Handler`], [`Request`], [`Response`], etc.), import from
//! [`edgerun_http`] directly.
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
//! let cert = generate_self_signed(&["localhost"]).unwrap();
//! let mut server = Server::new()
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
//!
//! [`edgerun_http`]: https://docs.rs/edgerun-http

#![no_std]

extern crate alloc;

#[cfg(not(target_os = "none"))]
extern crate std;

#[cfg(any(feature = "dns", feature = "dhcp", feature = "tftp", feature = "proxy"))]
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt;
use core::module_path;
use core::time::Duration;
use edgerun_rt::CancellationToken;

#[cfg(target_os = "none")]
use edgerun_http::io;
#[cfg(not(target_os = "none"))]
use std::io;

use edgerun_http::connection_middleware::{
    ConnectionChain, ConnectionHandler, ConnectionMiddleware, MiddlewareAdapter, PassThroughHandler,
};
use edgerun_http::handler::Handler;
#[cfg(feature = "tls")]
use edgerun_http::server::TlsCertificate;
use edgerun_http::server::{BoundHttpServer, HttpServer};

#[cfg(all(
    any(feature = "imap", feature = "smtp", feature = "lmtp"),
    not(target_os = "none")
))]
pub mod connection_interceptor_adapter;
#[cfg(all(
    any(feature = "imap", feature = "smtp", feature = "lmtp"),
    not(target_os = "none")
))]
use connection_interceptor_adapter::ConnectionInterceptorAdapter;

#[cfg(any(feature = "dns", feature = "dhcp", feature = "tftp", feature = "proxy"))]
fn other_io_error(error: impl fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::Other, format!("{error}"))
}

// ---------------------------------------------------------------------------
// Optional protocol configs (gated by feature flags)
// ---------------------------------------------------------------------------
#[cfg(feature = "dns")]
mod dns_config {
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
pub use dns_config::DnsConfig;

#[cfg(feature = "dhcp")]
mod dhcp_config {

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

    impl TftpConfig {
        pub fn new(provider: Arc<dyn edgerun_tftp::server::FileProvider>) -> Self {
            Self {
                bind_addr: "0.0.0.0:69".to_string(),
                provider,
                blksize: 512,
            }
        }
    }
}
#[cfg(feature = "tftp")]
pub use tftp_config::TftpConfig;

#[cfg(feature = "imap")]
mod imap_config {
    use alloc::string::{String, ToString};
    #[cfg(target_os = "none")]
    use edgerun_http::path::PathBuf;
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
        pub tls_cert: Option<edgerun_tls::CertificateAndKey>,
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
pub use imap_config::ImapConfig;

#[cfg(feature = "smtp")]
mod smtp_config {
    use alloc::string::{String, ToString};
    use alloc::vec;
    use alloc::vec::Vec;
    #[cfg(target_os = "none")]
    use edgerun_http::path::PathBuf;
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
        pub relay_dns_server: String,
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
        pub tls_cert: Option<edgerun_tls::CertificateAndKey>,
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
                relay_dns_server: "8.8.8.8:53".to_string(),
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
pub use smtp_config::SmtpConfig;

#[cfg(feature = "lmtp")]
mod lmtp_config {
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
pub use lmtp_config::LmtpConfig;

#[cfg(feature = "proxy")]
mod proxy_config {
    use alloc::string::{String, ToString};
    use core::time::Duration;

    #[derive(Clone)]
    pub struct ProxyConfig {
        pub bind_addr: String,
        pub socks5_bind_addr: Option<String>,
        pub upstream_proxy: Option<String>,
        pub username: Option<String>,
        pub password: Option<String>,
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
                username: None,
                password: None,
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
pub use proxy_config::ProxyConfig;

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
    #[cfg(feature = "proxy")]
    proxy: Option<ProxyConfig>,
    connection_middleware: Vec<Arc<dyn ConnectionMiddleware>>,
}

struct HttpBuilder {
    handler: Arc<dyn Handler>,
    bind_addr: String,
    #[cfg(feature = "tls")]
    tls: Option<TlsCertificate>,
    #[cfg(feature = "http3")]
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
            #[cfg(feature = "proxy")]
            proxy: None,
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
    pub fn with_http<H: Handler>(mut self, handler: H, addr: impl fmt::Display) -> Self {
        self.http = Some(HttpBuilder {
            handler: Arc::new(handler),
            bind_addr: addr.to_string(),
            #[cfg(feature = "tls")]
            tls: None,
            #[cfg(feature = "http3")]
            http3: false,
            keep_alive: Some(Duration::from_secs(5)),
            max_request_size: 10 * 1024 * 1024,
        });
        self
    }

    /// Enable TLS for HTTP (required for HTTP/3).
    #[cfg(feature = "tls")]
    pub fn with_tls(mut self, cert: TlsCertificate) -> Self {
        if let Some(ref mut h) = self.http {
            h.tls = Some(cert);
        }
        self
    }

    /// Enable HTTP/3 on the same port as the TCP listener.
    #[cfg(feature = "http3")]
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

    #[cfg(feature = "proxy")]
    pub fn with_proxy(mut self, config: ProxyConfig) -> Self {
        self.proxy = Some(config);
        self
    }

    /// Build and bind all protocol listeners.
    pub async fn build(self) -> io::Result<BoundServer> {
        let http_bound = if let Some(h) = self.http {
            let mut server = HttpServer::new(h.handler);
            if let Some(ka) = h.keep_alive {
                server = server.keep_alive(Some(ka));
            }
            server = server.max_request_size(h.max_request_size);
            #[cfg(feature = "tls")]
            {
                if let Some(cert) = h.tls {
                    server = server.with_tls(cert);
                }
            }
            #[cfg(feature = "http3")]
            if h.http3 {
                server = server.with_http3();
            }
            Some(server.bind(h.bind_addr.as_str()).await?)
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
            let srv = edgerun_dns::DnsServer::new(dns_config).map_err(other_io_error)?;
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
                bootfile_by_arch: alloc::collections::BTreeMap::new(),
            };
            let srv =
                edgerun_dhcp::DhcpServer::new(dhcp_config, config.pool_start, config.pool_end)
                    .map_err(other_io_error)?;
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
            let srv = edgerun_tftp::TftpServer::new(tftp_config, config.provider)
                .map_err(other_io_error)?;
            Some(srv)
        } else {
            None
        };

        #[cfg(feature = "proxy")]
        let proxy_server = if let Some(config) = self.proxy {
            let srv = edgerun_proxy::ProxyServer::new(edgerun_proxy::ProxyConfig {
                bind_addr: config.bind_addr,
                socks5_bind_addr: config.socks5_bind_addr,
                upstream_proxy: config.upstream_proxy,
                username: config.username,
                password: config.password,
                connect_timeout: config.connect_timeout,
                tunnel_buffer_size: config.tunnel_buffer_size,
                tunnel_read_timeout: config.tunnel_read_timeout,
                tunnel_write_timeout: config.tunnel_write_timeout,
                max_connections: config.max_connections,
            });
            Some(srv)
        } else {
            None
        };

        // Build connection middleware chain
        let connection_middleware: Arc<dyn ConnectionHandler> =
            if self.connection_middleware.is_empty() {
                Arc::new(PassThroughHandler)
            } else {
                let chain = self
                    .connection_middleware
                    .into_iter()
                    .fold(ConnectionChain::new(PassThroughHandler), |chain, mw| {
                        chain.with(MiddlewareAdapter::new(mw))
                    });
                chain.build()
            };
        #[cfg(all(
            any(feature = "imap", feature = "smtp", feature = "lmtp"),
            not(target_os = "none")
        ))]
        let connection_interceptor = Arc::new(ConnectionInterceptorAdapter::new(Arc::clone(
            &connection_middleware,
        )))
            as Arc<dyn edgerun_email::server::ConnectionInterceptor>;

        #[cfg(all(feature = "imap", not(target_os = "none")))]
        let imap_server = if let Some(config) = self.imap {
            let imap_config = edgerun_email::imap::server::ImapServerConfig {
                bind_addr: config.bind_addr,
                domain_name: config.domain_name,
                imaps: config.imaps,
                #[cfg(feature = "tls")]
                tls_cert: config.tls_cert,
                ..Default::default()
            };
            let mut srv = if let Some(ref maildir_root) = config.maildir_root {
                let store = edgerun_email::imap::MaildirImapStore::new(maildir_root)?;
                edgerun_email::imap::ImapServer::with_store(imap_config, Arc::new(store))?
            } else {
                edgerun_email::imap::ImapServer::new(imap_config)?
            };
            srv = srv.with_connection_interceptor(Arc::clone(&connection_interceptor));
            Some(srv)
        } else {
            None
        };

        #[cfg(all(feature = "smtp", not(target_os = "none")))]
        let smtp_server = if let Some(config) = self.smtp {
            let smtp_config = edgerun_email::smtp::server::SmtpServerConfig {
                bind_addr: config.bind_addr,
                domain: config.domain_name,
                limits: edgerun_email::smtp::ServerLimits {
                    max_message_size: config.max_message_size,
                    ..Default::default()
                },
                smtps: config.smtps,
                starttls: config.starttls,
                local_domains: config.local_domains,
                queue_data_root: config.queue_data_root,
                relay_dns_server: config.relay_dns_server,
                #[cfg(feature = "tls")]
                tls_cert: config.tls_cert,
                ..Default::default()
            };

            let mut srv = if let Some(ref maildir_root) = config.maildir_root {
                let store = edgerun_email::smtp::server::MaildirStore::new(maildir_root)?;
                let handler = Arc::new(store);
                edgerun_email::smtp::server::SmtpServer::new(smtp_config, handler)?
            } else {
                edgerun_email::smtp::SmtpServer::with_memory_store(smtp_config)?
            };
            srv = srv.with_connection_interceptor(Arc::clone(&connection_interceptor));
            Some(srv)
        } else {
            None
        };

        #[cfg(all(feature = "lmtp", not(target_os = "none")))]
        let lmtp_server = if let Some(config) = self.lmtp {
            let lmtp_config = edgerun_email::lmtp::server::LmtpServerConfig {
                bind_addr: config.bind_addr,
                domain: config.domain_name,
                limits: edgerun_email::smtp::ServerLimits {
                    max_message_size: config.max_message_size,
                    ..Default::default()
                },
            };
            let mut srv = edgerun_email::lmtp::LmtpServer::with_memory_store(lmtp_config)?;
            srv = srv.with_connection_interceptor(Arc::clone(&connection_interceptor));
            Some(srv)
        } else {
            None
        };

        Ok(BoundServer {
            http: http_bound.map(Arc::new),
            #[cfg(feature = "dns")]
            dns: dns_server.map(Arc::new),
            #[cfg(feature = "dhcp")]
            dhcp: dhcp_server,
            #[cfg(feature = "tftp")]
            tftp: tftp_server,
            #[cfg(all(feature = "imap", not(target_os = "none")))]
            imap: imap_server,
            #[cfg(all(feature = "smtp", not(target_os = "none")))]
            smtp: smtp_server,
            #[cfg(all(feature = "lmtp", not(target_os = "none")))]
            lmtp: lmtp_server,
            #[cfg(feature = "proxy")]
            proxy: proxy_server,
            connection_middleware,
        })
    }
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
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
    #[cfg(all(feature = "imap", not(target_os = "none")))]
    imap: Option<edgerun_email::imap::ImapServer>,
    #[cfg(all(feature = "smtp", not(target_os = "none")))]
    smtp: Option<edgerun_email::smtp::SmtpServer>,
    #[cfg(all(feature = "lmtp", not(target_os = "none")))]
    lmtp: Option<edgerun_email::lmtp::LmtpServer>,
    #[cfg(feature = "proxy")]
    proxy: Option<edgerun_proxy::ProxyServer>,
    /// Compiled connection middleware chain.
    /// If empty, connections go directly to protocol handlers.
    #[allow(dead_code)]
    connection_middleware: Arc<dyn ConnectionHandler>,
}

impl BoundServer {
    /// Run all protocol listeners until `shutdown` is cancelled.
    pub async fn run(&mut self, shutdown: CancellationToken) -> io::Result<()> {
        let mut tasks: Vec<edgerun_rt::JoinHandle<io::Result<()>>> = Vec::new();

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
                let _dhcp = dhcp;
                while !token.is_cancelled() {
                    edgerun_rt::sleep(Duration::from_millis(100)).await;
                }
                Ok(())
            }));
        }

        // TFTP
        #[cfg(feature = "tftp")]
        if let Some(tftp) = self.tftp.take() {
            let token = shutdown.clone();
            tasks.push(edgerun_rt::spawn(async move {
                let tftp_token = edgerun_tftp::CancellationToken::new();
                let cancel_tftp = tftp_token.clone();
                let bridge = edgerun_rt::spawn(async move {
                    token.cancelled().await;
                    cancel_tftp.cancel();
                    Ok::<(), io::Error>(())
                });

                tftp.run(tftp_token).await;
                let _ = bridge.await;
                Ok(())
            }));
        }

        // IMAP
        #[cfg(all(feature = "imap", not(target_os = "none")))]
        if let Some(imap) = self.imap.take() {
            let token = shutdown.clone();
            tasks.push(edgerun_rt::spawn(async move { imap.run(token).await }));
        }

        // SMTP
        #[cfg(all(feature = "smtp", not(target_os = "none")))]
        if let Some(smtp) = self.smtp.take() {
            let token = shutdown.clone();
            tasks.push(edgerun_rt::spawn(async move { smtp.run(token).await }));
        }

        // LMTP
        #[cfg(all(feature = "lmtp", not(target_os = "none")))]
        if let Some(lmtp) = self.lmtp.take() {
            let token = shutdown.clone();
            tasks.push(edgerun_rt::spawn(async move { lmtp.run(token).await }));
        }

        // Proxy
        #[cfg(feature = "proxy")]
        if let Some(proxy) = self.proxy.take() {
            let token = shutdown.clone();
            tasks.push(edgerun_rt::spawn(async move {
                proxy.run(token).await.map_err(other_io_error)
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
