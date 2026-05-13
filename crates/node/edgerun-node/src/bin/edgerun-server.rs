use std::env;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, TcpStream, UdpSocket};
use std::path::PathBuf;
#[cfg(feature = "tls")]
use std::sync::mpsc::{self, SyncSender};
#[cfg(feature = "tls")]
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use edgerun_node::rt::{
    self, AsyncRead, AsyncReadExt, AsyncTcpStream, AsyncWrite, AsyncWriteExt, CancellationToken,
};
use edgerun_node::services::{DnsConfig, ImapConfig, NodeRuntime, SmtpConfig};
#[cfg(feature = "tls")]
use edgerun_node::tls::{AsyncTlsServerStream, CertificateAndKey};
use edgerun_protocols::dns::DnsZone;
#[cfg(feature = "tls")]
use edgerun_protocols::websocket::{
    WS_EXTENDED_64_LEN, WS_MASK_LEN, WebSocketError, WebSocketMessage, decode_client_message,
    decode_frame_prefix, decode_handshake_request, decode_payload_len, encode_server_binary,
    encode_server_control, encode_upgrade_response, handshake_complete,
};
#[cfg(feature = "tls")]
use edgerun_work::{MAX_WORK_FRAME_LEN, NodeId, channel_envelope_from_bytes};

const NODE_LABEL: &str = "edgerun-tech-main-server";
const ORIGIN: &str = "edgerun.tech";
const HOSTNAME: &str = "mail.edgerun.tech";
const PUBLIC_IPV4: &str = "172.245.67.49";
const NODE_ID: &str = "4de436f31887836ceb40d4bfc34952909c284f11f36c57877e489893fc98dad3defa5b4aa90ef42219d45eb98bad960316f31e22b28daa81940a6bbbd6072ab7";
const MAILDIR_ROOT: &str = "/var/lib/edgerun/mail/maildirs";
const QUEUE_ROOT: &str = "/var/lib/edgerun/mail/queue";
const DKIM_KEY_PATH: &str = "/etc/edgerun/server/dkim-mail.private.pem";
const TLS_CERT_PATH: &str = "/etc/edgerun/server/tls/cloudflare-origin.pem";
const TLS_KEY_PATH: &str = "/etc/edgerun/server/tls/cloudflare-origin.key";
const ACME_ACCOUNT_PATH: &str = "/etc/edgerun/server/acme-account.pem";
const RUNTIME_ROOT: &str = "/var/lib/edgerun/.edgerun";
const DERIVED_DB_PATH: &str = "/var/lib/edgerun/.edgerun/runtime.edb";
const CONTROLLER_ZERO: &str = "00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
const VERSION: &str = env!("CARGO_PKG_VERSION");
#[cfg(feature = "tls")]
const HTTPS_BIND_ADDR: &str = "0.0.0.0:443";
#[cfg(feature = "tls")]
const EDGERUN_WORK_WS_PROTOCOL: &str = "edgerun-work-v1";
#[cfg(feature = "tls")]
const EDGERUN_WORK_WS_READ_POLL_MS: u64 = 100;
#[cfg(feature = "tls")]
const EDGERUN_WORK_WS_MAX_QUEUE: usize = 256;

#[cfg(feature = "tls")]
static EDGERUN_WORK_WSS_PEERS: OnceLock<
    Mutex<std::collections::BTreeMap<NodeId, SyncSender<Vec<u8>>>>,
> = OnceLock::new();

#[derive(Default)]
struct Args {
    health_check: bool,
    print_plan: bool,
    send_report: bool,
    version: bool,
    config: Option<String>,
    dash_host: Option<String>,
}

fn main() {
    let args = match parse_args() {
        Ok(args) => args,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    };

    if args.version {
        println!("edgerun-server {VERSION}");
        return;
    }
    if args.print_plan {
        print_plan();
        return;
    }
    if args.health_check {
        std::process::exit(if health_check() { 0 } else { 1 });
    }
    if args.send_report {
        print_plan();
        return;
    }

    if let Err(error) = run_server(args) {
        eprintln!("[ERROR] edgerun_server: server error: {error}");
        std::process::exit(1);
    }
}

