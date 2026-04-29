//! Host mail server binary.
//!
//! Loads Kubernetes-style Edgerun YAML via `edgerun-config`, serves
//! authoritative DNS zones with `edgerun-dns`, accepts SMTP, relays outbound
//! mail through the built-in queue, and exposes IMAP over the same Maildir.

use std::collections::BTreeMap;
use std::future::Future;
use std::io;
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process;
use std::sync::Arc;
use std::time::Duration;

use edgerun_acme::{AccountKey, AcmeClient, AcmeConfig};
use edgerun_acme::{ChallengeStatus, ChallengeType, DirectoryUrl, OrderStatus};
use edgerun_config::edgerun_json::JsonValue;
use edgerun_config::{
    ConfigResource, DnsServerSpec, DnsZoneSpec, ImapServerSpec, MailUserSpec, SmtpServerSpec,
    ZoneRecord,
};
use edgerun_dns::{DnsRecord, DnsServer, DnsServerConfig, DnsZone};
use edgerun_email::imap::{ImapServer, ImapServerConfig, MaildirImapStore};
use edgerun_email::smtp::server::{MaildirStore, SmtpServer, SmtpServerConfig};
use edgerun_email::smtp::ServerLimits;
use edgerun_encoding::base64::standard_decode;
use edgerun_http::{Handler, HttpServer, Request, Response, StatusCode};
use edgerun_rt::CancellationToken;
use edgerun_tls::CertificateAndKey;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage(
            args.first()
                .map(String::as_str)
                .unwrap_or("edgerun-mail-server"),
        );
        return;
    }
    if args.iter().any(|arg| arg == "--init-material") {
        if let Err(error) = init_material(&args) {
            eprintln!("{error}");
            process::exit(1);
        }
        return;
    }
    if args.iter().any(|arg| arg == "--check-config") {
        match load_resources_from_args(&args) {
            Ok(resources) => {
                let (dns, zones, smtp, imap) = count_resources(&resources);
                println!(
                    "dns_servers={dns} dns_zones={zones} smtp_servers={smtp} imap_servers={imap}"
                );
                for resource in &resources {
                    if let ConfigResource::DnsZone(zone) = resource {
                        println!("dns_zone={} records={}", zone.origin, zone.records.len());
                    } else if let ConfigResource::SmtpServer(smtp) = resource {
                        println!(
                            "smtp_server={} local_domains={} users={}",
                            smtp.hostname,
                            smtp.local_domains.join(","),
                            smtp.users.as_ref().map(|users| users.len()).unwrap_or(0)
                        );
                    }
                }
            }
            Err(error) => {
                eprintln!("{error}");
                process::exit(1);
            }
        }
        return;
    }
    let config_path = match parse_config_path(&args) {
        Ok(path) => path,
        Err(message) => {
            eprintln!("{message}");
            process::exit(2);
        }
    };

    let resources = match load_resources(&config_path) {
        Ok(resources) => resources,
        Err(error) => {
            eprintln!("{error}");
            process::exit(1);
        }
    };

    let rt = edgerun_rt::Runtime::new_multi_thread()
        .enable_all()
        .build()
        .unwrap_or_else(|error| {
            eprintln!("failed to build runtime: {error}");
            process::exit(1);
        });

    rt.block_on(async move {
        if let Err(error) = run(resources).await {
            eprintln!("edgerun-mail-server: {error}");
            process::exit(1);
        }
    });
}

fn print_usage(program: &str) {
    println!(
        "usage: {program} --config /etc/edgerun/mail.yaml\n\
         usage: {program} --init-material --domain edgerun.tech --selector mail --out-dir /etc/edgerun/mail"
    );
}

fn load_resources_from_args(args: &[String]) -> Result<Vec<ConfigResource>, String> {
    let config_path = parse_config_path(args)?;
    load_resources(&config_path)
}

fn load_resources(config_path: &Path) -> Result<Vec<ConfigResource>, String> {
    let yaml = std::fs::read_to_string(config_path)
        .map_err(|error| format!("failed to read {}: {error}", config_path.display()))?;
    edgerun_config::parse_config_file(&yaml)
        .map_err(|error| format!("failed to parse {}: {error}", config_path.display()))
}

fn count_resources(resources: &[ConfigResource]) -> (usize, usize, usize, usize) {
    let mut dns = 0;
    let mut zones = 0;
    let mut smtp = 0;
    let mut imap = 0;
    for resource in resources {
        match resource {
            ConfigResource::DnsServer(_) => dns += 1,
            ConfigResource::DnsZone(_) => zones += 1,
            ConfigResource::SmtpServer(_) => smtp += 1,
            ConfigResource::ImapServer(_) => imap += 1,
            _ => {}
        }
    }
    (dns, zones, smtp, imap)
}

fn parse_config_path(args: &[String]) -> Result<PathBuf, String> {
    let mut config = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--check-config" => {}
            "--config" | "-c" if i + 1 < args.len() => {
                config = Some(PathBuf::from(&args[i + 1]));
                i += 1;
            }
            other => return Err(format!("unknown argument: {other}")),
        }
        i += 1;
    }
    config.ok_or_else(|| {
        "missing --config /path/to/mail.yaml\nusage: edgerun-mail-server --config /etc/edgerun/mail.yaml"
            .to_string()
    })
}

