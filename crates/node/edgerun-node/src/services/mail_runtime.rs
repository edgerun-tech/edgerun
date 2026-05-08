use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::net::SocketAddr;

use crate::network::{HostSocketTransport, TransportAddress};
use crate::rt::{
    self, AsyncReadExt, AsyncTcpListener, AsyncTcpStream, AsyncWriteExt, CancellationToken,
};
#[cfg(feature = "imap")]
use edgerun_protocols::imap::{
    ImapPeerContext, ImapResponse, ImapSessionAction, ImapSessionConfig, ImapSessionCore,
    RejectAllImapPolicy,
};
#[cfg(feature = "lmtp")]
use edgerun_protocols::lmtp::{
    AllowAllLmtpPolicy, LmtpSessionAction, LmtpSessionConfig, LmtpSessionCore,
};
#[cfg(feature = "smtp")]
use edgerun_protocols::smtp::{
    AllowAllSmtpPolicy, ServerLimits, SmtpPeerContext, SmtpResponse, SmtpSessionAction,
    SmtpSessionConfig, SmtpSessionCore,
};
#[cfg(all(feature = "lmtp", not(feature = "smtp")))]
use edgerun_protocols::smtp::{ServerLimits, SmtpResponse};

#[cfg(target_os = "none")]
use crate::rt::io;
#[cfg(not(target_os = "none"))]
use std::io;

#[cfg(feature = "smtp")]
#[derive(Clone)]
pub struct SmtpNodeConfig {
    pub bind_addr: String,
    pub domain_name: String,
    pub max_message_size: usize,
    pub starttls: bool,
    pub local_domains: Vec<String>,
    pub queue_available: bool,
}

#[cfg(feature = "imap")]
#[derive(Clone)]
pub struct ImapNodeConfig {
    pub bind_addr: String,
    pub domain_name: String,
    pub starttls: bool,
}

#[cfg(feature = "lmtp")]
#[derive(Clone)]
pub struct LmtpNodeConfig {
    pub bind_addr: String,
    pub domain_name: String,
    pub max_message_size: usize,
}

#[cfg(feature = "smtp")]
pub struct SmtpNodeService {
    listener: Arc<AsyncTcpListener>,
    config: SmtpNodeConfig,
}

#[cfg(feature = "imap")]
pub struct ImapNodeService {
    listener: Arc<AsyncTcpListener>,
    config: ImapNodeConfig,
}

#[cfg(feature = "lmtp")]
pub struct LmtpNodeService {
    listener: Arc<AsyncTcpListener>,
    config: LmtpNodeConfig,
}

#[cfg(feature = "smtp")]
impl SmtpNodeService {
    pub fn new(config: SmtpNodeConfig) -> io::Result<Self> {
        Ok(Self {
            listener: Arc::new(bind_node_listener(&config.bind_addr)?),
            config,
        })
    }

    pub async fn run(self, shutdown: CancellationToken) -> io::Result<()> {
        run_accept_loop(self.listener, shutdown, move |stream, peer| {
            let config = self.config.clone();
            async move { run_smtp_session(stream, peer, config).await }
        })
        .await
    }
}

#[cfg(feature = "imap")]
impl ImapNodeService {
    pub fn new(config: ImapNodeConfig) -> io::Result<Self> {
        Ok(Self {
            listener: Arc::new(bind_node_listener(&config.bind_addr)?),
            config,
        })
    }

    pub async fn run(self, shutdown: CancellationToken) -> io::Result<()> {
        run_accept_loop(self.listener, shutdown, move |stream, _peer| {
            let config = self.config.clone();
            async move { run_imap_session(stream, config).await }
        })
        .await
    }
}

#[cfg(feature = "lmtp")]
impl LmtpNodeService {
    pub fn new(config: LmtpNodeConfig) -> io::Result<Self> {
        Ok(Self {
            listener: Arc::new(bind_node_listener(&config.bind_addr)?),
            config,
        })
    }

    pub async fn run(self, shutdown: CancellationToken) -> io::Result<()> {
        run_accept_loop(self.listener, shutdown, move |stream, _peer| {
            let config = self.config.clone();
            async move { run_lmtp_session(stream, config).await }
        })
        .await
    }
}

async fn run_accept_loop<F, Fut>(
    listener: Arc<AsyncTcpListener>,
    shutdown: CancellationToken,
    handler: F,
) -> io::Result<()>
where
    F: Fn(Arc<AsyncTcpStream>, SocketAddr) -> Fut + Send + Sync + 'static,
    Fut: core::future::Future<Output = io::Result<()>> + Send + 'static,
{
    let handler = Arc::new(handler);
    while !shutdown.is_cancelled() {
        match rt::timeout(core::time::Duration::from_millis(100), listener.accept()).await {
            Ok(Ok((stream, peer))) => {
                let handler = Arc::clone(&handler);
                rt::spawn(async move {
                    if let Err(error) = handler(stream, peer).await {
                        crate::node_warn!("mail session failed: {}", error);
                    }
                });
            }
            Ok(Err(error)) => return Err(io_error(error)),
            Err(_) => {}
        }
    }
    Ok(())
}