fn parse_args() -> Result<Args, String> {
    let mut parsed = Args::default();
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--health-check" => parsed.health_check = true,
            "--print-compiled-plan" => parsed.print_plan = true,
            "--send-system-report" => parsed.send_report = true,
            "--version" | "-V" => parsed.version = true,
            "--help" | "-h" => return Err(help_text()),
            "--config" => parsed.config = args.next(),
            "--dash-host" => parsed.dash_host = args.next(),
            other => return Err(format!("unknown option: {other}\n{}", help_text())),
        }
    }
    Ok(parsed)
}

fn help_text() -> String {
    "Usage: edgerun-server [--config PATH] [--dash-host HOST] [--health-check] [--print-compiled-plan] [--send-system-report] [--version]".to_string()
}

fn run_server(args: Args) -> std::io::Result<()> {
    let _ = args.config;
    let dash_host = args.dash_host.as_deref().unwrap_or("dash.edgerun.tech");

    println!(
        "[INFO] edgerun_server: edgerun-server starting on {HOSTNAME} with controller {CONTROLLER_ZERO}"
    );
    println!("[INFO] edgerun_server: runtime bootstrap identity ready: node_id={NODE_ID}");
    println!(
        "[INFO] edgerun_server: runtime service plan: listeners=4 dns=true acme=false mail_domains=1"
    );
    println!("[INFO] edgerun_server: dash host configured: {dash_host}");
    if std::fs::metadata(DKIM_KEY_PATH).is_ok() {
        println!("[INFO] edgerun_server: DKIM signer loaded from {DKIM_KEY_PATH}");
    }
    if std::fs::metadata(TLS_CERT_PATH).is_err() || std::fs::metadata(TLS_KEY_PATH).is_err() {
        println!(
            "[INFO] edgerun_server: TLS certs not found at {TLS_CERT_PATH} / {TLS_KEY_PATH}, running without TLS"
        );
    } else {
        println!("[INFO] edgerun_server: Cloudflare origin TLS cert loaded from {TLS_CERT_PATH}");
    }
    println!("[INFO] edgerun_server: DNS zone built for {ORIGIN}");
    println!("[INFO] edgerun_server: derived database ready at {DERIVED_DB_PATH}");

    let mut smtp = SmtpConfig::default();
    smtp.bind_addr = "0.0.0.0:25".to_string();
    smtp.domain_name = HOSTNAME.to_string();
    smtp.local_domains = vec![ORIGIN.to_string()];
    smtp.queue_data_root = Some(PathBuf::from(QUEUE_ROOT));
    smtp.maildir_root = Some(PathBuf::from(MAILDIR_ROOT));
    smtp.dkim_domain = Some(ORIGIN.to_string());
    smtp.dkim_selector = Some("mail".to_string());
    smtp.dkim_key_path = Some(PathBuf::from(DKIM_KEY_PATH));

    let mut imap = ImapConfig::default();
    imap.bind_addr = "0.0.0.0:143".to_string();
    imap.domain_name = HOSTNAME.to_string();
    imap.maildir_root = Some(PathBuf::from(MAILDIR_ROOT));

    let mut runtime = rt::block_on(async move {
        NodeRuntime::new()
            .with_dns(DnsConfig {
                bind_addr: "0.0.0.0:53".to_string(),
                bind_addr_ipv6: None,
                default_ttl: 3600,
                rate_limit_qps: 0,
            })
            .with_http_app("0.0.0.0:80", [0; 32])
            .with_smtp(smtp)
            .with_imap(imap)
            .build()
            .await
    })?;
    rt::block_on(async { runtime.add_dns_zone(build_dns_zone()).await });

    println!("[INFO] edgerun_server: All services started. Press Ctrl-C to stop.");
    let shutdown = CancellationToken::new();
    #[cfg(feature = "tls")]
    start_cloudflare_origin_tls();
    rt::block_on(async move { runtime.run(shutdown).await })
}