async fn run(resources: Vec<ConfigResource>) -> io::Result<()> {
    let mut dns_servers = Vec::new();
    let mut zones = Vec::new();
    let mut smtp_specs = Vec::new();
    let mut imap_specs = Vec::new();

    for resource in resources {
        match resource {
            ConfigResource::DnsServer(spec) => dns_servers.push(spec),
            ConfigResource::DnsZone(spec) => zones.push(spec),
            ConfigResource::SmtpServer(spec) => smtp_specs.push(spec),
            ConfigResource::ImapServer(spec) => imap_specs.push(spec),
            _ => {}
        }
    }
    eprintln!(
        "edgerun-mail-server: config dns_servers={} dns_zones={} smtp_servers={} imap_servers={}",
        dns_servers.len(),
        zones.len(),
        smtp_specs.len(),
        imap_specs.len()
    );

    let shutdown = CancellationToken::new();
    let mut tasks = Vec::new();
    let mut dns_server = None;

    if !zones.is_empty() {
        let dns = Arc::new(build_dns_server(dns_servers.first(), &zones).await?);
        let dns_run = Arc::clone(&dns);
        tasks.push(edgerun_rt::spawn(async move {
            dns_run.run().await.map_err(to_io_error)
        }));
        let dns_shutdown = Arc::clone(&dns);
        let token = shutdown.clone();
        tasks.push(edgerun_rt::spawn(async move {
            token.cancelled().await;
            dns_shutdown.shutdown().await;
            Ok(())
        }));
        dns_server = Some(dns);
    }

    for spec in &smtp_specs {
        if spec.acme_enabled {
            let Some(dns) = dns_server.as_ref() else {
                return Err(invalid_config("ACME DNS-01 requires at least one DnsZone"));
            };
            ensure_acme_certificate(spec, dns, &zones).await?;
        }
    }

    if let Some(webmail) = build_webmail_config(&smtp_specs, &imap_specs)? {
        let http = HttpServer::new(WebmailHandler::new(webmail.clone()))
            .bind("0.0.0.0:80")
            .await
            .map_err(to_io_error)?;
        let token = shutdown.clone();
        tasks.push(edgerun_rt::spawn(async move {
            http.serve_with_shutdown(token).await.map_err(to_io_error)
        }));

        if let Some(tls) =
            load_tls_from_spec(webmail.tls_cert.as_deref(), webmail.tls_key.as_deref())?
        {
            let https = HttpServer::new(WebmailHandler::new(webmail))
                .with_tls(tls)
                .bind("0.0.0.0:443")
                .await
                .map_err(to_io_error)?;
            let token = shutdown.clone();
            tasks.push(edgerun_rt::spawn(async move {
                https.serve_with_shutdown(token).await.map_err(to_io_error)
            }));
        }
    }

    for spec in smtp_specs {
        for server in build_smtp_servers(&spec)? {
            let token = shutdown.clone();
            tasks.push(edgerun_rt::spawn(async move { server.run(token).await }));
        }
    }

    for spec in imap_specs {
        for server in build_imap_servers(&spec)? {
            let token = shutdown.clone();
            tasks.push(edgerun_rt::spawn(async move { server.run(token).await }));
        }
    }

    if tasks.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "config did not define any DnsZone, SmtpServer, or ImapServer resources",
        ));
    }

    eprintln!(
        "edgerun-mail-server: running {} service task(s)",
        tasks.len()
    );
    for task in tasks {
        match task.await {
            Ok(result) => result?,
            Err(error) => return Err(io::Error::other(error)),
        }
    }
    Ok(())
}

#[derive(Clone)]
struct WebmailConfig {
    hostname: String,
    username: String,
    password: String,
    address: String,
    maildir_root: PathBuf,
    smtp_addr: String,
    tls_cert: Option<String>,
    tls_key: Option<String>,
}

fn build_webmail_config(
    smtp_specs: &[SmtpServerSpec],
    imap_specs: &[ImapServerSpec],
) -> io::Result<Option<WebmailConfig>> {
    let Some(imap) = imap_specs.first() else {
        return Ok(None);
    };
    let Some(user) = imap.users.as_deref().and_then(|users| users.first()) else {
        return Ok(None);
    };
    let Some(password) = user.password.clone() else {
        return Ok(None);
    };
    let smtp = smtp_specs.first();
    let hostname = smtp
        .map(|spec| spec.hostname.clone())
        .unwrap_or_else(|| imap.hostname.clone());
    let domain = smtp
        .and_then(|spec| spec.local_domains.first().cloned())
        .unwrap_or_else(|| hostname.trim_start_matches("mail.").to_string());
    let smtp_addr = smtp
        .and_then(|spec| spec.bind_address.clone())
        .unwrap_or_else(|| "127.0.0.1:25".to_string())
        .replace("0.0.0.0:", "127.0.0.1:");
    let maildir_root = imap
        .maildir_root
        .clone()
        .or_else(|| smtp.and_then(|spec| spec.maildir_root.clone()))
        .unwrap_or_else(|| "/var/lib/edgerun/mail/maildirs".to_string());
    Ok(Some(WebmailConfig {
        hostname,
        username: user.username.clone(),
        password,
        address: format!("{}@{}", user.username, domain),
        maildir_root: PathBuf::from(maildir_root),
        smtp_addr,
        tls_cert: imap.tls_cert.clone(),
        tls_key: imap.tls_key.clone(),
    }))
}

struct WebmailHandler {
    config: WebmailConfig,
}

impl WebmailHandler {
    fn new(config: WebmailConfig) -> Self {
        Self { config }
    }

    fn handle_sync(&self, request: Request) -> Response {
        let path = request.uri().request_target();
        if path == "/" || path == "/index.html" {
            return html_response(WEBMAIL_HTML);
        }
        if path.starts_with("/api/") && !authorized(&request, &self.config) {
            return unauthorized();
        }
        match (request.method().as_str(), path.as_str()) {
            ("GET", "/api/messages") => match list_webmail_messages(&self.config) {
                Ok(body) => json_response(&body),
                Err(error) => server_error(&error.to_string()),
            },
            ("GET", path) if path.starts_with("/api/message/") => {
                let id = percent_decode(&path["/api/message/".len()..]);
                match read_webmail_message(&self.config, &id) {
                    Ok(body) => json_response(&body),
                    Err(error) if error.kind() == io::ErrorKind::NotFound => Response::not_found(),
                    Err(error) => server_error(&error.to_string()),
                }
            }
            ("POST", "/api/send") => match send_webmail_message(&self.config, &request) {
                Ok(()) => json_response(r#"{"ok":true}"#),
                Err(error) => server_error(&error.to_string()),
            },
            _ => Response::not_found(),
        }
    }
}

impl Handler for WebmailHandler {
    fn handle(&self, request: Request) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
        Box::pin(async move { self.handle_sync(request) })
    }
}

fn authorized(request: &Request, config: &WebmailConfig) -> bool {
    let Some(header) = request.headers().get("authorization") else {
        return false;
    };
    let value = header.as_str();
    let Some(encoded) = value.strip_prefix("Basic ") else {
        return false;
    };
    let Ok(decoded) = standard_decode(encoded) else {
        return false;
    };
    let Ok(credentials) = String::from_utf8(decoded) else {
        return false;
    };
    credentials == format!("{}:{}", config.username, config.password)
        || credentials == format!("{}:{}", config.address, config.password)
}

