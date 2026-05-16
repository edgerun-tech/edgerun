use super::*;

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

    /// Request an HTTP listener routed to a verified runnable app.
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
            let dns_config = crate::dns::DnsRuntimeConfig {
                bind_addr: config.bind_addr,
                default_ttl: config.default_ttl,
                rate_limit_qps: config.rate_limit_qps,
                bind_addr_ipv6: config.bind_addr_ipv6,
            };
            let srv = crate::dns::DnsRuntime::new(dns_config).map_err(other_io_error)?;
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