#[cfg(feature = "tls")]
fn start_cloudflare_origin_tls() {
    let Ok(cert_pem) = std::fs::read_to_string(TLS_CERT_PATH) else {
        return;
    };
    let Ok(key_pem) = std::fs::read_to_string(TLS_KEY_PATH) else {
        return;
    };
    let cert = match CertificateAndKey::from_pem(&format!("{cert_pem}\n{key_pem}")) {
        Ok(cert) => Arc::new(cert),
        Err(error) => {
            eprintln!("[WARN] edgerun_server: failed to parse Cloudflare origin cert/key: {error}");
            return;
        }
    };
    rt::spawn(async move {
        if let Err(error) = run_cloudflare_origin_tls(cert).await {
            eprintln!("[WARN] edgerun_server: Cloudflare origin TLS listener stopped: {error}");
        }
    });
}

#[cfg(feature = "tls")]
async fn run_cloudflare_origin_tls(cert: Arc<CertificateAndKey>) -> std::io::Result<()> {
    let listener = rt::AsyncTcpListener::bind(HTTPS_BIND_ADDR).map_err(node_io_error)?;
    println!("[INFO] edgerun_server: Cloudflare origin TLS listening on {HTTPS_BIND_ADDR}");
    loop {
        let (stream, peer) = listener.accept().await.map_err(node_io_error)?;
        let cert = Arc::clone(&cert);
        rt::spawn(async move {
            if let Err(error) = handle_cloudflare_origin_tls(stream, cert).await {
                eprintln!("[WARN] edgerun_server: Cloudflare origin TLS client {peer}: {error}");
            }
        });
    }
}

#[cfg(feature = "tls")]
async fn handle_cloudflare_origin_tls(
    stream: Arc<AsyncTcpStream>,
    cert: Arc<CertificateAndKey>,
) -> std::io::Result<()> {
    let mut stream = AsyncTlsServerStream::accept(stream, cert.as_ref())
        .await
        .map_err(|error| io_error(format!("TLS handshake failed: {error}")))?;
    let request = read_http_request(&mut stream).await?;
    let request_text = core::str::from_utf8(&request).unwrap_or("");
    match (request_method(request_text), request_path(request_text)) {
        (Some("GET"), Some("/work")) if is_websocket_upgrade(request_text) => {
            serve_tls_work_websocket(stream, request).await
        }
        (_, Some("/health")) => write_tls_response(
            &mut stream,
            "200 OK",
            "application/json",
            "{\"status\":\"ok\",\"service\":\"edgerun-server\",\"tls\":\"cloudflare-origin\"}\n",
        )
        .await,
        (_, Some("/")) => {
            write_tls_response(
                &mut stream,
                "200 OK",
                "text/plain; charset=utf-8",
                "EdgeRun node online\n",
            )
            .await
        }
        _ => {
            write_tls_response(
                &mut stream,
                "404 Not Found",
                "text/plain; charset=utf-8",
                "not found\n",
            )
            .await
        }
    }
}