fn unauthorized() -> Response {
    Response::text(StatusCode::new(401).unwrap(), "authentication required")
        .with_header("WWW-Authenticate", r#"Basic realm="Edgerun Mail""#)
        .with_header("Cache-Control", "no-store")
}

fn html_response(body: &str) -> Response {
    Response::html(StatusCode::OK, body)
        .with_header("Cache-Control", "no-store")
        .with_header("X-Content-Type-Options", "nosniff")
}

fn json_response(body: &str) -> Response {
    Response::json(StatusCode::OK, body).with_header("Cache-Control", "no-store")
}

fn server_error(message: &str) -> Response {
    let body = format!(r#"{{"ok":false,"error":"{}"}}"#, json_escape(message));
    Response::json(StatusCode::INTERNAL_SERVER_ERROR, &body)
}

fn list_webmail_messages(config: &WebmailConfig) -> io::Result<String> {
    let mut messages = Vec::new();
    collect_maildir_entries(config, "new", &mut messages)?;
    collect_maildir_entries(config, "cur", &mut messages)?;
    messages.sort_by(|a, b| b.modified.cmp(&a.modified));
    messages.truncate(100);

    let mut out = String::from(r#"{"messages":["#);
    for (idx, message) in messages.iter().enumerate() {
        let raw = std::fs::read_to_string(&message.path).unwrap_or_default();
        if idx > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            r#"{{"id":"{}","from":"{}","to":"{}","subject":"{}","date":"{}","state":"{}","preview":"{}"}}"#,
            json_escape(&message.id),
            json_escape(header_value(&raw, "From").unwrap_or_default().as_str()),
            json_escape(header_value(&raw, "To").unwrap_or_default().as_str()),
            json_escape(header_value(&raw, "Subject").unwrap_or_else(|| "(no subject)".to_string()).as_str()),
            json_escape(header_value(&raw, "Date").unwrap_or_default().as_str()),
            message.state,
            json_escape(&message_preview(&raw)),
        ));
    }
    out.push_str("]}");
    Ok(out)
}

fn read_webmail_message(config: &WebmailConfig, id: &str) -> io::Result<String> {
    let path = find_maildir_message(config, id)?;
    let raw = std::fs::read_to_string(path)?;
    Ok(format!(
        r#"{{"id":"{}","from":"{}","to":"{}","subject":"{}","date":"{}","body":"{}","raw":"{}"}}"#,
        json_escape(id),
        json_escape(header_value(&raw, "From").unwrap_or_default().as_str()),
        json_escape(header_value(&raw, "To").unwrap_or_default().as_str()),
        json_escape(
            header_value(&raw, "Subject")
                .unwrap_or_else(|| "(no subject)".to_string())
                .as_str()
        ),
        json_escape(header_value(&raw, "Date").unwrap_or_default().as_str()),
        json_escape(&message_body(&raw)),
        json_escape(&raw),
    ))
}

fn send_webmail_message(config: &WebmailConfig, request: &Request) -> io::Result<()> {
    let body = request.body().unwrap_or_default();
    let value: JsonValue = edgerun_config::edgerun_json::from_json_slice(body)
        .map_err(|error| invalid_config(format!("invalid JSON: {error}")))?;
    let to = json_field(&value, "to")?;
    let subject = json_field(&value, "subject")?;
    let text = json_field(&value, "body")?;
    if !to.contains('@') || to.contains('\r') || to.contains('\n') {
        return Err(invalid_config("invalid recipient"));
    }
    let message = format!(
        "From: {}\r\nTo: {}\r\nSubject: {}\r\nDate: {}\r\nMessage-ID: <{}@{}>\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{}\r\n",
        config.address,
        sanitize_header(&to),
        sanitize_header(&subject),
        http_date_now(),
        unique_webmail_id(),
        config.hostname,
        normalize_crlf(&text),
    );
    let smtp_addr = config.smtp_addr.clone();
    let hostname = config.hostname.clone();
    let from = config.address.clone();
    std::thread::spawn(move || {
        if let Err(error) = submit_smtp(&smtp_addr, &hostname, &from, &to, &message) {
            eprintln!("edgerun-webmail: SMTP submit failed: {error}");
        }
    });
    Ok(())
}

fn json_field(value: &JsonValue, key: &str) -> io::Result<String> {
    value
        .get(key)
        .and_then(JsonValue::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| invalid_config(format!("missing JSON field: {key}")))
}

#[derive(Clone)]
struct WebmailMessage {
    id: String,
    path: PathBuf,
    state: &'static str,
    modified: std::time::SystemTime,
}

fn collect_maildir_entries(
    config: &WebmailConfig,
    state: &'static str,
    messages: &mut Vec<WebmailMessage>,
) -> io::Result<()> {
    let dir = config.maildir_root.join(&config.username).join(state);
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(std::time::UNIX_EPOCH);
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        messages.push(WebmailMessage {
            id: name.to_string(),
            path,
            state,
            modified,
        });
    }
    Ok(())
}

fn find_maildir_message(config: &WebmailConfig, id: &str) -> io::Result<PathBuf> {
    if id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err(io::Error::new(io::ErrorKind::NotFound, "message not found"));
    }
    for state in ["new", "cur"] {
        let path = config
            .maildir_root
            .join(&config.username)
            .join(state)
            .join(id);
        if path.is_file() {
            return Ok(path);
        }
    }
    Err(io::Error::new(io::ErrorKind::NotFound, "message not found"))
}

fn submit_smtp(addr: &str, hostname: &str, from: &str, to: &str, message: &str) -> io::Result<()> {
    let mut stream = std::net::TcpStream::connect(addr)?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    expect_smtp(&mut reader, 220)?;
    smtp_command(
        &mut stream,
        &mut reader,
        &format!("EHLO {hostname}\r\n"),
        250,
    )?;
    smtp_command(
        &mut stream,
        &mut reader,
        &format!("MAIL FROM:<{from}>\r\n"),
        250,
    )?;
    smtp_command(
        &mut stream,
        &mut reader,
        &format!("RCPT TO:<{to}>\r\n"),
        250,
    )?;
    smtp_command(&mut stream, &mut reader, "DATA\r\n", 354)?;
    stream.write_all(dot_stuffed(message).as_bytes())?;
    stream.write_all(b"\r\n.\r\n")?;
    stream.flush()?;
    expect_smtp(&mut reader, 250)?;
    let _ = smtp_command(&mut stream, &mut reader, "QUIT\r\n", 221);
    Ok(())
}

