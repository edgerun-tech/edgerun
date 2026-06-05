use crate::codealyzer::crate_model::*;
use std::path::Path;

pub fn generate_issue_link(
    crate_name: &str,
    report_url: &str,
    commit: Option<&str>,
    finding_id: Option<&str>,
    file: Option<&Path>,
    line: Option<usize>,
    severity: Option<&Severity>,
    is_security: bool,
    repo_url: &str,
) -> String {
    if is_security {
        return format!("{}SECURITY.md", repo_url.trim_end_matches('/'));
    }

    let mut title = format!("Issue: {}", crate_name);
    if let Some(id) = finding_id {
        title.push_str(&format!(" - {}", id));
    }

    let mut body = format!("Crate: {}\n", crate_name);
    body.push_str(&format!("Report: {}\n", report_url));
    if let Some(c) = commit {
        body.push_str(&format!("Commit: {}\n", c));
    }
    if let Some(id) = finding_id {
        body.push_str(&format!("Finding: {}\n", id));
    }
    if let Some(f) = file {
        body.push_str(&format!("File: {:?}\n", f));
    }
    if let Some(l) = line {
        body.push_str(&format!("Line: {}\n", l));
    }
    if let Some(s) = severity {
        body.push_str(&format!("Severity: {:?}\n", s));
    }
    body.push_str("\n## Description\n\n");
    body.push_str("<!-- Describe the issue here -->\n");

    let repo = repo_url.trim_end_matches('/');
    format!(
        "{}/issues/new?title={}&body={}",
        repo,
        urlencode(&title),
        urlencode(&body)
    )
}

pub fn generate_local_issue(
    crate_name: &str,
    finding_id: Option<&str>,
    file: Option<&Path>,
    line: Option<usize>,
    severity: Option<&Severity>,
    recommendation: Option<&str>,
) -> String {
    let mut fields = Vec::new();
    fields.push(json_field("crate", &json_string(crate_name)));
    if let Some(id) = finding_id {
        fields.push(json_field("finding_id", &json_string(id)));
    }
    if let Some(f) = file {
        fields.push(json_field("file", &json_string(&f.to_string_lossy())));
    }
    if let Some(l) = line {
        fields.push(json_field("line", &l.to_string()));
    }
    if let Some(s) = severity {
        fields.push(json_field("severity", &json_string(&format!("{:?}", s))));
    }
    if let Some(r) = recommendation {
        fields.push(json_field("recommendation", &json_string(r)));
    }
    fields.push(json_field("status", &json_string("pending")));
    format!("{{{}}}", fields.join(","))
}

fn json_field(name: &str, value: &str) -> String {
    format!("{}:{}", json_string(name), value)
}

fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn urlencode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            ' ' => "%20".into(),
            '\n' => "%0A".into(),
            '&' => "%26".into(),
            '=' => "%3D".into(),
            _ => c.to_string(),
        })
        .collect()
}