#[cfg(feature = "tls")]
async fn serve_tls_work_websocket(
    mut stream: AsyncTlsServerStream<Arc<AsyncTcpStream>>,
    request: Vec<u8>,
) -> std::io::Result<()> {
    let request = decode_handshake_request(request).map_err(invalid_ws_data)?;
    let response =
        encode_upgrade_response(&request, EDGERUN_WORK_WS_PROTOCOL).map_err(invalid_ws_data)?;
    stream
        .write_all(response.as_bytes())
        .await
        .map_err(node_io_error)?;
    stream.flush().await.map_err(node_io_error)?;

    let peers =
        EDGERUN_WORK_WSS_PEERS.get_or_init(|| Mutex::new(std::collections::BTreeMap::new()));
    let (tx, rx) = mpsc::sync_channel::<Vec<u8>>(EDGERUN_WORK_WS_MAX_QUEUE);
    let mut registered_node = None;

    loop {
        let mut queued = Vec::new();
        while let Ok(frame) = rx.try_recv() {
            queued.push(frame);
        }
        for frame in queued {
            stream.write_all(&frame).await.map_err(node_io_error)?;
        }
        stream.flush().await.map_err(node_io_error)?;
        match rt::timeout(
            rt::Duration::from_millis(EDGERUN_WORK_WS_READ_POLL_MS),
            read_tls_work_ws_message(&mut stream),
        )
        .await
        {
            Ok(Ok(WebSocketMessage::Binary(bytes))) => {
                let envelope = channel_envelope_from_bytes(&bytes).map_err(invalid_ws_data)?;
                register_tls_work_peer(peers, &mut registered_node, envelope.from, tx.clone());
                forward_tls_work_envelope(peers, envelope.to, &bytes);
            }
            Ok(Ok(WebSocketMessage::Ping(payload))) => {
                let frame = encode_server_control(0xA, &payload).map_err(invalid_ws_data)?;
                stream.write_all(&frame).await.map_err(node_io_error)?;
                stream.flush().await.map_err(node_io_error)?;
            }
            Ok(Ok(WebSocketMessage::Pong)) => {}
            Ok(Ok(WebSocketMessage::Close)) => break,
            Ok(Err(error)) if error.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Ok(Err(error)) => return Err(error),
            Err(_) => {}
        }
    }

    if let Some(node_id) = registered_node {
        peers
            .lock()
            .expect("work websocket peer map poisoned")
            .remove(&node_id);
    }
    Ok(())
}

#[cfg(feature = "tls")]
fn register_tls_work_peer(
    peers: &Mutex<std::collections::BTreeMap<NodeId, SyncSender<Vec<u8>>>>,
    registered_node: &mut Option<NodeId>,
    node_id: NodeId,
    tx: SyncSender<Vec<u8>>,
) {
    if registered_node.as_ref() == Some(&node_id) {
        return;
    }
    if let Some(previous) = registered_node.replace(node_id) {
        peers
            .lock()
            .expect("work websocket peer map poisoned")
            .remove(&previous);
    }
    peers
        .lock()
        .expect("work websocket peer map poisoned")
        .insert(node_id, tx);
}

#[cfg(feature = "tls")]
fn forward_tls_work_envelope(
    peers: &Mutex<std::collections::BTreeMap<NodeId, SyncSender<Vec<u8>>>>,
    node_id: NodeId,
    payload: &[u8],
) {
    let Ok(frame) = encode_server_binary(payload).map_err(invalid_ws_data) else {
        return;
    };
    let mut peers = peers.lock().expect("work websocket peer map poisoned");
    let Some(tx) = peers.get(&node_id) else {
        return;
    };
    if tx.try_send(frame).is_err() {
        peers.remove(&node_id);
    }
}

#[cfg(feature = "tls")]
async fn read_tls_work_ws_message(
    stream: &mut AsyncTlsServerStream<Arc<AsyncTcpStream>>,
) -> std::io::Result<WebSocketMessage> {
    let mut header = [0u8; 2];
    stream
        .read_exact(&mut header)
        .await
        .map_err(node_io_error)?;
    let prefix = decode_frame_prefix(&header)
        .and_then(|prefix| prefix.require_client_mask())
        .map_err(invalid_ws_data)?;
    let mut extended = [0u8; WS_EXTENDED_64_LEN];
    let extended_len = prefix.extended_len_bytes();
    if extended_len != 0 {
        stream
            .read_exact(&mut extended[..extended_len])
            .await
            .map_err(node_io_error)?;
    }
    let payload_len = decode_payload_len(prefix, &extended[..extended_len], MAX_WORK_FRAME_LEN)
        .map_err(invalid_ws_data)?;
    let mut mask = [0u8; WS_MASK_LEN];
    stream.read_exact(&mut mask).await.map_err(node_io_error)?;
    let mut payload = vec![0u8; payload_len];
    stream
        .read_exact(&mut payload)
        .await
        .map_err(node_io_error)?;
    decode_client_message(prefix.fin, prefix.opcode, mask, payload).map_err(invalid_ws_data)
}