fn smtp_command(
    stream: &mut std::net::TcpStream,
    reader: &mut BufReader<std::net::TcpStream>,
    command: &str,
    expected: u16,
) -> io::Result<()> {
    stream.write_all(command.as_bytes())?;
    stream.flush()?;
    expect_smtp(reader, expected)
}

fn expect_smtp(reader: &mut BufReader<std::net::TcpStream>, expected: u16) -> io::Result<()> {
    let mut line = String::new();
    loop {
        line.clear();
        let n = match reader.read_line(&mut line) {
            Ok(n) => n,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(25));
                continue;
            }
            Err(error) => return Err(error),
        };
        if n == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "SMTP closed"));
        }
        if line.len() < 4 {
            continue;
        }
        let code = line[..3].parse::<u16>().unwrap_or(0);
        let more = line.as_bytes().get(3) == Some(&b'-');
        if !more {
            if code == expected {
                return Ok(());
            }
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("SMTP expected {expected}, got {}", line.trim_end()),
            ));
        }
    }
}

fn header_value(raw: &str, name: &str) -> Option<String> {
    let mut current_name = String::new();
    let mut current_value = String::new();
    for line in raw.lines() {
        if line.is_empty() || line == "\r" {
            break;
        }
        if line.starts_with(' ') || line.starts_with('\t') {
            if current_name.eq_ignore_ascii_case(name) {
                current_value.push(' ');
                current_value.push_str(line.trim());
            }
            continue;
        }
        if current_name.eq_ignore_ascii_case(name) {
            return Some(current_value.trim().to_string());
        }
        if let Some((left, right)) = line.split_once(':') {
            current_name.clear();
            current_name.push_str(left.trim());
            current_value.clear();
            current_value.push_str(right.trim());
        }
    }
    if current_name.eq_ignore_ascii_case(name) {
        Some(current_value.trim().to_string())
    } else {
        None
    }
}

fn message_body(raw: &str) -> String {
    raw.split_once("\r\n\r\n")
        .or_else(|| raw.split_once("\n\n"))
        .map(|(_, body)| body.trim().to_string())
        .unwrap_or_default()
}

fn message_preview(raw: &str) -> String {
    let body = message_body(raw).replace('\r', " ").replace('\n', " ");
    let mut preview = String::new();
    for ch in body.chars().take(180) {
        preview.push(ch);
    }
    preview
}

fn json_escape(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => {}
            ch => out.push(ch),
        }
    }
    out
}

fn percent_decode(value: &str) -> String {
    let mut out = Vec::new();
    let bytes = value.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(a), Some(b)) = (hex_value(bytes[i + 1]), hex_value(bytes[i + 2])) {
                out.push((a << 4) | b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn sanitize_header(value: &str) -> String {
    value.replace(['\r', '\n'], " ").trim().to_string()
}

fn normalize_crlf(value: &str) -> String {
    value
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\n', "\r\n")
}

fn dot_stuffed(value: &str) -> String {
    let mut out = String::new();
    for line in value.replace("\r\n", "\n").split('\n') {
        if line.starts_with('.') {
            out.push('.');
        }
        out.push_str(line);
        out.push_str("\r\n");
    }
    out.trim_end_matches("\r\n").to_string()
}

fn unique_webmail_id() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_micros())
        .unwrap_or(0);
    format!("webmail-{now}")
}

fn http_date_now() -> String {
    // A stable RFC 5322-ish date is enough for the simple composer; MTAs add Received headers.
    format!("{:?}", std::time::SystemTime::now())
}

