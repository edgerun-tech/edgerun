use super::*;

/// A native realization of requested node services.
pub struct BoundNodeRuntime {
    #[cfg(feature = "http")]
    pub(super) http: Vec<HttpNodeBinding>,
    #[cfg(feature = "dns")]
    pub(super) dns: Option<Arc<crate::dns::DnsRuntime>>,
    #[cfg(feature = "dhcp")]
    pub(super) dhcp: Option<dhcp_runtime::DhcpServer>,
    #[cfg(feature = "tftp")]
    pub(super) tftp: Option<tftp_runtime::TftpServer>,
    #[cfg(feature = "imap")]
    pub(super) imap: Option<mail_runtime::ImapNodeService>,
    #[cfg(feature = "smtp")]
    pub(super) smtp: Option<mail_runtime::SmtpNodeService>,
    #[cfg(feature = "lmtp")]
    pub(super) lmtp: Option<mail_runtime::LmtpNodeService>,
    #[cfg(feature = "proxy")]
    pub(super) proxy: Option<proxy_runtime::ProxyRuntime>,
    #[cfg(all(feature = "virtual-disk", not(target_os = "none")))]
    pub(super) virtual_disks: Vec<virtual_disk_runtime::VirtualDiskRuntime>,
    /// Compiled connection middleware chain.
    /// If empty, connections go directly to protocol handlers.
    #[cfg(feature = "http")]
    #[allow(dead_code)]
    pub(super) connection_middleware: Arc<dyn ConnectionHandler>,
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