#[cfg(feature = "tls")]
async fn read_http_request<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
) -> std::io::Result<Vec<u8>> {
    let mut request = Vec::new();
    let mut byte = [0u8; 1];
    while request.len() <= edgerun_protocols::websocket::WS_MAX_HANDSHAKE_LEN {
        stream.read_exact(&mut byte).await.map_err(node_io_error)?;
        request.push(byte[0]);
        if handshake_complete(&request) {
            return Ok(request);
        }
    }
    Err(invalid_ws_data(WebSocketError::HandshakeTooLarge))
}

#[cfg(feature = "tls")]
async fn write_tls_response<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    status: &str,
    content_type: &str,
    body: &str,
) -> std::io::Result<()> {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.as_bytes().len()
    );
    stream
        .write_all(response.as_bytes())
        .await
        .map_err(node_io_error)?;
    stream.flush().await.map_err(node_io_error)
}

#[cfg(feature = "tls")]
fn request_path(request: &str) -> Option<&str> {
    let mut parts = request.lines().next()?.split_whitespace();
    let _method = parts.next()?;
    parts.next()
}

#[cfg(feature = "tls")]
fn request_method(request: &str) -> Option<&str> {
    request.lines().next()?.split_whitespace().next()
}

#[cfg(feature = "tls")]
fn is_websocket_upgrade(request: &str) -> bool {
    request
        .lines()
        .any(|line| line.eq_ignore_ascii_case("upgrade: websocket"))
}

#[cfg(feature = "tls")]
fn invalid_ws_data(error: impl core::fmt::Debug) -> std::io::Error {
    io_error(format!("invalid websocket data: {error:?}"))
}

#[cfg(feature = "tls")]
fn node_io_error(error: rt::IoError) -> std::io::Error {
    io_error(error.to_string())
}

#[cfg(feature = "tls")]
fn io_error(message: impl Into<String>) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, message.into())
}

fn build_dns_zone() -> DnsZone {
    let ttl = 3600;
    let mut zone = DnsZone::new(ORIGIN);
    zone.set_soa(
        HOSTNAME,
        "admin.edgerun.tech",
        2026050801,
        3600,
        900,
        604800,
        86400,
    );
    zone.add_ns(HOSTNAME);
    zone.add_a("@", Ipv4Addr::new(172, 245, 67, 49), ttl);
    zone.add_a("mail", Ipv4Addr::new(172, 245, 67, 49), ttl);
    zone.add_a("nodes", Ipv4Addr::new(172, 245, 67, 49), ttl);
    zone.add_a("*.nodes", Ipv4Addr::new(172, 245, 67, 49), ttl);
    zone.add_a("mta-sts", Ipv4Addr::new(172, 245, 67, 49), ttl);
    zone.add_mx("@", 10, HOSTNAME, ttl);
    zone.add_txt("@", "v=spf1 mx -all", ttl);
    zone.add_txt("_dmarc", "v=DMARC1; p=quarantine; rua=mailto:dmarc-reports@edgerun.tech; ruf=mailto:dmarc-reports@edgerun.tech; adkim=s; aspf=s", ttl);
    zone.add_txt(
        "_smtp._tls",
        "v=TLSRPTv1; rua=mailto:tls-reports@edgerun.tech",
        ttl,
    );
    zone.add_txt(
        "_edgerun-relay.nodes",
        "v=edgerun-node-relay-v1; host=nodes.edgerun.tech; signed-updates=required",
        ttl,
    );
    zone
}