const WEBMAIL_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Edgerun Mail</title>
<style>
:root{color-scheme:light;--bg:#f7f7f4;--panel:#fff;--ink:#1c1d1f;--muted:#656b73;--line:#d8d9d4;--accent:#0f766e;--accent2:#7c2d12}
*{box-sizing:border-box}body{margin:0;font:14px/1.45 system-ui,-apple-system,Segoe UI,sans-serif;background:var(--bg);color:var(--ink)}
.app{height:100vh;display:grid;grid-template-columns:minmax(280px,380px) 1fr}
.list{border-right:1px solid var(--line);background:#fbfbf8;display:flex;flex-direction:column;min-width:0}
.top{height:56px;display:flex;align-items:center;gap:10px;padding:0 14px;border-bottom:1px solid var(--line)}
.brand{font-weight:700;font-size:16px}.who{color:var(--muted);font-size:12px;margin-left:auto}
button{border:1px solid var(--line);background:#fff;border-radius:6px;padding:8px 10px;cursor:pointer;color:var(--ink)}
button.primary{background:var(--accent);border-color:var(--accent);color:#fff}button:disabled{opacity:.55;cursor:not-allowed}
.messages{overflow:auto;min-height:0}.item{padding:12px 14px;border-bottom:1px solid var(--line);cursor:pointer}
.item:hover,.item.active{background:#eef6f4}.from{font-weight:650;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.subject{margin-top:2px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}.preview{margin-top:4px;color:var(--muted);font-size:12px;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden}
.pane{min-width:0;display:grid;grid-template-rows:auto 1fr;background:var(--panel)}
.actions{height:56px;display:flex;align-items:center;gap:8px;padding:0 16px;border-bottom:1px solid var(--line)}
.content{overflow:auto;padding:22px;max-width:980px;width:100%}.empty{color:var(--muted);margin-top:20vh;text-align:center}
h1{font-size:22px;margin:0 0 8px}.meta{color:var(--muted);margin-bottom:18px;display:grid;gap:3px}
pre{white-space:pre-wrap;word-break:break-word;font:14px/1.5 ui-monospace,SFMono-Regular,Menlo,monospace;background:#fafafa;border:1px solid var(--line);border-radius:6px;padding:14px}
.compose{display:none;padding:16px;border-bottom:1px solid var(--line);background:#fff}.compose.open{display:grid;gap:10px}
input,textarea{width:100%;border:1px solid var(--line);border-radius:6px;padding:10px;font:inherit;background:#fff;color:var(--ink)}
textarea{min-height:150px;resize:vertical}.row{display:flex;gap:8px}.status{color:var(--accent2);font-size:13px}
@media(max-width:760px){.app{grid-template-columns:1fr;grid-template-rows:45vh 55vh}.list{border-right:0;border-bottom:1px solid var(--line)}.content{padding:16px}}
</style>
</head>
<body>
<main class="app">
  <section class="list">
    <div class="top"><div class="brand">Edgerun Mail</div><button id="refresh">Refresh</button><div class="who">ken@edgerun.tech</div></div>
    <div id="messages" class="messages"></div>
  </section>
  <section class="pane">
    <div class="actions"><button id="composeBtn" class="primary">Compose</button><div id="status" class="status"></div></div>
    <form id="compose" class="compose">
      <input id="to" placeholder="To" autocomplete="off">
      <input id="subject" placeholder="Subject" autocomplete="off">
      <textarea id="body" placeholder="Message"></textarea>
      <div class="row"><button class="primary" type="submit">Send</button><button id="cancel" type="button">Cancel</button></div>
    </form>
    <article id="content" class="content"><div class="empty">Select a message</div></article>
  </section>
</main>
<script>
const messagesEl=document.getElementById('messages'),content=document.getElementById('content'),statusEl=document.getElementById('status');
let selected='';
function esc(s){return (s||'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));}
async function api(path,opts){const r=await fetch(path,opts);if(r.status===401){statusEl.textContent='Login required';throw new Error('auth');}if(!r.ok)throw new Error(await r.text());return r.json();}
async function load(){statusEl.textContent='';const data=await api('/api/messages');messagesEl.innerHTML=data.messages.map(m=>`<div class="item ${m.id===selected?'active':''}" data-id="${encodeURIComponent(m.id)}"><div class="from">${esc(m.from||'(unknown)')}</div><div class="subject">${esc(m.subject)}</div><div class="preview">${esc(m.preview)}</div></div>`).join('')||'<div class="empty">No mail</div>';}
messagesEl.onclick=e=>{const item=e.target.closest('.item');if(item)openMsg(decodeURIComponent(item.dataset.id));};
async function openMsg(id){selected=id;load();const m=await api('/api/message/'+encodeURIComponent(id));content.innerHTML=`<h1>${esc(m.subject)}</h1><div class="meta"><div>From: ${esc(m.from)}</div><div>To: ${esc(m.to)}</div><div>${esc(m.date)}</div></div><pre>${esc(m.body||m.raw)}</pre>`;}
document.getElementById('refresh').onclick=load;
document.getElementById('composeBtn').onclick=()=>document.getElementById('compose').classList.add('open');
document.getElementById('cancel').onclick=()=>document.getElementById('compose').classList.remove('open');
document.getElementById('compose').onsubmit=async e=>{e.preventDefault();statusEl.textContent='Sending...';await api('/api/send',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({to:to.value,subject:subject.value,body:body.value})});to.value=subject.value=body.value='';document.getElementById('compose').classList.remove('open');statusEl.textContent='Sent';setTimeout(()=>statusEl.textContent='',2500);load();};
load().catch(err=>statusEl.textContent=err.message);
</script>
</body>
</html>"#;

async fn build_dns_server(
    server_spec: Option<&DnsServerSpec>,
    zone_specs: &[DnsZoneSpec],
) -> io::Result<DnsServer> {
    let config = DnsServerConfig {
        bind_addr: server_spec
            .and_then(|spec| spec.bind_address.clone())
            .unwrap_or_else(|| "0.0.0.0:53".to_string()),
        bind_addr_ipv6: server_spec.and_then(|spec| spec.bind_address_ipv6.clone()),
        default_ttl: server_spec
            .and_then(|spec| spec.default_ttl)
            .unwrap_or(3600),
        rate_limit_qps: server_spec
            .and_then(|spec| spec.rate_limit_qps)
            .unwrap_or(0),
    };
    let server = DnsServer::new(config).map_err(to_io_error)?;
    if let Some(forward_to) = server_spec.and_then(|spec| spec.forward_to.clone()) {
        server.set_forward_to(Some(forward_to)).await;
    }
    let mut by_origin: BTreeMap<String, Vec<&DnsZoneSpec>> = BTreeMap::new();
    for zone_spec in zone_specs {
        by_origin
            .entry(zone_spec.origin.to_ascii_lowercase())
            .or_default()
            .push(zone_spec);
    }
    for specs in by_origin.values() {
        server.add_zone(zone_from_config(specs)?).await;
    }
    Ok(server)
}

fn zone_from_config(specs: &[&DnsZoneSpec]) -> io::Result<DnsZone> {
    let spec = specs
        .first()
        .ok_or_else(|| invalid_config("empty DNS zone group"))?;
    let mut zone = DnsZone::new(&spec.origin);
    zone.add_record(DnsRecord::soa(
        spec.origin.clone(),
        normalize_target(&spec.soa.mname, &spec.origin),
        spec.soa.rname.clone(),
        spec.soa.serial,
        spec.soa.refresh,
        spec.soa.retry,
        spec.soa.expire,
        spec.soa.minimum,
        spec.soa.minimum,
    ));
    for spec in specs {
        for record in &spec.records {
            add_zone_record(&mut zone, record, &spec.origin)?;
        }
        if let Some(wildcards) = &spec.wildcards {
            for record in wildcards {
                add_zone_record(&mut zone, record, &spec.origin)?;
            }
        }
    }
    Ok(zone)
}

fn add_zone_record(zone: &mut DnsZone, record: &ZoneRecord, origin: &str) -> io::Result<()> {
    let ttl = record.ttl.unwrap_or(3600);
    let rtype = record.record_type.to_ascii_uppercase();
    let value = record_value_string(record)?;

    match rtype.as_str() {
        "A" => zone.add_a(
            &record.name,
            value.parse::<Ipv4Addr>().map_err(invalid_config)?,
            ttl,
        ),
        "AAAA" => zone.add_aaaa(
            &record.name,
            value.parse::<Ipv6Addr>().map_err(invalid_config)?,
            ttl,
        ),
        "CNAME" => zone.add_cname(&record.name, &normalize_target(&value, origin), ttl),
        "NS" => zone.add_ns(&normalize_target(&value, origin)),
        "PTR" => zone.add_ptr(&record.name, &normalize_target(&value, origin), ttl),
        "MX" => {
            let (priority, exchange) = parse_mx(&value)?;
            zone.add_mx(
                &record.name,
                priority,
                &normalize_target(&exchange, origin),
                ttl,
            );
        }
        "TXT" | "SPF" => zone.add_txt(&record.name, &value, ttl),
        "CAA" => {
            let (critical, tag, caa_value) = parse_caa(&value)?;
            zone.add_caa(&record.name, critical, &tag, &caa_value, ttl);
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unsupported DNS record type in mail server config: {rtype}"),
            ));
        }
    }
    Ok(())
}

async fn ensure_acme_certificate(
    spec: &SmtpServerSpec,
    dns: &Arc<DnsServer>,
    zones: &[DnsZoneSpec],
) -> io::Result<()> {
    let cert_dir = PathBuf::from(
        spec.acme_cert_dir
            .as_deref()
            .unwrap_or("/etc/edgerun/mail/tls"),
    );
    let fullchain_path = cert_dir.join("fullchain.pem");
    let privkey_path = cert_dir.join("privkey.pem");
    if fullchain_path.exists() && privkey_path.exists() {
        return Ok(());
    }

    std::fs::create_dir_all(&cert_dir)?;
    let domains = spec
        .acme_domains
        .clone()
        .unwrap_or_else(|| vec![spec.hostname.clone()]);
    let domain_refs: Vec<&str> = domains.iter().map(String::as_str).collect();
    let account_key_path = PathBuf::from(
        spec.acme_account_key_path
            .as_deref()
            .unwrap_or("/etc/edgerun/mail/acme-account.pem"),
    );
    let account_key = if account_key_path.exists() {
        let pem = std::fs::read_to_string(&account_key_path)?;
        AccountKey::from_pem(&pem).map_err(acme_io_error)?
    } else {
        let key = AccountKey::generate();
        if let Some(parent) = account_key_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&account_key_path, key.pem())?;
        key
    };

    let client = AcmeClient::new(
        AcmeConfig {
            directory_url: parse_acme_directory(spec.acme_directory.as_deref())?,
            email: spec
                .acme_contact_email
                .as_deref()
                .map(|email| vec![format!("mailto:{email}")])
                .unwrap_or_default(),
            terms_of_service_agreed: true,
        },
        account_key,
    );
    client.init().await.map_err(acme_io_error)?;
    client.create_account().await.map_err(acme_io_error)?;
    let order = client.create_order(&domains).await.map_err(acme_io_error)?;

    let mut challenge_records = Vec::new();
    for auth_url in order.authorization_urls() {
        let authorization = client
            .get_authorization(auth_url)
            .await
            .map_err(acme_io_error)?;
        if authorization.status == edgerun_acme::types::AuthorizationStatus::Valid {
            continue;
        }
        let challenge = authorization
            .challenges
            .as_deref()
            .and_then(|challenges| {
                challenges
                    .iter()
                    .find(|challenge| challenge.challenge_type == ChallengeType::Dns01)
            })
            .ok_or_else(|| invalid_config("ACME authorization has no dns-01 challenge"))?;
        let token = challenge
            .token
            .as_deref()
            .ok_or_else(|| invalid_config("ACME dns-01 challenge missing token"))?;
        let domain = authorization.identifier.value.as_str();
        let dns_manager = client.dns_manager();
        let dns_challenge = dns_manager.create_challenge(domain, token);
        challenge_records.push(ZoneRecord {
            name: dns_challenge.record_name().to_string(),
            record_type: "TXT".to_string(),
            ttl: Some(60),
            value: JsonValue::String(dns_challenge.record_value().to_string()),
        });
        publish_acme_challenge_records(dns, zones, &challenge_records).await?;
        edgerun_rt::sleep(Duration::from_secs(20)).await;
        client
            .validate_challenge(&challenge.url)
            .await
            .map_err(acme_io_error)?;
        wait_for_challenge(&client, &challenge.url).await?;
    }

    let mut ready_order = client
        .get_order(&order.inner.id)
        .await
        .map_err(acme_io_error)?;
    for _ in 0..30 {
        match ready_order.status() {
            OrderStatus::Ready | OrderStatus::Valid => break,
            OrderStatus::Invalid => return Err(invalid_config("ACME order became invalid")),
            _ => {
                edgerun_rt::sleep(Duration::from_secs(2)).await;
                ready_order = client
                    .get_order(&order.inner.id)
                    .await
                    .map_err(acme_io_error)?;
            }
        }
    }
    let finalize_url = ready_order
        .finalize_url()
        .ok_or_else(|| invalid_config("ACME order missing finalize URL"))?;
    let (csr_der, signing_key) = edgerun_tls::generate_csr(&domain_refs)
        .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?;
    let mut finalized = client
        .finalize_order(finalize_url, &csr_der)
        .await
        .map_err(acme_io_error)?;
    for _ in 0..30 {
        match finalized.status() {
            OrderStatus::Valid => break,
            OrderStatus::Invalid => {
                return Err(invalid_config("ACME finalized order became invalid"))
            }
            _ => {
                edgerun_rt::sleep(Duration::from_secs(2)).await;
                finalized = client
                    .get_order(&finalized.inner.id)
                    .await
                    .map_err(acme_io_error)?;
            }
        }
    }
    let certificate_url = finalized
        .certificate_url()
        .ok_or_else(|| invalid_config("ACME order missing certificate URL"))?;
    let cert_pem = client
        .download_certificate(certificate_url)
        .await
        .map_err(acme_io_error)?;
    let key_pem = edgerun_tls::signing_key_to_pem(&signing_key)
        .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?;
    std::fs::write(&fullchain_path, cert_pem)?;
    std::fs::write(&privkey_path, key_pem)?;
    Ok(())
}

async fn wait_for_challenge(client: &AcmeClient, url: &edgerun_url::Url) -> io::Result<()> {
    for _ in 0..30 {
        let challenge = client.get_challenge(url).await.map_err(acme_io_error)?;
        match challenge.status() {
            ChallengeStatus::Valid => return Ok(()),
            ChallengeStatus::Invalid => {
                return Err(invalid_config("ACME challenge became invalid"))
            }
            _ => edgerun_rt::sleep(Duration::from_secs(2)).await,
        }
    }
    Err(invalid_config("ACME challenge did not become valid"))
}

async fn publish_acme_challenge_records(
    dns: &Arc<DnsServer>,
    zones: &[DnsZoneSpec],
    records: &[ZoneRecord],
) -> io::Result<()> {
    let mut by_origin: BTreeMap<String, Vec<&DnsZoneSpec>> = BTreeMap::new();
    for zone_spec in zones {
        by_origin
            .entry(zone_spec.origin.to_ascii_lowercase())
            .or_default()
            .push(zone_spec);
    }
    for (origin, specs) in by_origin {
        let mut merged = Vec::new();
        for spec in specs {
            merged.push(spec);
        }
        let mut owned = merged
            .first()
            .ok_or_else(|| invalid_config("empty DNS zone group"))?
            .to_owned()
            .clone();
        owned.records.extend(
            records
                .iter()
                .filter(|record| {
                    let fqdn = normalize_record_name(&record.name, &origin);
                    fqdn == origin || fqdn.ends_with(&format!(".{origin}"))
                })
                .cloned(),
        );
        let refs = vec![&owned];
        dns.add_zone(zone_from_config(&refs)?).await;
    }
    Ok(())
}

fn parse_acme_directory(value: Option<&str>) -> io::Result<DirectoryUrl> {
    match value.unwrap_or("letsencrypt").to_ascii_lowercase().as_str() {
        "letsencrypt" | "production" => Ok(DirectoryUrl::LetsEncrypt),
        "letsencrypt-staging" | "staging" => Ok(DirectoryUrl::LetsEncryptStaging),
        other => Err(invalid_config(format!(
            "unsupported ACME directory preset: {other}"
        ))),
    }
}

fn normalize_record_name(name: &str, origin: &str) -> String {
    let trimmed = name.trim_end_matches('.').to_ascii_lowercase();
    let origin = origin.trim_end_matches('.').to_ascii_lowercase();
    if trimmed == "@" || trimmed.is_empty() {
        origin
    } else if trimmed == origin || trimmed.ends_with(&format!(".{origin}")) {
        trimmed
    } else {
        format!("{trimmed}.{origin}")
    }
}

fn acme_io_error(error: edgerun_acme::AcmeError) -> io::Error {
    match error {
        edgerun_acme::AcmeError::Server(status, Some(detail)) => io::Error::new(
            io::ErrorKind::Other,
            format!(
                "ACME server error {status}: {}: {}",
                detail.error_type, detail.detail
            ),
        ),
        other => io::Error::new(io::ErrorKind::Other, other.to_string()),
    }
}

fn build_smtp_servers(spec: &SmtpServerSpec) -> io::Result<Vec<SmtpServer>> {
    let tls_cert = load_tls_from_spec(spec.tls_cert.as_deref(), spec.tls_key.as_deref())?;
    let mut config = SmtpServerConfig {
        bind_addr: spec
            .bind_address
            .clone()
            .unwrap_or_else(|| "0.0.0.0:25".to_string()),
        domain: spec.hostname.clone(),
        limits: ServerLimits {
            max_message_size: spec.max_message_size.unwrap_or(35_882_577),
            ..Default::default()
        },
        smtps: false,
        starttls: spec.starttls,
        local_domains: spec.local_domains.clone(),
        queue_data_root: if spec.relay_enabled {
            spec.queue_dir.as_ref().map(PathBuf::from)
        } else {
            None
        },
        relay_dns_server: spec
            .dns_server
            .clone()
            .unwrap_or_else(|| "127.0.0.1:53".to_string()),
        #[cfg(feature = "tls")]
        tls_cert: tls_cert.clone(),
        #[cfg(feature = "dkim")]
        dkim_signer: load_dkim_signer(spec)?,
        ..Default::default()
    };

    let handler = Arc::new(MaildirStore::new(required_path(
        spec.maildir_root.as_deref(),
        "SmtpServer.maildir_root",
    )?)?);
    register_smtp_users(&handler, spec)?;
    let mut servers = vec![SmtpServer::new(config.clone(), handler.clone())?];

    if spec.smtps {
        config.bind_addr = implicit_tls_addr(&config.bind_addr, 465);
        config.smtps = true;
        config.starttls = false;
        servers.push(SmtpServer::new(config, handler)?);
    }
    Ok(servers)
}

fn build_imap_servers(spec: &ImapServerSpec) -> io::Result<Vec<ImapServer>> {
    let tls_cert = load_tls_from_spec(spec.tls_cert.as_deref(), spec.tls_key.as_deref())?;
    let store = Arc::new(MaildirImapStore::new(required_path(
        spec.maildir_root.as_deref(),
        "ImapServer.maildir_root",
    )?)?);
    register_imap_users(&store, spec);
    let config = ImapServerConfig {
        bind_addr: spec
            .bind_address
            .clone()
            .unwrap_or_else(|| "0.0.0.0:143".to_string()),
        domain_name: spec.hostname.clone(),
        imaps: false,
        #[cfg(feature = "tls")]
        tls_cert: tls_cert.clone(),
        ..Default::default()
    };

    let mut servers = vec![ImapServer::with_store(config, store.clone())?];
    if spec.imaps {
        let config = ImapServerConfig {
            bind_addr: implicit_tls_addr(
                spec.bind_address.as_deref().unwrap_or("0.0.0.0:143"),
                993,
            ),
            domain_name: spec.hostname.clone(),
            imaps: true,
            #[cfg(feature = "tls")]
            tls_cert,
            ..Default::default()
        };
        servers.push(ImapServer::with_store(config, store)?);
    }
    Ok(servers)
}

fn register_smtp_users(store: &MaildirStore, spec: &SmtpServerSpec) -> io::Result<()> {
    let users = configured_users(spec.users.as_deref(), &spec.local_domains);
    for user in users {
        let domains = user
            .domains
            .as_deref()
            .unwrap_or(spec.local_domains.as_slice());
        let domain_refs: Vec<&str> = domains.iter().map(String::as_str).collect();
        store.add_user(&user.username, &domain_refs)?;
    }
    if let Some(username) = spec.catch_all_user.as_deref() {
        store.set_catch_all_user(username)?;
    }
    Ok(())
}

fn register_imap_users(store: &MaildirImapStore, spec: &ImapServerSpec) {
    let users = configured_users(spec.users.as_deref(), &[]);
    for user in users {
        if let Some(password) = user.password.as_deref() {
            store.add_user(&user.username, password);
        }
    }
}

fn configured_users(users: Option<&[MailUserSpec]>, local_domains: &[String]) -> Vec<MailUserSpec> {
    if let Some(users) = users {
        return users.to_vec();
    }
    let domains = if local_domains.is_empty() {
        None
    } else {
        Some(local_domains.to_vec())
    };
    vec![MailUserSpec {
        username: "postmaster".to_string(),
        password: None,
        domains,
    }]
}

#[cfg(feature = "dkim")]
fn load_dkim_signer(spec: &SmtpServerSpec) -> io::Result<Option<edgerun_email_auth::DkimSigner>> {
    let (Some(domain), Some(selector), Some(path)) = (
        spec.dkim_domain.as_deref(),
        spec.dkim_selector.as_deref(),
        spec.dkim_key_path.as_deref(),
    ) else {
        return Ok(None);
    };
    let pem = std::fs::read_to_string(path)?;
    edgerun_email_auth::DkimSigner::from_private_key_pem(domain, selector, &pem).map(Some)
}

fn load_tls_from_spec(
    cert: Option<&str>,
    key: Option<&str>,
) -> io::Result<Option<CertificateAndKey>> {
    let (Some(cert), Some(key)) = (cert, key) else {
        return Ok(None);
    };
    let cert_pem = read_pem_or_file(cert)?;
    let key_pem = read_pem_or_file(key)?;
    CertificateAndKey::from_pem(&format!("{cert_pem}\n{key_pem}"))
        .map(Some)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, format!("{error}")))
}

fn read_pem_or_file(value: &str) -> io::Result<String> {
    if value.contains("-----BEGIN ") {
        Ok(value.to_string())
    } else {
        std::fs::read_to_string(value)
    }
}

fn required_path<'a>(value: Option<&'a str>, field: &str) -> io::Result<&'a Path> {
    value.map(Path::new).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{field} is required for persistent mail service"),
        )
    })
}

