//! Node-owned runtime services.
//!
//! Protocol modules and apps can request HTTP, DNS, DHCP, SMTP, IMAP, LMTP,
//! TFTP, and proxy service bindings. The node owns the actual resources and
//! decides whether those requests become native sockets, browser message
//! routes, mesh routes, or no binding on the current host.
//!
//! HTTP handlers live inside apps. The node owns the listener and routes HTTP
//! requests to the target app over node IPC.
//!
//! # Example
//! ```no_run
//! use edgerun_node::services::NodeRuntime;
//! # async fn example() -> std::io::Result<()> {
//! let mut runtime = NodeRuntime::new()
//!     .with_http_app("127.0.0.1:8080", [7; 32])
//!     .build()
//!     .await?;
//!
//! let shutdown = crate::rt::CancellationToken::new();
//! runtime.run(shutdown).await
//! # }
//! ```
//!
use crate::rt::CancellationToken;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt;
use core::module_path;
use core::time::Duration;

#[cfg(target_os = "none")]
use crate::rt::io;
#[cfg(not(target_os = "none"))]
use std::io;

#[cfg(feature = "http")]
use self::http_runtime::HttpNodeBinding;
#[cfg(feature = "http")]
use self::middleware::{
    ConnectionChain, ConnectionHandler, ConnectionMiddleware, MiddlewareAdapter, PassThroughHandler,
};
#[cfg(all(feature = "http", feature = "tls"))]
use edgerun_protocols::tls::CertificateAndKey as TlsCertificate;

#[cfg(any(feature = "http", feature = "imap", feature = "smtp", feature = "lmtp"))]
use crate::transport::{HostSocketTransport, TransportAddress};

#[cfg(feature = "acme")]
pub mod acme_runtime;
pub mod config;
#[cfg(feature = "dhcp")]
pub mod dhcp_runtime;
#[cfg(feature = "dns")]
pub mod dns_runtime;
#[cfg(feature = "http")]
pub mod http_runtime;
#[cfg(any(feature = "imap", feature = "smtp", feature = "lmtp"))]
pub mod mail_runtime;
#[cfg(feature = "http")]
pub mod middleware;
#[cfg(feature = "proxy")]
mod proxy_runtime;
#[cfg(feature = "tftp")]
pub mod tftp_runtime;
#[cfg(all(feature = "virtual-disk", not(target_os = "none")))]
pub mod virtual_disk_runtime;
pub use crate::resource::{
    binding_intents, decide_binding, decide_bindings, NodeTransportSurface, ServiceBindingDecision,
    ServiceBindingIntent,
};
pub use config::*;
#[cfg(any(
    feature = "http",
    feature = "dns",
    feature = "dhcp",
    feature = "tftp",
    feature = "proxy"
))]
fn other_io_error(error: impl fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::Other, format!("{error}"))
}

#[cfg(any(feature = "http", feature = "imap", feature = "smtp", feature = "lmtp"))]
fn bind_node_tcp_listener(addr: &str) -> io::Result<crate::rt::AsyncTcpListener> {
    HostSocketTransport
        .bind_stream_now(&TransportAddress::host_stream(addr.as_bytes().to_vec()))
        .map_err(other_io_error)
}

// ---------------------------------------------------------------------------
// NodeRuntime
// ---------------------------------------------------------------------------

/// Node runtime service builder.
///
/// The builder records service requests. Calling [`build`](Self::build) is the
/// point where this native host runtime tries to realize those requests as
/// local resources. Browser, mesh, and embedded hosts can use the same service
/// plan as routing metadata without binding native ports.
pub struct NodeRuntime {
    #[cfg(feature = "http")]
    http: Vec<HttpBuilder>,
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
    #[cfg(all(feature = "virtual-disk", not(target_os = "none")))]
    virtual_disks: Vec<virtual_disk_runtime::VirtualDiskBinding>,
    #[cfg(feature = "http")]
    connection_middleware: Vec<Arc<dyn ConnectionMiddleware>>,
}

#[cfg(feature = "http")]
struct HttpBuilder {
    bind_addr: String,
    target_app_id: [u8; 32],
    #[cfg(feature = "tls")]
    tls: Option<TlsCertificate>,
    #[cfg(feature = "http3")]
    http3: bool,
}