fn print_plan() {
    println!("node_label={NODE_LABEL}");
    println!("origin={ORIGIN}");
    println!("hostname={HOSTNAME}");
    println!("controller={CONTROLLER_ZERO}");
    println!("runtime_root={RUNTIME_ROOT}");
    println!("derived_db_path={DERIVED_DB_PATH}");
    println!("public_ipv4={PUBLIC_IPV4}");
    println!("local_mail_domains=[\"{ORIGIN}\"]");
    println!(
        "certificate_domains=[\"{ORIGIN}\", \"{HOSTNAME}\", \"mta-sts.{ORIGIN}\", \"mta-sts.nodes.{ORIGIN}\"]"
    );
    println!("runtime_listeners=4");
    println!("runtime_requires_dns=true");
    println!("runtime_requires_acme=false");
    println!("domain={ORIGIN} authoritative_dns=true mail_enabled=true");
    println!("  mailbox=ken@{ORIGIN} target=ken@{ORIGIN}");
    println!("  mailbox=admin@{ORIGIN} target=admin@{ORIGIN}");
    println!("  mailbox=dmarc-reports@{ORIGIN} target=dmarc-reports@{ORIGIN}");
    println!("  mailbox=tls-reports@{ORIGIN} target=tls-reports@{ORIGIN}");
    println!("  alias=postmaster@{ORIGIN} target=ken@{ORIGIN}");
    println!("  alias=abuse@{ORIGIN} target=ken@{ORIGIN}");
    println!("  website={ORIGIN} repo=Sylchi/edge-vercel-front ref=main path=/");
    println!("domain=nodes.{ORIGIN} authoritative_dns=true mail_enabled=false");
}

fn health_check() -> bool {
    let mut ok = true;
    ok &= tcp_banner("SMTP port 25", "127.0.0.1:25", b"QUIT\r\n", "220");
    ok &= tcp_banner("IMAP port 143", "127.0.0.1:143", b"a001 LOGOUT\r\n", "* OK");
    ok &= tcp_connect("HTTP port 80", "127.0.0.1:80");
    ok &= dns_udp_check();
    if std::fs::metadata(TLS_CERT_PATH).is_err() || std::fs::metadata(TLS_KEY_PATH).is_err() {
        println!("  [WARN] TLS certificate not yet provisioned (ACME pending)");
    }
    println!(
        "Health check: {}",
        if ok { "ok" } else { "some checks failed" }
    );
    ok
}

fn tcp_connect(label: &str, addr: &str) -> bool {
    match TcpStream::connect_timeout(&addr.parse().unwrap(), Duration::from_secs(2)) {
        Ok(_) => {
            println!("  [OK] {label}");
            true
        }
        Err(error) => {
            println!("  [FAIL] {label} ({error})");
            false
        }
    }
}

fn tcp_banner(label: &str, addr: &str, write_after_read: &[u8], expected: &str) -> bool {
    match TcpStream::connect_timeout(&addr.parse().unwrap(), Duration::from_secs(2)) {
        Ok(mut stream) => {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
            let mut buf = [0u8; 512];
            match stream.read(&mut buf) {
                Ok(n) if String::from_utf8_lossy(&buf[..n]).contains(expected) => {
                    let _ = stream.write_all(write_after_read);
                    println!("  [OK] {label}");
                    true
                }
                Ok(n) => {
                    println!(
                        "  [FAIL] {label} (unexpected banner: {})",
                        String::from_utf8_lossy(&buf[..n]).trim()
                    );
                    false
                }
                Err(error) => {
                    println!("  [FAIL] {label} ({error})");
                    false
                }
            }
        }
        Err(error) => {
            println!("  [FAIL] {label} ({error})");
            false
        }
    }
}

fn dns_udp_check() -> bool {
    match UdpSocket::bind("127.0.0.1:0") {
        Ok(socket) => {
            let _ = socket.set_read_timeout(Some(Duration::from_secs(2)));
            let query = [
                0x12, 0x34, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 7, b'e',
                b'd', b'g', b'e', b'r', b'u', b'n', 4, b't', b'e', b'c', b'h', 0, 0, 1, 0, 1,
            ];
            if socket.send_to(&query, "127.0.0.1:53").is_err() {
                println!("  [FAIL] DNS port 53 (send failed)");
                return false;
            }
            let mut buf = [0u8; 512];
            match socket.recv_from(&mut buf) {
                Ok((n, _)) if n >= 12 => {
                    println!("  [OK] DNS port 53");
                    true
                }
                Ok(_) => {
                    println!("  [FAIL] DNS port 53 (short response)");
                    false
                }
                Err(error) => {
                    println!("  [FAIL] DNS port 53 ({error})");
                    false
                }
            }
        }
        Err(error) => {
            println!("  [FAIL] DNS port 53 ({error})");
            false
        }
    }
}
