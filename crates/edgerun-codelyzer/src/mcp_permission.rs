use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
struct PermissionRequest {
    kind: String,
    token: String,
    repo_root: String,
    operation: String,
    target_path: String,
    requested_at_ms: u128,
    approval_file: String,
    note: String,
}

#[derive(Debug, Clone)]
pub struct PermissionGrant {
    pub approved: bool,
    pub token: String,
    pub request_path: PathBuf,
}

pub fn require_text_edit_permission(
    repo_root: &Path,
    operation: &str,
    target_path: &str,
    approval_token: Option<&str>,
) -> Result<PermissionGrant, String> {
    if std::env::var("EDGERUN_MCP_ALLOW_TEXT_EDITS").as_deref() == Ok("1") {
        return Ok(PermissionGrant {
            approved: true,
            token: "env-allow".to_string(),
            request_path: request_dir().join("env-allow.json"),
        });
    }

    let token = stable_token(repo_root, operation, target_path);
    let dir = request_dir();
    fs::create_dir_all(&dir).map_err(|err| format!("failed to create permission dir: {err}"))?;
    let request_path = dir.join(format!("{token}.json"));
    let grant_path = dir.join(format!("{token}.approved"));

    if approval_token == Some(token.as_str()) && grant_path.exists() {
        return Ok(PermissionGrant {
            approved: true,
            token,
            request_path,
        });
    }

    let body = PermissionRequest {
        kind: "edgerun.mcp.text_edit_permission_request".to_string(),
        token: token.clone(),
        repo_root: repo_root.display().to_string(),
        operation: operation.to_string(),
        target_path: target_path.to_string(),
        requested_at_ms: now_ms(),
        approval_file: grant_path.display().to_string(),
        note: "Text edits require explicit user approval. Prefer rust_ast for Rust files."
            .to_string(),
    };
    fs::write(&request_path, permission_request_json(&body))
        .map_err(|err| format!("failed to write permission request: {err}"))?;

    Err(format!(
        "text edit requires user approval; request={} token={} approval_file={} then retry with approval_token",
        request_path.display(),
        token,
        grant_path.display()
    ))
}

fn permission_request_json(request: &PermissionRequest) -> String {
    let mut out = String::from("{");
    write_json_field(&mut out, "kind", &request.kind, false);
    write_json_field(&mut out, "token", &request.token, true);
    write_json_field(&mut out, "repo_root", &request.repo_root, true);
    write_json_field(&mut out, "operation", &request.operation, true);
    write_json_field(&mut out, "target_path", &request.target_path, true);
    out.push_str(",\"requested_at_ms\":");
    out.push_str(&request.requested_at_ms.to_string());
    write_json_field(&mut out, "approval_file", &request.approval_file, true);
    write_json_field(&mut out, "note", &request.note, true);
    out.push('}');
    out
}

fn write_json_field(out: &mut String, key: &str, value: &str, comma: bool) {
    if comma {
        out.push(',');
    }
    out.push('"');
    out.push_str(key);
    out.push_str("\":\"");
    push_json_escaped(out, value);
    out.push('"');
}

fn push_json_escaped(out: &mut String, value: &str) {
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
}

pub fn request_dir() -> PathBuf {
    user_cache_dir()
        .join("edgerun-codelyzer")
        .join("permissions")
}

fn user_cache_dir() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}

fn stable_token(repo_root: &Path, operation: &str, target_path: &str) -> String {
    let input = format!("{}\0{}\0{}", repo_root.display(), operation, target_path);
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in input.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("mcp-edit-{hash:016x}")
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