#[cfg(feature = "smtp")]
async fn run_smtp_session(
    mut stream: Arc<AsyncTcpStream>,
    _peer: SocketAddr,
    config: SmtpNodeConfig,
) -> io::Result<()> {
    let mut limits = ServerLimits::default();
    limits.max_message_size = config.max_message_size;
    let session_config = SmtpSessionConfig {
        domain: config.domain_name,
        limits,
        local_domains: config.local_domains,
        auth_mechanisms: Vec::new(),
        require_auth: false,
        tls_configured: config.starttls,
        starttls_available: config.starttls,
        queue_available: config.queue_available,
    };
    let mut core = SmtpSessionCore::new(
        session_config,
        SmtpPeerContext {
            trusted_submitter: false,
            tls_active: false,
        },
    );
    write_smtp_response(&mut stream, core.greeting()).await?;

    while let Some(line) = read_line(&mut stream).await? {
        let step = core.handle_line(&line, &AllowAllSmtpPolicy);
        for response in step.responses {
            write_smtp_response(&mut stream, response).await?;
        }
        match step.action {
            SmtpSessionAction::Continue => {}
            SmtpSessionAction::Quit => break,
            SmtpSessionAction::StartTls => {
                write_smtp_response(
                    &mut stream,
                    SmtpResponse::command_not_implemented("STARTTLS transport upgrade"),
                )
                .await?;
            }
            SmtpSessionAction::ReadBdat { .. } => {
                write_smtp_response(
                    &mut stream,
                    SmtpResponse::command_not_implemented("BDAT transport chunking"),
                )
                .await?;
            }
            SmtpSessionAction::Deliver => {
                crate::node_info!(
                    "smtp accepted message bytes={} recipients={}",
                    core.envelope.data.len(),
                    core.envelope.recipient_count()
                );
                write_smtp_response(&mut stream, SmtpResponse::ok("Queued by edgerun-node"))
                    .await?;
            }
        }
    }
    Ok(())
}

#[cfg(feature = "imap")]
async fn run_imap_session(
    mut stream: Arc<AsyncTcpStream>,
    config: ImapNodeConfig,
) -> io::Result<()> {
    let mut core = ImapSessionCore::new(
        ImapSessionConfig {
            domain_name: config.domain_name,
            tls_configured: config.starttls,
            starttls_available: config.starttls,
        },
        ImapPeerContext { tls_active: false },
    );
    write_imap_response(&mut stream, core.greeting()).await?;

    while let Some(line) = read_line(&mut stream).await? {
        let step = core.handle_line(&line, &RejectAllImapPolicy);
        for response in step.responses {
            write_imap_response(&mut stream, response).await?;
        }
        match step.action {
            ImapSessionAction::Continue => {}
            ImapSessionAction::Logout => break,
            ImapSessionAction::StartTls => {
                write_imap_response(
                    &mut stream,
                    ImapResponse::bad("*", "STARTTLS transport upgrade is node policy"),
                )
                .await?;
            }
            ImapSessionAction::Authenticate { .. } | ImapSessionAction::AdapterCommand { .. } => {
                write_imap_response(
                    &mut stream,
                    ImapResponse::no("*", "mailbox storage is an app capability"),
                )
                .await?;
            }
        }
    }
    Ok(())
}

#[cfg(feature = "lmtp")]
async fn run_lmtp_session(
    mut stream: Arc<AsyncTcpStream>,
    config: LmtpNodeConfig,
) -> io::Result<()> {
    let mut limits = ServerLimits::default();
    limits.max_message_size = config.max_message_size;
    let mut core = LmtpSessionCore::new(LmtpSessionConfig {
        domain: config.domain_name,
        limits,
    });
    write_smtp_response(&mut stream, core.greeting()).await?;

    while let Some(line) = read_line(&mut stream).await? {
        let step = core.handle_line(&line, &AllowAllLmtpPolicy);
        for response in step.responses {
            write_smtp_response(&mut stream, response).await?;
        }
        match step.action {
            LmtpSessionAction::Continue => {}
            LmtpSessionAction::Quit => break,
            LmtpSessionAction::Deliver => {
                let delivered = core
                    .envelope
                    .recipients
                    .iter()
                    .map(|_| true)
                    .collect::<Vec<_>>();
                for response in core.recipient_delivery_responses(delivered) {
                    write_smtp_response(&mut stream, response).await?;
                }
                core.reset_transaction();
            }
        }
    }
    Ok(())
}

async fn read_line(stream: &mut Arc<AsyncTcpStream>) -> io::Result<Option<String>> {
    let mut bytes = Vec::new();
    let mut buf = [0u8; 1];
    loop {
        let n = stream.read(&mut buf).await.map_err(io_error)?;
        if n == 0 {
            if bytes.is_empty() {
                return Ok(None);
            }
            break;
        }
        bytes.push(buf[0]);
        if buf[0] == b'\n' {
            break;
        }
        if bytes.len() > 8192 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "line too long"));
        }
    }
    while bytes.last().copied() == Some(b'\n') || bytes.last().copied() == Some(b'\r') {
        bytes.pop();
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "line is not utf-8"))
}

#[cfg(any(feature = "smtp", feature = "lmtp"))]
async fn write_smtp_response(
    stream: &mut Arc<AsyncTcpStream>,
    response: SmtpResponse,
) -> io::Result<()> {
    stream
        .write_all(response.format().as_bytes())
        .await
        .map_err(io_error)
}

#[cfg(feature = "imap")]
async fn write_imap_response(
    stream: &mut Arc<AsyncTcpStream>,
    response: ImapResponse,
) -> io::Result<()> {
    stream
        .write_all(response.to_wire().as_bytes())
        .await
        .map_err(io_error)
}

fn bind_node_listener(addr: &str) -> io::Result<AsyncTcpListener> {
    HostSocketTransport
        .bind_stream_now(&TransportAddress::host_stream(addr.as_bytes().to_vec()))
        .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))
}

fn io_error(error: rt::IoError) -> io::Error {
    match error {
        rt::IoError::UnexpectedEof => io::Error::new(io::ErrorKind::UnexpectedEof, error),
        rt::IoError::WriteZero => io::Error::new(io::ErrorKind::WriteZero, error),
        rt::IoError::Other(_) => io::Error::new(io::ErrorKind::Other, error),
    }
}