fn record_value_string(record: &ZoneRecord) -> io::Result<String> {
    if let Some(value) = record.value.as_str() {
        return Ok(value.to_string());
    }
    if let Some(value) = record.value.as_i64() {
        return Ok(value.to_string());
    }
    if let Some(value) = record.value.as_u64() {
        return Ok(value.to_string());
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        format!(
            "DNS record {} {} must use a scalar value",
            record.name, record.record_type
        ),
    ))
}

fn parse_mx(value: &str) -> io::Result<(u16, String)> {
    let mut parts = value.split_whitespace();
    let priority = parts
        .next()
        .ok_or_else(|| invalid_config("MX value must be '<priority> <exchange>'"))?
        .parse::<u16>()
        .map_err(invalid_config)?;
    let exchange = parts
        .next()
        .ok_or_else(|| invalid_config("MX value must be '<priority> <exchange>'"))?;
    Ok((priority, exchange.to_string()))
}

fn parse_caa(value: &str) -> io::Result<(bool, String, String)> {
    let mut parts = value.splitn(3, char::is_whitespace);
    let flags = parts
        .next()
        .ok_or_else(|| invalid_config("CAA value must be '<flags> <tag> <value>'"))?
        .parse::<u8>()
        .map_err(invalid_config)?;
    let tag = parts
        .next()
        .ok_or_else(|| invalid_config("CAA value must be '<flags> <tag> <value>'"))?;
    let caa_value = parts
        .next()
        .ok_or_else(|| invalid_config("CAA value must be '<flags> <tag> <value>'"))?;
    Ok((flags & 0x80 != 0, tag.to_string(), caa_value.to_string()))
}

