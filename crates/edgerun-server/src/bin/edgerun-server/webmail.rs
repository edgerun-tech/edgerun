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
use edgerun_web_ui::{FooterLink, PageShell};

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

    pub(crate) fn hostname(&self) -> &str {
        &self.config.hostname
    }

    pub(crate) fn handle_sync(&self, request: Request) -> Response {
        let target = request.uri().request_target();
        let path = target.split('?').next().unwrap_or(target.as_str());
        let embedded = target.contains("workspace=1");
        if path == "/" || path == "/index.html" {
            return match request.method().as_str() {
                "GET" | "HEAD" => html_response(&render_webmail_html(), embedded),
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
        match (request.method().as_str(), path) {
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

    pub(crate) fn handle_dash_mail(&self, request: Request) -> Response {
        let target = request.uri().request_target();
        let path = target.split('?').next().unwrap_or(target.as_str());
        if !authorized(&request, &self.config) {
            return match request.method().as_str() {
                "GET" | "HEAD" => dash_mail_response(&render_dash_mail_login()),
                _ => unauthorized(),
            };
        }
        match (request.method().as_str(), path) {
            ("GET" | "HEAD", "/surface/mail") => match render_dash_mail_inbox(&self.config, None) {
                Ok(body) => dash_mail_response(&body),
                Err(error) => dash_mail_response(&render_dash_mail_error(&error.to_string())),
            },
            ("GET" | "HEAD", "/surface/mail/compose") => {
                dash_mail_detail_response(&render_dash_mail_compose(None, None, None))
            }
            ("GET" | "HEAD", path) if path.starts_with("/surface/mail/reply/") => {
                let id = percent_decode(&path["/surface/mail/reply/".len()..]);
                match dash_mail_detail(&self.config, &id) {
                    Ok(message) => dash_mail_detail_response(&render_dash_mail_compose(
                        Some(&reply_recipient(&message.from)),
                        Some(&reply_subject(&message.subject)),
                        Some(&message.body),
                    )),
                    Err(error) if error.kind() == io::ErrorKind::NotFound => Response::not_found(),
                    Err(error) => {
                        dash_mail_detail_response(&render_dash_mail_error(&error.to_string()))
                    }
                }
            }
            ("GET" | "HEAD", path) if path.starts_with("/surface/mail/message/") => {
                let id = percent_decode(&path["/surface/mail/message/".len()..]);
                let _ = apply_message_action(&self.config, &id, "read");
                match dash_mail_detail(&self.config, &id) {
                    Ok(message) => dash_mail_detail_response(&render_dash_mail_message(&message)),
                    Err(error) if error.kind() == io::ErrorKind::NotFound => Response::not_found(),
                    Err(error) => {
                        dash_mail_detail_response(&render_dash_mail_error(&error.to_string()))
                    }
                }
            }
            ("POST", "/surface/mail/send") => match send_webmail_message(&self.config, &request) {
                Ok(()) => {
                    match render_dash_mail_inbox(&self.config, Some("Message queued for delivery."))
                    {
                        Ok(body) => dash_mail_response(&body),
                        Err(error) => {
                            dash_mail_response(&render_dash_mail_error(&error.to_string()))
                        }
                    }
                }
                Err(error) => {
                    dash_mail_detail_response(&render_dash_mail_error(&error.to_string()))
                }
            },
            ("POST", path) if path.starts_with("/surface/mail/message/") => {
                let rest = &path["/surface/mail/message/".len()..];
                let Some((id, action)) = rest.rsplit_once('/') else {
                    return Response::not_found();
                };
                let id = percent_decode(id);
                match apply_message_action(&self.config, &id, action) {
                    Ok(()) => match render_dash_mail_inbox(&self.config, None) {
                        Ok(body) => dash_mail_response(&body),
                        Err(error) => {
                            dash_mail_response(&render_dash_mail_error(&error.to_string()))
                        }
                    },
                    Err(error) if error.kind() == io::ErrorKind::NotFound => Response::not_found(),
                    Err(error) => dash_mail_response(&render_dash_mail_error(&error.to_string())),
                }
            }
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

fn html_response(body: &str, embedded: bool) -> Response {
    let response = Response::html(StatusCode::OK, body)
        .with_header("Cache-Control", "no-store")
        .with_header("X-Content-Type-Options", "nosniff")
        .with_header("Referrer-Policy", "no-referrer")
        .with_header(
            "Permissions-Policy",
            "camera=(), microphone=(), geolocation=()",
        )
        .with_header(
            "Strict-Transport-Security",
            "max-age=31536000; includeSubDomains",
        );
    if embedded {
        response.with_header(
            "Content-Security-Policy",
            "default-src 'self'; base-uri 'none'; object-src 'none'; frame-ancestors https://dash.edgerun.tech; img-src 'self' data:; style-src 'unsafe-inline'; script-src 'unsafe-inline'; connect-src 'self'; form-action 'self'",
        )
    } else {
        response
            .with_header("X-Frame-Options", "DENY")
            .with_header(
            "Content-Security-Policy",
            "default-src 'self'; base-uri 'none'; object-src 'none'; frame-ancestors 'none'; img-src 'self' data:; style-src 'unsafe-inline'; script-src 'unsafe-inline'; connect-src 'self'; form-action 'self'",
        )
    }
}

fn json_response(body: &str) -> Response {
    Response::json(StatusCode::OK, body)
        .with_header("Cache-Control", "no-store")
        .with_header("X-Content-Type-Options", "nosniff")
}

fn dash_html_response(body: &str) -> Response {
    Response::html(StatusCode::OK, body)
        .with_header("Cache-Control", "no-store")
        .with_header("X-Content-Type-Options", "nosniff")
        .with_header("Referrer-Policy", "no-referrer")
}

fn dash_mail_response(content: &str) -> Response {
    dash_html_response(&format!(
        "<section id=\"surfaceSlot\" class=\"dash-stage\" aria-label=\"Mail workspace\"><header><div><strong id=\"surfaceTitle\">Mail</strong><span id=\"surfaceUrl\">backend: mail.edgerun.tech</span></div></header><div class=\"dash-surface\">{}</div></section>",
        content
    ))
}

fn dash_mail_detail_response(content: &str) -> Response {
    dash_html_response(content)
}

#[derive(Clone)]
struct DashMailSummary {
    id: String,
    from: String,
    subject: String,
    date: String,
    unread: bool,
    warning: Option<String>,
    attachment_count: usize,
    preview: String,
}

struct DashMailDetail {
    id: String,
    from: String,
    to: String,
    subject: String,
    date: String,
    warning: Option<String>,
    attachments: Vec<MailAttachment>,
    body: String,
}

fn dash_mail_summaries(config: &WebmailConfig) -> io::Result<Vec<DashMailSummary>> {
    let mut messages = Vec::new();
    collect_maildir_entries(config, "new", &mut messages)?;
    collect_maildir_entries(config, "cur", &mut messages)?;
    messages.sort_by(|a, b| b.modified.cmp(&a.modified));
    messages.truncate(100);
    let mut summaries = Vec::new();
    for message in messages {
        let raw = std::fs::read_to_string(&message.path).unwrap_or_default();
        summaries.push(DashMailSummary {
            id: message.id,
            from: header_value(&raw, "From").unwrap_or_default(),
            subject: header_value(&raw, "Subject").unwrap_or_else(|| "(no subject)".to_string()),
            date: header_value(&raw, "Date").unwrap_or_default(),
            unread: message.state == "new",
            warning: auth_warning_reason(&raw),
            attachment_count: message_attachments(&raw).len(),
            preview: message_preview(&raw),
        });
    }
    Ok(summaries)
}

fn dash_mail_detail(config: &WebmailConfig, id: &str) -> io::Result<DashMailDetail> {
    let path = find_maildir_message(config, id)?;
    let raw = std::fs::read_to_string(path)?;
    Ok(DashMailDetail {
        id: id.to_string(),
        from: header_value(&raw, "From").unwrap_or_default(),
        to: header_value(&raw, "To").unwrap_or_default(),
        subject: header_value(&raw, "Subject").unwrap_or_else(|| "(no subject)".to_string()),
        date: header_value(&raw, "Date").unwrap_or_default(),
        warning: auth_warning_reason(&raw),
        attachments: message_attachments(&raw),
        body: message_body(&raw),
    })
}

fn render_dash_mail_login() -> String {
    "<form class=\"dash-mail-login\" data-mail-login><strong>Unlock mail</strong><label>Username<input name=\"username\" autocomplete=\"username\" value=\"ken\"></label><label>Password<input name=\"password\" type=\"password\" autocomplete=\"current-password\"></label><button class=\"dash-mail-button\" type=\"submit\">Unlock mail</button></form>".to_string()
}

fn render_dash_mail_inbox(config: &WebmailConfig, notice: Option<&str>) -> io::Result<String> {
    let messages = dash_mail_summaries(config)?;
    let mut list = String::new();
    for message in &messages {
        let encoded = percent_encode(&message.id);
        let unread = if message.unread {
            "<span class=\"dash-mail-unread\">Unread</span>"
        } else {
            ""
        };
        let warning = if message.warning.is_some() {
            "<span>Warning</span>"
        } else {
            ""
        };
        let attachments = if message.attachment_count > 0 {
            format!("<span>{} attachments</span>", message.attachment_count)
        } else {
            String::new()
        };
        list.push_str(&format!(
            "<a class=\"dash-mail-item\" href=\"https://dash.edgerun.tech/#mail\" hx-get=\"/surface/mail/message/{}\" hx-target=\"#dashMailDetail\" hx-swap=\"outerHTML\"><strong>{}</strong><span>{}</span><div class=\"dash-mail-meta\">{}<span>{}</span>{}{}</div><span class=\"dash-mail-preview\">{}</span></a>",
            encoded,
            edgerun_web_ui::escape_html(&message.subject),
            edgerun_web_ui::escape_html(&message.from),
            unread,
            edgerun_web_ui::escape_html(&message.date),
            warning,
            attachments,
            edgerun_web_ui::escape_html(&message.preview),
        ));
    }
    if list.is_empty() {
        list.push_str("<p class=\"dash-mail-empty\">No messages yet.</p>");
    }
    let notice = notice
        .map(|value| {
            format!(
                "<p class=\"dash-mail-warning\">{}</p>",
                edgerun_web_ui::escape_html(value)
            )
        })
        .unwrap_or_default();
    Ok(format!(
        "{}<div class=\"dash-mail-toolbar\"><a class=\"dash-mail-button\" href=\"https://dash.edgerun.tech/#mail\" hx-get=\"/surface/mail/compose\" hx-target=\"#dashMailDetail\" hx-swap=\"outerHTML\">Compose</a><span>{} messages</span></div><div class=\"dash-mail\"><nav class=\"dash-mail-list\" aria-label=\"Messages\">{}</nav><article id=\"dashMailDetail\" class=\"dash-mail-detail\"><p class=\"dash-mail-empty\">Select a message, or compose a new one.</p></article></div>",
        notice,
        messages.len(),
        list
    ))
}

fn render_dash_mail_message(message: &DashMailDetail) -> String {
    let encoded = percent_encode(&message.id);
    let warning = message
        .warning
        .as_ref()
        .map(|value| {
            format!(
                "<p class=\"dash-mail-warning\">{}</p>",
                edgerun_web_ui::escape_html(value)
            )
        })
        .unwrap_or_default();
    let attachments = if message.attachments.is_empty() {
        String::new()
    } else {
        format!(
            "<p class=\"dash-mail-meta\">{} attachments</p>",
            message.attachments.len()
        )
    };
    format!(
        "<article id=\"dashMailDetail\" class=\"dash-mail-detail\"><header><h2>{}</h2><div class=\"dash-mail-meta\"><span>From {}</span><span>To {}</span><span>{}</span></div></header>{}{}<div class=\"dash-mail-actions\"><a class=\"dash-mail-button\" href=\"https://dash.edgerun.tech/#mail\" hx-get=\"/surface/mail/reply/{}\" hx-target=\"#dashMailDetail\" hx-swap=\"outerHTML\">Reply</a></div><pre class=\"dash-mail-body\">{}</pre></article>",
        edgerun_web_ui::escape_html(&message.subject),
        edgerun_web_ui::escape_html(&message.from),
        edgerun_web_ui::escape_html(&message.to),
        edgerun_web_ui::escape_html(&message.date),
        warning,
        attachments,
        encoded,
        edgerun_web_ui::escape_html(&message.body),
    )
}

fn render_dash_mail_compose(
    to: Option<&str>,
    subject: Option<&str>,
    quoted: Option<&str>,
) -> String {
    let body = quoted
        .map(|value| {
            let mut out = String::from("\n\n");
            for line in value.lines().take(120) {
                out.push_str("> ");
                out.push_str(line);
                out.push('\n');
            }
            out
        })
        .unwrap_or_default();
    format!(
        "<article id=\"dashMailDetail\" class=\"dash-mail-detail\"><form class=\"dash-mail-compose\" data-dash-mail-compose action=\"/surface/mail/send\"><label>To<input name=\"to\" autocomplete=\"email\" value=\"{}\"></label><label>Subject<input name=\"subject\" value=\"{}\"></label><label>Message<textarea name=\"body\">{}</textarea></label><div class=\"dash-mail-actions\"><button class=\"dash-mail-button\" type=\"submit\">Send</button><a class=\"dash-mail-button\" href=\"https://dash.edgerun.tech/#mail\" hx-get=\"/surface/mail\" hx-target=\"#surfaceSlot\" hx-swap=\"outerHTML\">Cancel</a></div></form></article>",
        edgerun_web_ui::escape_attr(to.unwrap_or_default()),
        edgerun_web_ui::escape_attr(subject.unwrap_or_default()),
        edgerun_web_ui::escape_html(&body),
    )
}

fn render_dash_mail_error(message: &str) -> String {
    format!(
        "<article id=\"dashMailDetail\" class=\"dash-mail-detail\"><p class=\"dash-mail-warning\">{}</p></article>",
        edgerun_web_ui::escape_html(message)
    )
}

fn reply_recipient(from: &str) -> String {
    sanitize_header(from)
}

fn reply_subject(subject: &str) -> String {
    if subject.to_ascii_lowercase().starts_with("re:") {
        subject.to_string()
    } else {
        format!("Re: {subject}")
    }
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

fn percent_encode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
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

fn render_webmail_html() -> String {
    let header_center =
        edgerun_web_ui::render_header_search_input("search", "Search mail", "Search mail");
    let header_actions = "<span id=\"mailCount\" class=\"mail-count\"></span><span class=\"who\">ken@edgerun.tech</span><er-theme-toggle></er-theme-toggle>";
    let local_links = [FooterLink {
        href: "mailto:ken@edgerun.tech",
        label: "Contact",
    }];
    let footer = edgerun_web_ui::render_common_footer("mail", &local_links, "");
    let style = format!("{}{}", edgerun_web_ui::BASE_STYLE, WEBMAIL_STYLE);
    let body = format!(
        "{}<script>{}{}{} </script>",
        WEBMAIL_BODY,
        edgerun_web_ui::THEME_TOGGLE_JS,
        edgerun_web_ui::WORKSPACE_JS,
        WEBMAIL_SCRIPT
    );
    edgerun_web_ui::render_page(&PageShell {
        lang: "en",
        title: "Edgerun Mail",
        description: "Private Edgerun webmail.",
        theme_color: "#146c63",
        generator: "edgerun-server",
        extra_head: "<link rel=\"icon\" href='data:image/svg+xml,<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 100 100\"><text y=\"76\" font-size=\"76\">📧</text></svg>'>",
        style: &style,
        brand_href: "/",
        brand_label: "Edgerun Mail home",
        brand_text: "Edgerun Mail",
        header_center: &header_center,
        header_actions,
        footer: &footer,
        body: &body,
        script_src: None,
        workspace_modules: &[],
    })
}

const WEBMAIL_STYLE: &str = include_str!("webmail.css");
const WEBMAIL_BODY: &str = include_str!("webmail.html");
const WEBMAIL_SCRIPT: &str = include_str!("webmail.js");

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
