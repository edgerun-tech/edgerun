use std::path::Path;
use crate::codealyzer::crate_model::*;
use edgerun_json::{JsonValue, Map};

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
    format!("{}/issues/new?title={}&body={}", repo, urlencode(&title), urlencode(&body))
}

pub fn generate_local_issue(
    crate_name: &str,
    finding_id: Option<&str>,
    file: Option<&Path>,
    line: Option<usize>,
    severity: Option<&Severity>,
    recommendation: Option<&str>,
) -> JsonValue {
    let mut issue = Map::new();
    issue.insert("crate".into(), JsonValue::String(crate_name.into()));
    if let Some(id) = finding_id {
        issue.insert("finding_id".into(), JsonValue::String(id.into()));
    }
    if let Some(f) = file {
        issue.insert("file".into(), JsonValue::String(f.to_string_lossy().into()));
    }
    if let Some(l) = line {
        issue.insert("line".into(), JsonValue::Number((l as u64).into()));
    }
    if let Some(s) = severity {
        issue.insert("severity".into(), JsonValue::String(format!("{:?}", s)));
    }
    if let Some(r) = recommendation {
        issue.insert("recommendation".into(), JsonValue::String(r.into()));
    }
    issue.insert("status".into(), JsonValue::String("pending".into()));
    JsonValue::Object(issue)
}

fn urlencode(s: &str) -> String {
    s.chars().map(|c| match c {
        ' ' => "%20".into(),
        '\n' => "%0A".into(),
        '&' => "%26".into(),
        '=' => "%3D".into(),
        _ => c.to_string(),
    }).collect()
}