impl NodeRuntime {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "http")]
            http: Vec::new(),
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
            #[cfg(all(feature = "virtual-disk", not(target_os = "none")))]
            virtual_disks: Vec::new(),
            #[cfg(feature = "http")]
            connection_middleware: Vec::new(),
        }
    }

    /// Add connection-level middleware that runs on every TCP connection
    /// before protocol parsing.
    ///
    /// Middleware is applied in order: first `.with_connection_middleware()`
    /// = outermost (runs first on connect).
    #[cfg(feature = "http")]
    pub fn with_connection_middleware<M: ConnectionMiddleware>(mut self, mw: M) -> Self {
        self.connection_middleware.push(Arc::new(mw));
        self
    }

    /// Request an HTTP listener routed to an installed app.
    ///
    /// The node owns the listener. The target app owns HTTP handling behind
    /// node IPC and never receives a port or socket directly.
    #[cfg(feature = "http")]
    pub fn with_http_app(mut self, addr: impl fmt::Display, target_app_id: [u8; 32]) -> Self {
        self.http.push(HttpBuilder {
            bind_addr: addr.to_string(),
            target_app_id,
            #[cfg(feature = "tls")]
            tls: None,
            #[cfg(feature = "http3")]
            http3: false,
        });
        self
    }

    /// Enable TLS for HTTP (required for HTTP/3).
    #[cfg(all(feature = "http", feature = "tls"))]
    pub fn with_tls(mut self, cert: TlsCertificate) -> Self {
        if let Some(h) = self.http.last_mut() {
            h.tls = Some(cert);
        }
        self
    }

    /// Enable HTTP/3 on the same port as the TCP listener.
    #[cfg(all(feature = "http", feature = "http3"))]
    pub fn with_http3(mut self) -> Self {
        if let Some(h) = self.http.last_mut() {
            h.http3 = true;
        }
        self
    }

    /// Request DNS service.
    #[cfg(feature = "dns")]
    pub fn with_dns(mut self, config: DnsConfig) -> Self {
        self.dns = Some(config);
        self
    }

    /// Request DHCP service.
    #[cfg(feature = "dhcp")]
    pub fn with_dhcp(mut self, config: DhcpConfig) -> Self {
        self.dhcp = Some(config);
        self
    }

    /// Request TFTP service.
    #[cfg(feature = "tftp")]
    pub fn with_tftp(mut self, config: TftpConfig) -> Self {
        self.tftp = Some(config);
        self
    }

    /// Request IMAP service.
    #[cfg(feature = "imap")]
    pub fn with_imap(mut self, config: ImapConfig) -> Self {
        self.imap = Some(config);
        self
    }

    /// Request SMTP service.
    #[cfg(feature = "smtp")]
    pub fn with_smtp(mut self, config: SmtpConfig) -> Self {
        self.smtp = Some(config);
        self
    }

    /// Request LMTP service.
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

    /// Request a node-owned virtual block listener.
    ///
    /// The node owns the native listener and passes accepted streams into the
    /// deterministic virtual disk block session handler. Apps never receive
    /// the socket directly.
    #[cfg(all(feature = "virtual-disk", not(target_os = "none")))]
    pub fn with_virtual_block_tcp<B>(mut self, addr: impl fmt::Display, backend: Arc<B>) -> Self
    where
        B: edgerun_protocols::block::BlockBackend + Send + Sync + 'static,
    {
        self.virtual_disks
            .push(virtual_disk_runtime::VirtualDiskBinding::block_tcp(
                addr.to_string(),
                backend,
            ));
        self
    }

    /// Request a node-owned NBD listener backed by virtual disk exports.
    #[cfg(all(feature = "virtual-disk", not(target_os = "none")))]
    pub fn with_virtual_nbd_tcp(
        mut self,
        addr: impl fmt::Display,
        exports: Vec<edgerun_virtual_disk::NbdExportEntry>,
    ) -> Self {
        self.virtual_disks
            .push(virtual_disk_runtime::VirtualDiskBinding::nbd_tcp(
                addr.to_string(),
                exports,
            ));
        self
    }

    /// Realize requested services for the current native host.
    pub async fn build(self) -> io::Result<BoundNodeRuntime> {
        #[cfg(feature = "http")]
        let http_bound = self
            .http
            .into_iter()
            .map(|h| {
                #[cfg(feature = "tls")]
                {
                    let _ = h.tls;
                }
                #[cfg(feature = "http3")]
                let _ = h.http3;
                HttpNodeBinding::bind(&h.bind_addr, h.target_app_id)
            })
            .collect::<io::Result<Vec<_>>>()?;

        #[cfg(feature = "dns")]
        let dns_server = if let Some(config) = self.dns {
            let dns_config = dns_runtime::DnsRuntimeConfig {
                bind_addr: config.bind_addr,
                default_ttl: config.default_ttl,
                rate_limit_qps: config.rate_limit_qps,
                bind_addr_ipv6: config.bind_addr_ipv6,
            };
            let srv = dns_runtime::DnsRuntime::new(dns_config).map_err(other_io_error)?;
            Some(srv)
        } else {
            None
        };

        #[cfg(feature = "dhcp")]
        let dhcp_server = if let Some(config) = self.dhcp {
            let dhcp_config = edgerun_protocols::dhcp::DhcpServerConfig {
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
                dhcp_runtime::DhcpServer::new(dhcp_config, config.pool_start, config.pool_end)
                    .map_err(other_io_error)?;
            Some(srv)
        } else {
            None
        };

        #[cfg(feature = "tftp")]
        let tftp_server = if let Some(config) = self.tftp {
            let tftp_config = tftp_runtime::TftpServerConfig {
                bind_addr: config.bind_addr,
                default_blksize: config.blksize,
                timeout_secs: 5,
            };
            let srv = tftp_runtime::TftpServer::new(tftp_config, config.provider)
                .map_err(other_io_error)?;
            Some(srv)
        } else {
            None
        };

        #[cfg(feature = "proxy")]
        let proxy_server = if let Some(config) = self.proxy {
            let srv = proxy_runtime::ProxyRuntime::new(proxy_runtime::ProxyRuntimeConfig {
                bind_addr: config.bind_addr,
                socks5_bind_addr: config.socks5_bind_addr,
                upstream_proxy: config.upstream_proxy,
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
        #[cfg(feature = "http")]
        let connection_middleware: Arc<dyn ConnectionHandler> =
            if self.connection_middleware.is_empty() {
                Arc::new(PassThroughHandler)
            } else {
                let chain = self
                    .connection_middleware
                    .into_iter()
                    .fold(ConnectionChain::new(PassThroughHandler), |chain, mw| {
                        chain.with(MiddlewareAdapter(mw))
                    });
                Arc::new(chain.build())
            };
        #[cfg(feature = "imap")]
        let imap_server = if let Some(config) = self.imap {
            let srv = mail_runtime::ImapNodeService::new(mail_runtime::ImapNodeConfig {
                bind_addr: config.bind_addr.clone(),
                domain_name: config.domain_name,
                starttls: config.imaps || cfg!(feature = "tls"),
            })?;
            let _ = config.maildir_root;
            #[cfg(feature = "tls")]
            let _ = config.tls_cert;
            Some(srv)
        } else {
            None
        };

        #[cfg(feature = "smtp")]
        let smtp_server = if let Some(config) = self.smtp {
            let srv = mail_runtime::SmtpNodeService::new(mail_runtime::SmtpNodeConfig {
                bind_addr: config.bind_addr.clone(),
                domain_name: config.domain_name,
                max_message_size: config.max_message_size,
                starttls: config.starttls,
                local_domains: config.local_domains,
                queue_available: config.queue_data_root.is_some(),
            })?;
            let _ = config.smtps;
            let _ = config.relay_dns_server;
            let _ = config.maildir_root;
            let _ = config.dkim_domain;
            let _ = config.dkim_selector;
            let _ = config.dkim_key_path;
            #[cfg(feature = "tls")]
            let _ = config.tls_cert;
            Some(srv)
        } else {
            None
        };

        #[cfg(feature = "lmtp")]
        let lmtp_server = if let Some(config) = self.lmtp {
            let srv = mail_runtime::LmtpNodeService::new(mail_runtime::LmtpNodeConfig {
                bind_addr: config.bind_addr.clone(),
                domain_name: config.domain_name,
                max_message_size: config.max_message_size,
            })?;
            Some(srv)
        } else {
            None
        };

        #[cfg(all(feature = "virtual-disk", not(target_os = "none")))]
        let virtual_disks = self
            .virtual_disks
            .into_iter()
            .map(virtual_disk_runtime::VirtualDiskRuntime::bind)
            .collect::<io::Result<Vec<_>>>()?;

        Ok(BoundNodeRuntime {
            #[cfg(feature = "http")]
            http: http_bound,
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
            #[cfg(feature = "proxy")]
            proxy: proxy_server,
            #[cfg(all(feature = "virtual-disk", not(target_os = "none")))]
            virtual_disks,
            #[cfg(feature = "http")]
            connection_middleware,
        })
    }
}

impl Default for NodeRuntime {
    fn default() -> Self {
        Self::new()
    }
}

/// A native realization of requested node services.
pub struct BoundNodeRuntime {
    #[cfg(feature = "http")]
    http: Vec<HttpNodeBinding>,
    #[cfg(feature = "dns")]
    dns: Option<Arc<dns_runtime::DnsRuntime>>,
    #[cfg(feature = "dhcp")]
    dhcp: Option<dhcp_runtime::DhcpServer>,
    #[cfg(feature = "tftp")]
    tftp: Option<tftp_runtime::TftpServer>,
    #[cfg(feature = "imap")]
    imap: Option<mail_runtime::ImapNodeService>,
    #[cfg(feature = "smtp")]
    smtp: Option<mail_runtime::SmtpNodeService>,
    #[cfg(feature = "lmtp")]
    lmtp: Option<mail_runtime::LmtpNodeService>,
    #[cfg(feature = "proxy")]
    proxy: Option<proxy_runtime::ProxyRuntime>,
    #[cfg(all(feature = "virtual-disk", not(target_os = "none")))]
    virtual_disks: Vec<virtual_disk_runtime::VirtualDiskRuntime>,
    /// Compiled connection middleware chain.
    /// If empty, connections go directly to protocol handlers.
    #[cfg(feature = "http")]
    #[allow(dead_code)]
    connection_middleware: Arc<dyn ConnectionHandler>,
}

impl BoundNodeRuntime {
    /// Run all realized native services until `shutdown` is cancelled.
    pub async fn run(&mut self, shutdown: CancellationToken) -> io::Result<()> {
        let mut tasks: Vec<crate::rt::JoinHandle<io::Result<()>>> = Vec::new();

        // HTTP (TCP + optional HTTP/3 UDP)
        #[cfg(feature = "http")]
        for http in self.http.drain(..) {
            let token = shutdown.clone();
            tasks.push(crate::rt::spawn(async move { http.run(token).await }));
        }

        // DNS
        #[cfg(feature = "dns")]
        if let Some(ref dns) = self.dns {
            let dns_run = Arc::clone(dns);
            let token = shutdown.clone();
            tasks.push(crate::rt::spawn(async move {
                dns_run.run(token).await.map_err(other_io_error)?;
                Ok(())
            }));
        }

        // DHCP
        #[cfg(feature = "dhcp")]
        if let Some(dhcp) = self.dhcp.take() {
            let token = shutdown.clone();
            tasks.push(crate::rt::spawn(async move {
                dhcp.run(token).await.map_err(other_io_error)?;
                Ok(())
            }));
        }

        // TFTP
        #[cfg(feature = "tftp")]
        if let Some(tftp) = self.tftp.take() {
            let token = shutdown.clone();
            tasks.push(crate::rt::spawn(async move {
                let tftp_token = crate::rt::CancellationToken::new();
                let cancel_tftp = tftp_token.clone();
                let bridge = crate::rt::spawn(async move {
                    token.cancelled().await;
                    cancel_tftp.cancel();
                    Ok::<(), io::Error>(())
                });

                tftp.run(tftp_token).await.map_err(other_io_error)?;
                match bridge.await {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => return Err(error),
                    Err(error) => return Err(join_error(error)),
                }
                Ok(())
            }));
        }

        // IMAP
        #[cfg(feature = "imap")]
        if let Some(imap) = self.imap.take() {
            let token = shutdown.clone();
            tasks.push(crate::rt::spawn(async move { imap.run(token).await }));
        }

        // SMTP
        #[cfg(feature = "smtp")]
        if let Some(smtp) = self.smtp.take() {
            let token = shutdown.clone();
            tasks.push(crate::rt::spawn(async move { smtp.run(token).await }));
        }

        // LMTP
        #[cfg(feature = "lmtp")]
        if let Some(lmtp) = self.lmtp.take() {
            let token = shutdown.clone();
            tasks.push(crate::rt::spawn(async move { lmtp.run(token).await }));
        }

        // Proxy
        #[cfg(feature = "proxy")]
        if let Some(proxy) = self.proxy.take() {
            let token = shutdown.clone();
            tasks.push(crate::rt::spawn(async move {
                proxy.run(token).await.map_err(other_io_error)
            }));
        }

        #[cfg(all(feature = "virtual-disk", not(target_os = "none")))]
        for virtual_disk in self.virtual_disks.drain(..) {
            let token = shutdown.clone();
            tasks.push(crate::rt::spawn(
                async move { virtual_disk.run(token).await },
            ));
        }

        // Wait for all tasks
        for task in tasks {
            match task.await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => return Err(error),
                Err(error) => return Err(join_error(error)),
            }
        }

        crate::node_info!("all node runtime services shut down");
        Ok(())
    }
}

fn join_error(error: crate::rt::JoinError) -> io::Error {
    let _ = error;
    io::Error::new(io::ErrorKind::Other, "node service task failed")
}
