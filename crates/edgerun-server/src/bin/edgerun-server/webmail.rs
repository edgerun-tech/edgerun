//! Webmail HTTP handler and embedded client assets for the server binary.

use std::future::Future;
use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::Duration;

use edgerun_config::edgerun_json::JsonValue;
use edgerun_config::{ImapServerSpec, SmtpServerSpec};
use edgerun_email::smtp::types::MailEnvelope;
use edgerun_encoding::base64::{standard_decode, standard_encode_wrapped};
use edgerun_http::{Handler, Request, Response, StatusCode};

#[derive(Clone)]
pub(crate) struct WebmailConfig {
    pub(crate) hostname: String,
    username: String,
    password: String,
    address: String,
    maildir_root: PathBuf,
    smtp_addr: String,
    pub(crate) tls_cert: Option<String>,
    pub(crate) tls_key: Option<String>,
}

pub(crate) fn build_webmail_config(
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

#[derive(Clone)]
pub(crate) struct WebmailHandler {
    config: WebmailConfig,
}

impl WebmailHandler {
    pub(crate) fn new(config: WebmailConfig) -> Self {
        Self { config }
    }

    pub(crate) fn handle_sync(&self, request: Request) -> Response {
        let path = request.uri().request_target();
        if path == "/" || path == "/index.html" {
            return match request.method().as_str() {
                "GET" | "HEAD" => html_response(WEBMAIL_HTML),
                _ => method_not_allowed("GET, HEAD"),
            };
        }
        if path == "/bimi/logo.svg" {
            return match request.method().as_str() {
                "GET" | "HEAD" => svg_response(BIMI_LOGO_SVG),
                _ => method_not_allowed("GET, HEAD"),
            };
        }
        if path == "/.well-known/mta-sts.txt" {
            return match request.method().as_str() {
                "GET" | "HEAD" => mta_sts_policy_response(),
                _ => method_not_allowed("GET, HEAD"),
            };
        }
        if path.starts_with("/api/") && !authorized(&request, &self.config) {
            return unauthorized();
        }
        match (request.method().as_str(), path.as_str()) {
            ("GET", "/api/messages") => match list_webmail_messages(&self.config) {
                Ok(body) => json_response(&body),
                Err(error) => server_error(&error.to_string()),
            },
            ("GET", path) if path.starts_with("/api/message/") && path.contains("/attachment/") => {
                let rest = &path["/api/message/".len()..];
                let Some((id, index)) = rest.split_once("/attachment/") else {
                    return Response::not_found();
                };
                let id = percent_decode(id);
                let index = index.parse::<usize>().unwrap_or(usize::MAX);
                match read_webmail_attachment(&self.config, &id, index) {
                    Ok((attachment, data)) => attachment_response(&attachment, data),
                    Err(error) if error.kind() == io::ErrorKind::NotFound => Response::not_found(),
                    Err(error) => server_error(&error.to_string()),
                }
            }
            ("GET", path) if path.starts_with("/api/message/") => {
                let id = percent_decode(&path["/api/message/".len()..]);
                match read_webmail_message(&self.config, &id) {
                    Ok(body) => json_response(&body),
                    Err(error) if error.kind() == io::ErrorKind::NotFound => Response::not_found(),
                    Err(error) => server_error(&error.to_string()),
                }
            }
            ("POST", path) if path.starts_with("/api/message/") => {
                let rest = &path["/api/message/".len()..];
                let Some((id, action)) = rest.rsplit_once('/') else {
                    return Response::not_found();
                };
                let id = percent_decode(id);
                match apply_message_action(&self.config, &id, action) {
                    Ok(()) => json_response(r#"{"ok":true}"#),
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
        .with_header("X-Content-Type-Options", "nosniff")
}

pub(crate) fn method_not_allowed(allow: &str) -> Response {
    Response::text(StatusCode::new(405).unwrap(), "method not allowed")
        .with_header("Allow", allow)
        .with_header("Cache-Control", "no-store")
        .with_header("X-Content-Type-Options", "nosniff")
}

fn html_response(body: &str) -> Response {
    Response::html(StatusCode::OK, body)
        .with_header("Cache-Control", "no-store")
        .with_header("X-Content-Type-Options", "nosniff")
        .with_header("X-Frame-Options", "DENY")
        .with_header("Referrer-Policy", "no-referrer")
        .with_header("Permissions-Policy", "camera=(), microphone=(), geolocation=()")
        .with_header(
            "Strict-Transport-Security",
            "max-age=31536000; includeSubDomains",
        )
        .with_header(
            "Content-Security-Policy",
            "default-src 'self'; base-uri 'none'; object-src 'none'; frame-ancestors 'none'; img-src 'self' data:; style-src 'unsafe-inline'; script-src 'unsafe-inline'; connect-src 'self'; form-action 'self'",
        )
}

fn json_response(body: &str) -> Response {
    Response::json(StatusCode::OK, body)
        .with_header("Cache-Control", "no-store")
        .with_header("X-Content-Type-Options", "nosniff")
}

fn svg_response(body: &str) -> Response {
    Response::new(StatusCode::OK)
        .with_header("Content-Type", "image/svg+xml")
        .with_header("Cache-Control", "public, max-age=3600")
        .with_header("X-Content-Type-Options", "nosniff")
        .with_body(body.as_bytes().to_vec())
}

fn mta_sts_policy_response() -> Response {
    Response::new(StatusCode::OK)
        .with_header("Content-Type", "text/plain; charset=utf-8")
        .with_header("Cache-Control", "public, max-age=3600")
        .with_header("X-Content-Type-Options", "nosniff")
        .with_body(MTA_STS_POLICY.as_bytes().to_vec())
}

fn attachment_response(attachment: &MailAttachment, data: Vec<u8>) -> Response {
    Response::new(StatusCode::OK)
        .with_header("Content-Type", &attachment.content_type)
        .with_header(
            "Content-Disposition",
            &format!(
                r#"attachment; filename="{}""#,
                sanitize_quoted_header(&attachment.name)
            ),
        )
        .with_header("Cache-Control", "no-store")
        .with_header("X-Content-Type-Options", "nosniff")
        .with_body(data)
}

fn server_error(message: &str) -> Response {
    let body = format!(r#"{{"ok":false,"error":"{}"}}"#, json_escape(message));
    Response::json(StatusCode::INTERNAL_SERVER_ERROR, &body)
        .with_header("Cache-Control", "no-store")
        .with_header("X-Content-Type-Options", "nosniff")
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
        let warning = auth_warning_reason(&raw);
        let attachment_count = message_attachments(&raw).len();
        out.push_str(&format!(
            r#"{{"id":"{}","from":"{}","to":"{}","subject":"{}","date":"{}","state":"{}","unread":{},"warning":{},"warningReason":"{}","attachmentCount":{},"preview":"{}"}}"#,
            json_escape(&message.id),
            json_escape(header_value(&raw, "From").unwrap_or_default().as_str()),
            json_escape(header_value(&raw, "To").unwrap_or_default().as_str()),
            json_escape(header_value(&raw, "Subject").unwrap_or_else(|| "(no subject)".to_string()).as_str()),
            json_escape(header_value(&raw, "Date").unwrap_or_default().as_str()),
            message.state,
            if message.state == "new" { "true" } else { "false" },
            if warning.is_some() { "true" } else { "false" },
            json_escape(warning.unwrap_or_default().as_str()),
            attachment_count,
            json_escape(&message_preview(&raw)),
        ));
    }
    out.push_str("]}");
    Ok(out)
}

fn read_webmail_message(config: &WebmailConfig, id: &str) -> io::Result<String> {
    let path = find_maildir_message(config, id)?;
    let raw = std::fs::read_to_string(path)?;
    let warning = auth_warning_reason(&raw);
    let attachments = message_attachments(&raw);
    let mut attachments_json = String::new();
    for (idx, attachment) in attachments.iter().enumerate() {
        if idx > 0 {
            attachments_json.push(',');
        }
        attachments_json.push_str(&format!(
            r#"{{"index":{},"name":"{}","contentType":"{}","size":{}}}"#,
            attachment.index,
            json_escape(&attachment.name),
            json_escape(&attachment.content_type),
            attachment.size,
        ));
    }
    Ok(format!(
        r#"{{"id":"{}","from":"{}","to":"{}","subject":"{}","date":"{}","warning":{},"warningReason":"{}","attachments":[{}],"body":"{}","raw":"{}"}}"#,
        json_escape(id),
        json_escape(header_value(&raw, "From").unwrap_or_default().as_str()),
        json_escape(header_value(&raw, "To").unwrap_or_default().as_str()),
        json_escape(
            header_value(&raw, "Subject")
                .unwrap_or_else(|| "(no subject)".to_string())
                .as_str()
        ),
        json_escape(header_value(&raw, "Date").unwrap_or_default().as_str()),
        if warning.is_some() { "true" } else { "false" },
        json_escape(warning.unwrap_or_default().as_str()),
        attachments_json,
        json_escape(&message_body(&raw)),
        json_escape(&raw),
    ))
}

fn read_webmail_attachment(
    config: &WebmailConfig,
    id: &str,
    index: usize,
) -> io::Result<(MailAttachment, Vec<u8>)> {
    let path = find_maildir_message(config, id)?;
    let raw = std::fs::read_to_string(path)?;
    for attachment in message_attachments(&raw) {
        if attachment.index == index {
            let data = decode_attachment_body(&attachment)?;
            return Ok((attachment, data));
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "attachment not found",
    ))
}

fn apply_message_action(config: &WebmailConfig, id: &str, action: &str) -> io::Result<()> {
    match action {
        "read" => move_maildir_message(config, id, "cur"),
        "unread" => move_maildir_message(config, id, "new"),
        "delete" => {
            let path = find_maildir_message(config, id)?;
            std::fs::remove_file(path)
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unsupported message action: {action}"),
        )),
    }
}

fn send_webmail_message(config: &WebmailConfig, request: &Request) -> io::Result<()> {
    let body = request.body().unwrap_or_default();
    let value: JsonValue = edgerun_config::edgerun_json::from_json_slice(body)
        .map_err(|error| invalid_config(format!("invalid JSON: {error}")))?;
    let to = json_field(&value, "to")?;
    let subject = json_field(&value, "subject")?;
    let text = json_field(&value, "body")?;
    let attachments = json_attachments(&value)?;
    if !to.contains('@') || to.contains('\r') || to.contains('\n') {
        return Err(invalid_config("invalid recipient"));
    }
    let message_id = unique_webmail_id();
    let message = if attachments.is_empty() {
        format!(
            "From: {}\r\nTo: {}\r\nSubject: {}\r\nDate: {}\r\nMessage-ID: <{}@{}>\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: 8bit\r\n\r\n{}\r\n",
            config.address,
            sanitize_header(&to),
            sanitize_header(&subject),
            http_date_now(),
            message_id,
            config.hostname,
            normalize_crlf(&text),
        )
    } else {
        build_multipart_message(config, &to, &subject, &text, &message_id, &attachments)
    };
    let smtp_addr = config.smtp_addr.clone();
    let hostname = config.hostname.clone();
    let from = config.address.clone();
    std::thread::spawn(move || {
        if let Err(error) = submit_smtp(&smtp_addr, &hostname, &from, &to, &message) {
            eprintln!("edgerun-server: webmail SMTP submit failed: {error}");
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
struct OutgoingAttachment {
    name: String,
    content_type: String,
    data: Vec<u8>,
}

fn json_attachments(value: &JsonValue) -> io::Result<Vec<OutgoingAttachment>> {
    let Some(items) = value.get("attachments").and_then(JsonValue::as_array) else {
        return Ok(Vec::new());
    };
    if items.len() > 8 {
        return Err(invalid_config("too many attachments"));
    }
    let mut attachments = Vec::new();
    let mut total = 0usize;
    for item in items {
        let name = item
            .get("name")
            .and_then(JsonValue::as_str)
            .map(sanitize_attachment_name)
            .unwrap_or_else(|| "attachment.bin".to_string());
        let content_type = item
            .get("contentType")
            .and_then(JsonValue::as_str)
            .map(sanitize_content_type)
            .unwrap_or_else(|| "application/octet-stream".to_string());
        let data64 = item
            .get("data")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| invalid_config("attachment missing data"))?;
        let data = standard_decode(data64).map_err(invalid_config)?;
        total = total.saturating_add(data.len());
        if total > 15 * 1024 * 1024 {
            return Err(invalid_config("attachments exceed 15 MiB"));
        }
        attachments.push(OutgoingAttachment {
            name,
            content_type,
            data,
        });
    }
    Ok(attachments)
}

fn build_multipart_message(
    config: &WebmailConfig,
    to: &str,
    subject: &str,
    text: &str,
    message_id: &str,
    attachments: &[OutgoingAttachment],
) -> String {
    let boundary = format!("edgerun-{}-mixed", message_id);
    let mut message = format!(
        "From: {}\r\nTo: {}\r\nSubject: {}\r\nDate: {}\r\nMessage-ID: <{}@{}>\r\nMIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=\"{}\"\r\n\r\n",
        config.address,
        sanitize_header(to),
        sanitize_header(subject),
        http_date_now(),
        message_id,
        config.hostname,
        boundary,
    );
    message.push_str(&format!(
        "--{}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: 8bit\r\n\r\n{}\r\n",
        boundary,
        normalize_crlf(text),
    ));
    for attachment in attachments {
        let encoded = String::from_utf8(standard_encode_wrapped(&attachment.data))
            .unwrap_or_else(|_| String::new());
        message.push_str(&format!(
            "--{}\r\nContent-Type: {}; name=\"{}\"\r\nContent-Disposition: attachment; filename=\"{}\"\r\nContent-Transfer-Encoding: base64\r\n\r\n{}\r\n",
            boundary,
            attachment.content_type,
            sanitize_quoted_header(&attachment.name),
            sanitize_quoted_header(&attachment.name),
            encoded,
        ));
    }
    message.push_str(&format!("--{}--\r\n", boundary));
    message
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

fn move_maildir_message(config: &WebmailConfig, id: &str, target_state: &str) -> io::Result<()> {
    let path = find_maildir_message(config, id)?;
    let target_dir = config
        .maildir_root
        .join(&config.username)
        .join(target_state);
    std::fs::create_dir_all(&target_dir)?;
    let target = target_dir.join(id);
    if path == target {
        return Ok(());
    }
    std::fs::rename(path, target)
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
    if let Some(boundary) =
        header_value(raw, "Content-Type").and_then(|value| header_param(&value, "boundary"))
    {
        for part in multipart_parts(raw, &boundary) {
            let headers = part_headers(&part);
            let content_type = header_value(&headers, "Content-Type")
                .unwrap_or_else(|| "text/plain".to_string())
                .to_ascii_lowercase();
            let disposition = header_value(&headers, "Content-Disposition")
                .unwrap_or_default()
                .to_ascii_lowercase();
            if content_type.starts_with("text/plain") && !disposition.contains("attachment") {
                return part_body(&part).trim().to_string();
            }
        }
    }
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

#[derive(Clone)]
struct MailAttachment {
    index: usize,
    name: String,
    content_type: String,
    transfer_encoding: String,
    body: String,
    size: usize,
}

fn message_attachments(raw: &str) -> Vec<MailAttachment> {
    let Some(boundary) =
        header_value(raw, "Content-Type").and_then(|value| header_param(&value, "boundary"))
    else {
        return Vec::new();
    };
    let mut attachments = Vec::new();
    for part in multipart_parts(raw, &boundary) {
        let headers = part_headers(&part);
        let disposition = header_value(&headers, "Content-Disposition").unwrap_or_default();
        let content_type = header_value(&headers, "Content-Type")
            .unwrap_or_else(|| "application/octet-stream".to_string());
        let name =
            header_param(&disposition, "filename").or_else(|| header_param(&content_type, "name"));
        let is_attachment =
            disposition.to_ascii_lowercase().contains("attachment") || name.is_some();
        if !is_attachment {
            continue;
        }
        let transfer_encoding = header_value(&headers, "Content-Transfer-Encoding")
            .unwrap_or_default()
            .to_ascii_lowercase();
        let body = part_body(&part).trim().to_string();
        let size = if transfer_encoding == "base64" {
            standard_decode(
                &body
                    .chars()
                    .filter(|ch| !ch.is_whitespace())
                    .collect::<String>(),
            )
            .map(|data| data.len())
            .unwrap_or(0)
        } else {
            body.len()
        };
        attachments.push(MailAttachment {
            index: attachments.len(),
            name: name.unwrap_or_else(|| "attachment.bin".to_string()),
            content_type: content_type
                .split(';')
                .next()
                .unwrap_or("application/octet-stream")
                .trim()
                .to_string(),
            transfer_encoding,
            body,
            size,
        });
    }
    attachments
}

fn decode_attachment_body(attachment: &MailAttachment) -> io::Result<Vec<u8>> {
    if attachment.transfer_encoding == "base64" {
        let compact = attachment
            .body
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .collect::<String>();
        standard_decode(&compact).map_err(invalid_config)
    } else {
        Ok(attachment.body.as_bytes().to_vec())
    }
}

fn multipart_parts(raw: &str, boundary: &str) -> Vec<String> {
    let Some((_, body)) = raw
        .split_once("\r\n\r\n")
        .or_else(|| raw.split_once("\n\n"))
    else {
        return Vec::new();
    };
    let marker = format!("--{boundary}");
    body.split(&marker)
        .filter_map(|part| {
            let part = part.trim_start_matches(['\r', '\n']).trim_end();
            if part.is_empty() || part.starts_with("--") {
                None
            } else {
                Some(part.to_string())
            }
        })
        .collect()
}

fn part_headers(part: &str) -> String {
    part.split_once("\r\n\r\n")
        .or_else(|| part.split_once("\n\n"))
        .map(|(headers, _)| headers.to_string())
        .unwrap_or_default()
}

fn part_body(part: &str) -> &str {
    part.split_once("\r\n\r\n")
        .or_else(|| part.split_once("\n\n"))
        .map(|(_, body)| body)
        .unwrap_or_default()
}

fn header_param(value: &str, param: &str) -> Option<String> {
    for segment in value.split(';').skip(1) {
        if let Some((key, val)) = segment.split_once('=') {
            if key.trim().eq_ignore_ascii_case(param) {
                return Some(val.trim().trim_matches('"').trim_matches('\'').to_string());
            }
        }
    }
    None
}

fn auth_warning_reason(raw: &str) -> Option<String> {
    let auth = header_value(raw, "Authentication-Results")
        .or_else(|| header_value(raw, "X-Authentication-Results"))
        .unwrap_or_default()
        .to_ascii_lowercase();
    for marker in [
        "dmarc=fail",
        "spf=fail",
        "dkim=fail",
        "dmarc=permerror",
        "spf=permerror",
    ] {
        if auth.contains(marker) {
            return Some(format!("sender authentication reported {marker}"));
        }
    }

    let from_domain = header_value(raw, "From").and_then(|value| email_domain(&value));
    let return_path_domain =
        header_value(raw, "Return-Path").and_then(|value| email_domain(&value));
    if let (Some(from), Some(return_path)) = (from_domain, return_path_domain) {
        if !domains_align(&from, &return_path) {
            return Some(format!(
                "From domain {from} does not align with Return-Path domain {return_path}"
            ));
        }
    }
    None
}

fn email_domain(value: &str) -> Option<String> {
    let end = value.rfind('@')?;
    let domain = value[end + 1..]
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '.' && ch != '-')
        .to_ascii_lowercase();
    if domain.contains('.') {
        Some(domain)
    } else {
        None
    }
}

fn domains_align(left: &str, right: &str) -> bool {
    left == right || left.ends_with(&format!(".{right}")) || right.ends_with(&format!(".{left}"))
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

pub(crate) fn sanitize_header(value: &str) -> String {
    value.replace(['\r', '\n'], " ").trim().to_string()
}

fn sanitize_quoted_header(value: &str) -> String {
    sanitize_header(value).replace(['"', '\\'], "_")
}

fn sanitize_attachment_name(value: &str) -> String {
    let name = value
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("attachment.bin")
        .chars()
        .filter(|ch| ch.is_ascii_graphic() || *ch == ' ')
        .collect::<String>();
    let name = name.trim();
    if name.is_empty() {
        "attachment.bin".to_string()
    } else {
        name.chars().take(120).collect()
    }
}

fn sanitize_content_type(value: &str) -> String {
    let value = sanitize_header(value);
    if value.contains('/')
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '-' | '+' | '.'))
    {
        value
    } else {
        "application/octet-stream".to_string()
    }
}

pub(crate) fn normalize_crlf(value: &str) -> String {
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

pub(crate) fn unique_webmail_id() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_micros())
        .unwrap_or(0);
    format!("webmail-{now}")
}

pub(crate) fn http_date_now() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    edgerun_encoding::rfc2822::format_rfc2822_utc(seconds)
}

const WEBMAIL_HTML: &str = r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Edgerun Mail</title>
<link rel="icon" href='data:image/svg+xml,<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><text y="76" font-size="76">📧</text></svg>'>
<style>
:root{color-scheme:light dark;--bg:#f7f3eb;--panel:#fffdf8;--panel2:#f1eadc;--ink:#1c2430;--text:#1c2430;--muted:#627084;--line:#d8cfc0;--accent:#146c63;--accent-ink:#f4fffb;--accent2:#8b3f2f;--danger:#b42342;--code:#eee6d8}
:root[data-theme=dark]{--bg:#101418;--panel:#171d22;--panel2:#232b31;--ink:#f2ede4;--text:#f2ede4;--muted:#a5b2bf;--line:#2b353d;--accent:#6fc7b8;--accent-ink:#06201d;--accent2:#dfa06b;--danger:#fb7185;--code:#232b31}
*{box-sizing:border-box}body{margin:0;font:14px/1.45 ui-sans-serif,system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;background:var(--bg);color:var(--ink)}
.app{height:100vh;display:grid;grid-template-columns:minmax(280px,380px) 1fr}
.list{border-right:1px solid var(--line);background:var(--bg);display:flex;flex-direction:column;min-width:0}
.top{min-height:56px;display:grid;grid-template-columns:1fr auto;gap:10px;padding:10px 14px;border-bottom:1px solid var(--line)}
.brand{font-weight:700;font-size:16px}.who{color:var(--muted);font-size:12px;align-self:center;text-align:right}.mail-count{color:var(--muted);font-size:12px}
.search{grid-column:1/3;position:relative}.search svg{position:absolute;left:10px;top:50%;transform:translateY(-50%);width:15px;height:15px;color:var(--muted);stroke:currentColor;fill:none;stroke-width:2}.search input{padding-left:34px;height:34px}
button{border:1px solid var(--line);background:var(--panel2);border-radius:6px;padding:8px 10px;cursor:pointer;color:var(--ink);transition:background .12s ease,border-color .12s ease,opacity .12s ease}
button:hover:not(:disabled){border-color:var(--accent)}button.primary{background:var(--accent);border-color:var(--accent);color:var(--accent-ink);font-weight:650}button.danger:hover:not(:disabled){border-color:var(--danger);color:var(--danger)}button:disabled{opacity:.38;cursor:not-allowed}
.icon{width:32px;height:32px;padding:0;display:inline-grid;place-items:center}.icon svg{width:17px;height:17px;stroke:currentColor;fill:none;stroke-width:2;stroke-linecap:round;stroke-linejoin:round}.copy{font-size:13px}
.messages{overflow:auto;min-height:0}.item{padding:12px 14px;border-bottom:1px solid var(--line);cursor:pointer;transition:background .12s ease,opacity .12s ease;display:grid;grid-template-columns:22px 1fr;gap:2px 8px}
.item:hover,.item.active,.item.checked{background:color-mix(in srgb,var(--accent) 12%,var(--panel))}.item.pending{opacity:.55}.item.unread .from,.item.unread .subject{font-weight:750}.item:not(.unread) .from,.item:not(.unread) .subject{font-weight:450}.pick{grid-row:1/4;align-self:start;margin:2px 0 0;width:auto}.from{white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.subject{margin-top:2px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}.preview{margin-top:4px;color:var(--muted);font-size:12px;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden}.paperclip{color:var(--muted);margin-left:5px}.paperclip svg{width:13px;height:13px;stroke:currentColor;fill:none;stroke-width:2;vertical-align:-2px}
.warn{color:var(--accent2);font-weight:700;margin-right:5px}.item .warn{font-size:13px}.warn svg,.warning svg{stroke:currentColor;fill:none;stroke-width:2;stroke-linecap:round;stroke-linejoin:round;vertical-align:-2px}
.pane{min-width:0;display:grid;grid-template-rows:auto 1fr;background:var(--panel)}
.actions{height:56px;display:flex;align-items:center;gap:8px;padding:0 16px;border-bottom:1px solid var(--line)}
.content{overflow:auto;padding:22px;max-width:980px;width:100%}.empty{color:var(--muted);margin-top:20vh;text-align:center}
h1{font-size:22px;margin:0 0 8px}.meta{color:var(--muted);margin-bottom:18px;display:grid;gap:3px}
.meta-line{display:flex;align-items:center;gap:7px;flex-wrap:wrap}.warning{border:1px solid var(--accent2);background:color-mix(in srgb,var(--accent2) 12%,var(--panel));color:var(--accent2);border-radius:6px;padding:9px 10px;margin:0 0 14px}
pre{white-space:pre-wrap;word-break:break-word;font:14px/1.5 ui-monospace,SFMono-Regular,Menlo,monospace;background:var(--code);border:1px solid var(--line);border-radius:6px;padding:14px}
.compose{display:none;padding:16px;border-bottom:1px solid var(--line);background:var(--panel)}.compose.open{display:grid;gap:10px}
.compose-grid{display:grid;grid-template-columns:96px 1fr;gap:10px;align-items:center}.compose-grid textarea,.compose-grid .attach-row{grid-column:1/3}
label{color:var(--muted);font-size:12px;text-transform:uppercase;letter-spacing:.04em}
input,textarea,select{width:100%;border:1px solid var(--line);border-radius:6px;padding:10px;font:inherit;background:var(--bg);color:var(--ink)}
input:focus,textarea:focus{outline:2px solid rgba(45,212,191,.28);border-color:var(--accent)}textarea{min-height:190px;resize:vertical}.row{display:flex;gap:8px;align-items:center}.status{color:var(--muted);font-size:13px;min-width:82px}.status.warn{color:var(--accent2)}.grow{flex:1}.attachments{display:flex;flex-wrap:wrap;gap:6px}.chip,.attachment{border:1px solid var(--line);border-radius:6px;background:var(--code);color:var(--muted);padding:5px 8px;font-size:12px}.attachment{display:inline-flex;align-items:center;gap:7px;color:var(--ink);text-decoration:none;margin:0 6px 6px 0}.chip button{border:0;background:transparent;color:var(--muted);padding:0;margin-left:6px;width:auto;height:auto}.attach-input{display:none}
.spin svg{animation:spin .8s linear infinite}@keyframes spin{to{transform:rotate(360deg)}}
@media(max-width:760px){.app{grid-template-columns:1fr;grid-template-rows:45vh 55vh}.list{border-right:0;border-bottom:1px solid var(--line)}.content{padding:16px}}
</style>
</head>
<body>
<svg aria-hidden="true" style="position:absolute;width:0;height:0;overflow:hidden">
<symbol id="i-refresh" viewBox="0 0 24 24"><path d="M21 12a9 9 0 0 1-15.4 6.4L3 16"/><path d="M3 21v-5h5"/><path d="M3 12A9 9 0 0 1 18.4 5.6L21 8"/><path d="M21 3v5h-5"/></symbol>
<symbol id="i-edit" viewBox="0 0 24 24"><path d="M12 20h9"/><path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z"/></symbol>
<symbol id="i-send" viewBox="0 0 24 24"><path d="m22 2-7 20-4-9-9-4Z"/><path d="M22 2 11 13"/></symbol>
<symbol id="i-reply" viewBox="0 0 24 24"><path d="m9 17-6-5 6-5"/><path d="M3 12h11a7 7 0 0 1 7 7v1"/></symbol>
<symbol id="i-x" viewBox="0 0 24 24"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></symbol>
<symbol id="i-copy" viewBox="0 0 24 24"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></symbol>
<symbol id="i-trash" viewBox="0 0 24 24"><path d="M3 6h18"/><path d="M8 6V4h8v2"/><path d="m19 6-1 14H6L5 6"/><path d="M10 11v6"/><path d="M14 11v6"/></symbol>
<symbol id="i-mail-open" viewBox="0 0 24 24"><path d="M4 6 12 2l8 4v14H4Z"/><path d="m4 8 8 6 8-6"/></symbol>
<symbol id="i-mail" viewBox="0 0 24 24"><rect x="3" y="5" width="18" height="14" rx="2"/><path d="m3 7 9 6 9-6"/></symbol>
<symbol id="i-log-out" viewBox="0 0 24 24"><path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><path d="M16 17l5-5-5-5"/><path d="M21 12H9"/></symbol>
<symbol id="i-alert" viewBox="0 0 24 24"><path d="m21.7 18-8-14a2 2 0 0 0-3.4 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.7-3Z"/><path d="M12 9v4"/><path d="M12 17h.01"/></symbol>
<symbol id="i-search" viewBox="0 0 24 24"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></symbol>
<symbol id="i-paperclip" viewBox="0 0 24 24"><path d="m21.4 11.6-8.8 8.8a6 6 0 0 1-8.5-8.5l9.2-9.2a4 4 0 0 1 5.7 5.7l-9.2 9.2a2 2 0 0 1-2.8-2.8l8.5-8.5"/></symbol>
</svg>
<main class="app">
  <section class="list">
    <div class="top"><div><div class="brand">Edgerun Mail</div><div id="mailCount" class="mail-count"></div></div><div class="who">ken@edgerun.tech</div><div class="search"><svg><use href="#i-search"/></svg><input id="search" placeholder="Search mail" autocomplete="off"></div></div>
    <div id="messages" class="messages"></div>
  </section>
  <section class="pane">
    <div class="actions">
      <button id="composeBtn" class="icon primary" title="Compose" aria-label="Compose"><svg><use href="#i-edit"/></svg></button>
      <button id="replyMsg" class="icon" title="Reply" aria-label="Reply" disabled><svg><use href="#i-reply"/></svg></button>
      <button id="refresh" class="icon" title="Refresh" aria-label="Refresh"><svg><use href="#i-refresh"/></svg></button>
      <button id="markRead" class="icon" title="Mark read" aria-label="Mark read" disabled><svg><use href="#i-mail-open"/></svg></button>
      <button id="markUnread" class="icon" title="Mark unread" aria-label="Mark unread" disabled><svg><use href="#i-mail"/></svg></button>
      <button id="deleteMsg" class="icon danger" title="Delete" aria-label="Delete" disabled><svg><use href="#i-trash"/></svg></button>
      <div id="status" class="status"></div><div class="grow"></div>
      <er-theme-toggle></er-theme-toggle>
      <button id="logout" class="icon" title="Log out" aria-label="Log out"><svg><use href="#i-log-out"/></svg></button>
    </div>
    <form id="compose" class="compose">
      <div class="compose-grid">
        <label>From</label><div>ken@edgerun.tech</div>
        <label for="to">To</label><input id="to" placeholder="recipient@example.com" autocomplete="off">
        <label for="subject">Subject</label><input id="subject" placeholder="Subject" autocomplete="off">
        <textarea id="body" placeholder="Message"></textarea>
        <div class="attach-row"><input id="filePick" class="attach-input" type="file" multiple><button id="attachBtn" class="icon" type="button" title="Attach files" aria-label="Attach files"><svg><use href="#i-paperclip"/></svg></button><div id="composeAttachments" class="attachments"></div></div>
      </div>
      <div class="row"><button id="sendBtn" class="icon primary" type="submit" title="Send" aria-label="Send"><svg><use href="#i-send"/></svg></button><button id="cancel" class="icon" type="button" title="Cancel" aria-label="Cancel"><svg><use href="#i-x"/></svg></button><div class="grow"></div></div>
    </form>
    <article id="content" class="content"><div class="empty">Select a message</div></article>
  </section>
</main>
<script>
const root=document.documentElement;
const storedTheme=localStorage.getItem('theme');
if(storedTheme){root.dataset.theme=storedTheme}
if(!customElements.get('er-theme-toggle')){customElements.define('er-theme-toggle',class extends HTMLElement{connectedCallback(){this.attachShadow({mode:'open'}).innerHTML='<style>button{width:32px;height:32px;border:1px solid var(--line);border-radius:6px;background:var(--panel2);color:var(--ink);cursor:pointer;font:inherit}button:hover{border-color:var(--accent)}</style><button type="button" aria-label="Toggle color theme">◐</button>';this.shadowRoot.querySelector('button').onclick=()=>{const next=root.dataset.theme==='dark'?'light':'dark';root.dataset.theme=next;localStorage.setItem('theme',next)}}})}
const messagesEl=document.getElementById('messages'),content=document.getElementById('content'),statusEl=document.getElementById('status'),mailCount=document.getElementById('mailCount');
const refreshBtn=document.getElementById('refresh'),composeBtn=document.getElementById('composeBtn'),replyMsg=document.getElementById('replyMsg'),composeEl=document.getElementById('compose'),searchEl=document.getElementById('search'),filePick=document.getElementById('filePick'),composeAttachments=document.getElementById('composeAttachments');
const markRead=document.getElementById('markRead'),markUnread=document.getElementById('markUnread'),deleteMsg=document.getElementById('deleteMsg'),logout=document.getElementById('logout'),attachBtn=document.getElementById('attachBtn'),cancelBtn=document.getElementById('cancel'),sendBtn=document.getElementById('sendBtn');
const toEl=document.getElementById('to'),subjectEl=document.getElementById('subject'),bodyEl=document.getElementById('body');
let selected='',messages=[],checked=new Set(),draftAttachments=[],loading=false,sending=false,statusTimer=0,openedMessage=null;
function esc(s){return (s||'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));}
function flash(s,warn){clearTimeout(statusTimer);statusEl.textContent=s||'';statusEl.classList.toggle('warn',!!warn);if(s)statusTimer=setTimeout(()=>{statusEl.textContent='';statusEl.classList.remove('warn')},1800)}
function fmtSize(n){return n>1048576?(n/1048576).toFixed(1)+' MB':n>1024?Math.round(n/1024)+' KB':n+' B'}
function copyText(v){navigator.clipboard&&navigator.clipboard.writeText(v||'');flash('Copied')}
async function api(path,opts){const r=await fetch(path,opts);if(r.status===401){statusEl.textContent='Login required';throw new Error('auth');}if(!r.ok)throw new Error(await r.text());return r.json();}
function selectedMsg(){return messages.find(m=>m.id===selected)}
function actionIds(){return checked.size?[...checked]:(selected?[selected]:[])}
function setActionState(){const ids=actionIds(),items=ids.map(id=>messages.find(m=>m.id===id)).filter(Boolean);replyMsg.disabled=!selected;markRead.disabled=!items.some(m=>m.unread);markUnread.disabled=!items.some(m=>!m.unread);deleteMsg.disabled=!items.length}
function filteredMessages(){const q=searchEl.value.trim().toLowerCase();if(!q)return messages;return messages.filter(m=>[m.from,m.to,m.subject,m.preview,m.date].some(v=>(v||'').toLowerCase().includes(q)))}
function updateDraftStore(){localStorage.setItem('webmailDraft',JSON.stringify({to:toEl.value,subject:subjectEl.value,body:bodyEl.value,attachments:draftAttachments}))}
function restoreDraft(){try{const raw=localStorage.getItem('webmailDraft');if(!raw)return;const draft=JSON.parse(raw);toEl.value=draft.to||'';subjectEl.value=draft.subject||'';bodyEl.value=draft.body||'';draftAttachments=Array.isArray(draft.attachments)?draft.attachments:[];renderDraftAttachments()}catch(e){}}
function clearDraft(){toEl.value=subjectEl.value=bodyEl.value='';draftAttachments=[];renderDraftAttachments();localStorage.removeItem('webmailDraft')}
function openCompose(){composeEl.classList.add('open');toEl.focus()}
function closeCompose(){if(toEl.value||subjectEl.value||bodyEl.value||draftAttachments.length){updateDraftStore()}composeEl.classList.remove('open')}
function renderList(){const unread=messages.filter(m=>m.unread).length;document.title=(unread?`(${unread}) `:'')+'Edgerun Mail';mailCount.textContent=`${messages.length} total · ${unread} unread`;const visible=filteredMessages();messagesEl.innerHTML=visible.map(m=>`<div class="item ${m.unread?'unread':''} ${m.pending?'pending':''} ${m.id===selected?'active':''} ${checked.has(m.id)?'checked':''}" data-id="${encodeURIComponent(m.id)}"><input class="pick" type="checkbox" aria-label="Select message" ${checked.has(m.id)?'checked':''}><div class="from">${m.warning?'<span class="warn" title="'+esc(m.warningReason)+'"><svg width="13" height="13"><use href="#i-alert"/></svg></span>':''}${esc(m.from||'(unknown)')}${m.attachmentCount?'<span class="paperclip" title="'+m.attachmentCount+' attachment(s)"><svg><use href="#i-paperclip"/></svg></span>':''}</div><div class="subject">${esc(m.subject)}</div><div class="preview">${esc(m.preview)}</div></div>`).join('')||'<div class="empty">'+(messages.length?'No matches':'No mail')+'</div>';setActionState()}
async function load(silent){if(loading)return;loading=true;refreshBtn.classList.add('spin');try{const data=await api('/api/messages');const pending=new Map(messages.filter(m=>m.pending).map(m=>[m.id,m]));messages=data.messages.map(m=>pending.get(m.id)||m);if(selected&&!messages.some(m=>m.id===selected)){selected='';openedMessage=null;content.innerHTML='<div class="empty">Select a message</div>'}renderList();if(!silent)flash('Updated')}catch(e){if(!silent)flash(e.message,true)}finally{loading=false;refreshBtn.classList.remove('spin')}}
messagesEl.onclick=e=>{const item=e.target.closest('.item');if(!item)return;const id=decodeURIComponent(item.dataset.id);if(e.target.closest('.pick')){checked.has(id)?checked.delete(id):checked.add(id);renderList();return}openMsg(id);};
async function openMsg(id){selected=id;openedMessage=null;const local=messages.find(m=>m.id===id);if(local&&local.unread){local.unread=false;renderList();api('/api/message/'+encodeURIComponent(id)+'/read',{method:'POST'}).catch(()=>{local.unread=true;renderList();flash('Could not mark read',true)})}else renderList();content.innerHTML='<div class="empty">Loading...</div>';try{const m=await api('/api/message/'+encodeURIComponent(id));if(selected!==id)return;openedMessage=m;const attachments=(m.attachments||[]).map(a=>`<a class="attachment" href="/api/message/${encodeURIComponent(id)}/attachment/${a.index}" download="${esc(a.name)}"><svg width="15" height="15"><use href="#i-paperclip"/></svg>${esc(a.name)} <span>${fmtSize(a.size)}</span></a>`).join('');content.innerHTML=`<h1>${m.warning?'<span class="warn" title="'+esc(m.warningReason)+'"><svg width="18" height="18"><use href="#i-alert"/></svg></span>':''}${esc(m.subject)}</h1>${m.warning?'<div class="warning"><svg width="16" height="16"><use href="#i-alert"/></svg> '+esc(m.warningReason)+'</div>':''}<div class="meta"><div class="meta-line">From: <span>${esc(m.from)}</span><button class="icon copy" data-copy="${esc(m.from)}" title="Copy sender" aria-label="Copy sender"><svg><use href="#i-copy"/></svg></button></div><div class="meta-line">To: <span>${esc(m.to)}</span><button class="icon copy" data-copy="${esc(m.to)}" title="Copy recipient" aria-label="Copy recipient"><svg><use href="#i-copy"/></svg></button></div><div>${esc(m.date)}</div></div>${attachments?'<div class="attachments">'+attachments+'</div>':''}<pre>${esc(m.body||m.raw)}</pre>`;setActionState()}catch(e){content.innerHTML='<div class="empty">Could not open message</div>';flash(e.message,true)}}
document.addEventListener('click',e=>{const b=e.target.closest('.copy');if(b)copyText(b.dataset.copy||'')});
refreshBtn.onclick=()=>load(false);
searchEl.oninput=renderList;
composeBtn.onclick=()=>openCompose();
replyMsg.onclick=async()=>{let m=openedMessage;if(!m&&selected)m=await api('/api/message/'+encodeURIComponent(selected));if(!m)return;toEl.value=(m.from||'').replace(/^.*<([^>]+)>.*$/,'$1');subjectEl.value=/^re:/i.test(m.subject||'')?m.subject:'Re: '+(m.subject||'');bodyEl.value=`\n\nOn ${m.date||'the original message'}, ${m.from||'sender'} wrote:\n`+(m.body||'').split('\n').map(line=>'> '+line).join('\n');draftAttachments=[];renderDraftAttachments();updateDraftStore();openCompose()};
cancelBtn.onclick=closeCompose;
async function action(name){const ids=actionIds();if(!ids.length)return;const old=messages.map(m=>({...m}));if(name==='delete'){messages=messages.filter(m=>!ids.includes(m.id));checked.clear();selected='';content.innerHTML='<div class="empty">Select a message</div>'}else messages.forEach(m=>{if(ids.includes(m.id)){m.pending=true;m.unread=name==='unread'}});renderList();try{await Promise.all(ids.map(id=>api('/api/message/'+encodeURIComponent(id)+'/'+name,{method:'POST'})));flash(name==='delete'?'Deleted':name==='read'?'Marked read':'Marked unread');load(true)}catch(e){messages=old;renderList();flash(e.message,true)}}
markRead.onclick=()=>action('read');markUnread.onclick=()=>action('unread');deleteMsg.onclick=()=>{const n=actionIds().length;if(n&&confirm('Delete '+n+' message'+(n>1?'s':'')+'?'))action('delete')};
logout.onclick=async()=>{selected='';content.innerHTML='<div class="empty">Logged out</div>';messagesEl.innerHTML='';statusEl.textContent='Logged out';try{await fetch('/api/messages',{headers:{Authorization:'Basic '+btoa('logout:logout')}})}catch(e){}};
function renderDraftAttachments(){composeAttachments.innerHTML=draftAttachments.map((a,i)=>`<span class="chip"><svg width="13" height="13"><use href="#i-paperclip"/></svg> ${esc(a.name)} ${fmtSize(a.size)} <button type="button" data-idx="${i}" title="Remove attachment" aria-label="Remove attachment">×</button></span>`).join('')}
attachBtn.onclick=()=>filePick.click();
composeAttachments.onclick=e=>{const b=e.target.closest('button');if(b){draftAttachments.splice(Number(b.dataset.idx),1);renderDraftAttachments();updateDraftStore()}};
filePick.onchange=async()=>{for(const file of filePick.files){const data=await new Promise((ok,bad)=>{const r=new FileReader();r.onload=()=>ok(String(r.result).split(',')[1]||'');r.onerror=bad;r.readAsDataURL(file)});draftAttachments.push({name:file.name,contentType:file.type||'application/octet-stream',size:file.size,data})}filePick.value='';renderDraftAttachments();updateDraftStore();flash('Attached')};
[toEl,subjectEl,bodyEl].forEach(el=>el.addEventListener('input',updateDraftStore));
composeEl.onsubmit=async e=>{e.preventDefault();if(sending)return;sending=true;sendBtn.disabled=true;const draft={to:toEl.value,subject:subjectEl.value,body:bodyEl.value,attachments:draftAttachments.map(({name,contentType,data})=>({name,contentType,data}))};flash('Sending...');composeEl.classList.remove('open');try{await api('/api/send',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(draft)});clearDraft();flash('Sent');load(true)}catch(err){composeEl.classList.add('open');flash(err.message,true)}finally{sending=false;sendBtn.disabled=false}};
restoreDraft();
load(false).catch(err=>flash(err.message,true));
setInterval(()=>{if(!document.hidden)load(true)},10000);
</script>
</body>
</html>"##;

const BIMI_LOGO_SVG: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" version="1.2" baseProfile="tiny-ps" width="256" height="256" viewBox="0 0 256 256">
  <title>Edgerun</title>
  <desc>Edgerun square email brand mark</desc>
  <rect width="256" height="256" fill="#0f172a"/>
  <path fill="#22d3ee" d="M64 56h128v32H96v24h80v32H96v24h96v32H64z"/>
  <path fill="#f8fafc" d="M112 104h64v32h-64z"/>
</svg>
"##;

const MTA_STS_POLICY: &str =
    "version: STSv1\nmode: enforce\nmx: mail.edgerun.tech\nmax_age: 604800\n";

fn invalid_config(error: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, error.to_string())
}