fn normalize_target(value: &str, origin: &str) -> String {
    let trimmed = value.trim_end_matches('.');
    if trimmed == "@" {
        origin.to_string()
    } else if trimmed.ends_with(origin) {
        trimmed.to_string()
    } else {
        format!("{trimmed}.{origin}")
    }
}

fn implicit_tls_addr(addr: &str, port: u16) -> String {
    if let Some((host, _)) = addr.rsplit_once(':') {
        format!("{host}:{port}")
    } else {
        format!("0.0.0.0:{port}")
    }
}

fn invalid_config(error: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, error.to_string())
}

fn to_io_error(error: impl std::fmt::Display) -> io::Error {
    io::Error::other(error.to_string())
}

fn init_material(args: &[String]) -> io::Result<()> {
    let domain = arg_value(args, "--domain").unwrap_or("edgerun.tech");
    let selector = arg_value(args, "--selector").unwrap_or("mail");
    let out_dir = PathBuf::from(arg_value(args, "--out-dir").unwrap_or("/etc/edgerun/mail"));
    let tls_dir = out_dir.join("tls");
    std::fs::create_dir_all(&tls_dir)?;

    let dkim = edgerun_email_auth::DkimSigner::generate(domain, selector).map_err(to_io_error)?;
    let dkim_key_path = out_dir.join(format!("dkim-{selector}.private.pem"));
    std::fs::write(&dkim_key_path, dkim.private_key_pem().map_err(to_io_error)?)?;

    let (cert_pem, key_pem) =
        edgerun_tls::generate_self_signed_pem(&[&format!("mail.{domain}"), domain])
            .map_err(to_io_error)?;
    std::fs::write(tls_dir.join("fullchain.pem"), cert_pem)?;
    std::fs::write(tls_dir.join("privkey.pem"), key_pem)?;
    println!("dkim_key_path={}", dkim_key_path.display());
    println!(
        "dkim_txt_name={selector}._domainkey.{domain}\ndkim_txt_value={}",
        dkim.public_key_txt()
    );
    println!("tls_cert={}", tls_dir.join("fullchain.pem").display());
    println!("tls_key={}", tls_dir.join("privkey.pem").display());
    Ok(())
}

fn arg_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].as_str())
}

#[allow(dead_code)]
async fn _keep_acme_dns_challenge_api_reachable(
    zone: &mut DnsZone,
    challenge: &edgerun_acme::DnsChallenge,
) {
    challenge.add_to_zone(zone);
}
