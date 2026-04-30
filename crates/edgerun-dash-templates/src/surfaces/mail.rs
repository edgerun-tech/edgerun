//! Mail surface rendering for the dashboard.
//!
//! These render HTML fragments that get injected into the workspace surface slots.
//! The response wrapper functions return full `Response` objects.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use edgerun_web_ui::escape_attr;
use edgerun_web_ui::escape_html;

/// Wraps content in a mail surface section with the `#surfaceSlot` wrapper
/// that the JS form handlers depend on.
pub fn wrap_mail_surface(content: &str) -> String {
    format!(
        "<section id=\"surfaceSlot\" class=\"dash-stage\" aria-label=\"Mail workspace\">\
            <header><div>\
                <strong id=\"surfaceTitle\">Mail</strong>\
                <span id=\"surfaceUrl\">backend: mail.edgerun.tech</span>\
            </div></header>\
            <div class=\"dash-surface\">{}</div>\
        </section>",
        content,
    )
}

/// Mail login form shown when the user is not authenticated.
pub fn render_mail_login() -> String {
    format!(
        "<form class=\"dash-mail-login\" data-mail-login>\
            <strong>Unlock mail</strong>\
            <label>Username\
                <input name=\"username\" autocomplete=\"username\" value=\"{}\">\
            </label>\
            <label>Password\
                <input name=\"password\" type=\"password\" autocomplete=\"current-password\">\
            </label>\
            <button class=\"dash-mail-button\" type=\"submit\">Unlock mail</button>\
        </form>",
        default_username(),
    )
}

/// Mail inbox with message list and compose button.
pub fn render_mail_inbox(messages: &[MailSummary], notice: Option<&str>) -> String {
    let mut list = String::new();
    for message in messages {
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
            "<a class=\"dash-mail-item\" href=\"https://dash.edgerun.tech/#mail\" \
                hx-get=\"/surface/mail/message/{}\" hx-target=\"#dashMailDetail\" \
                hx-swap=\"outerHTML\">\
                <strong>{}</strong>\
                <span>{}</span>\
                <div class=\"dash-mail-meta\">{}<span>{}</span>{}{}</div>\
                <span class=\"dash-mail-preview\">{}</span>\
            </a>",
            encoded,
            escape_html(&message.subject),
            escape_html(&message.from),
            unread,
            escape_html(&message.date),
            warning,
            attachments,
            escape_html(&message.preview),
        ));
    }
    if list.is_empty() {
        list.push_str("<p class=\"dash-mail-empty\">No messages yet.</p>");
    }
    let notice_html = notice
        .map(|v| format!("<p class=\"dash-mail-warning\">{}</p>", escape_html(v)))
        .unwrap_or_default();

    format!(
        "{}\
        <div class=\"dash-mail-toolbar\">\
            <a class=\"dash-mail-button\" href=\"https://dash.edgerun.tech/#mail\" \
                hx-get=\"/surface/mail/compose\" hx-target=\"#dashMailDetail\" hx-swap=\"outerHTML\">\
                Compose\
            </a>\
            <span>{} messages</span>\
        </div>\
        <div class=\"dash-mail\">\
            <nav class=\"dash-mail-list\" aria-label=\"Messages\">{}</nav>\
            <article id=\"dashMailDetail\" class=\"dash-mail-detail\">\
                <p class=\"dash-mail-empty\">Select a message, or compose a new one.</p>\
            </article>\
        </div>",
        notice_html,
        messages.len(),
        list,
    )
}

/// Single mail message display with reply action.
pub fn render_mail_message(message: &MailDetail) -> String {
    let encoded = percent_encode(&message.id);
    let warning = message
        .warning
        .as_ref()
        .map(|v| format!("<p class=\"dash-mail-warning\">{}</p>", escape_html(v)))
        .unwrap_or_default();
    let attachments = if message.attachments.is_empty() {
        String::new()
    } else {
        format!("<p class=\"dash-mail-meta\">{} attachments</p>", message.attachments.len())
    };
    format!(
        "<article id=\"dashMailDetail\" class=\"dash-mail-detail\">\
            <header>\
                <h2>{}</h2>\
                <div class=\"dash-mail-meta\">\
                    <span>From {}</span>\
                    <span>To {}</span>\
                    <span>{}</span>\
                </div>\
            </header>\
            {}{}\
            <div class=\"dash-mail-actions\">\
                <a class=\"dash-mail-button\" href=\"https://dash.edgerun.tech/#mail\" \
                    hx-get=\"/surface/mail/reply/{}\" hx-target=\"#dashMailDetail\" \
                    hx-swap=\"outerHTML\">Reply</a>\
            </div>\
            <pre class=\"dash-mail-body\">{}</pre>\
        </article>",
        escape_html(&message.subject),
        escape_html(&message.from),
        escape_html(&message.to),
        escape_html(&message.date),
        warning,
        attachments,
        encoded,
        escape_html(&message.body),
    )
}

/// Compose/reply form with optional pre-filled fields.
pub fn render_mail_compose(to: Option<&str>, subject: Option<&str>, quoted: Option<&str>) -> String {
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
        "<article id=\"dashMailDetail\" class=\"dash-mail-detail\">\
            <form class=\"dash-mail-compose\" data-dash-mail-compose action=\"/surface/mail/send\">\
                <label>To\
                    <input name=\"to\" autocomplete=\"email\" value=\"{}\">\
                </label>\
                <label>Subject\
                    <input name=\"subject\" value=\"{}\">\
                </label>\
                <label>Message\
                    <textarea name=\"body\">{}</textarea>\
                </label>\
                <div class=\"dash-mail-actions\">\
                    <button class=\"dash-mail-button\" type=\"submit\">Send</button>\
                    <a class=\"dash-mail-button\" href=\"https://dash.edgerun.tech/#mail\" \
                        hx-get=\"/surface/mail\" hx-target=\"#surfaceSlot\" hx-swap=\"outerHTML\">\
                        Cancel\
                    </a>\
                </div>\
            </form>\
        </article>",
        escape_attr(to.unwrap_or_default()),
        escape_attr(subject.unwrap_or_default()),
        escape_html(&body),
    )
}

/// Error display for mail surface.
pub fn render_mail_error(message: &str) -> String {
    format!(
        "<article id=\"dashMailDetail\" class=\"dash-mail-detail\">\
            <p class=\"dash-mail-warning\">{}</p>\
        </article>",
        escape_html(message),
    )
}

/// Summary of a mail message for list display.
pub struct MailSummary {
    pub id: String,
    pub from: String,
    pub subject: String,
    pub date: String,
    pub unread: bool,
    pub warning: Option<String>,
    pub attachment_count: usize,
    pub preview: String,
}

/// Full detail of a mail message.
pub struct MailDetail {
    pub id: String,
    pub from: String,
    pub to: String,
    pub subject: String,
    pub date: String,
    pub warning: Option<String>,
    pub attachments: Vec<MailAttachment>,
    pub body: String,
}

/// Attachment metadata.
pub struct MailAttachment {
    pub filename: String,
    pub content_type: String,
    pub size: usize,
}

/// Default username for the login form.
fn default_username() -> &'static str {
    "ken"
}

/// Percent-encode a string for use in URLs.
fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 3);
    for byte in input.bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => {
                out.push('%');
                out.push(HEX_DIGITS[(byte >> 4) as usize]);
                out.push(HEX_DIGITS[(byte & 0xf) as usize]);
            }
        }
    }
    out
}

const HEX_DIGITS: [char; 16] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F',
];
