use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

#[derive(Debug, Clone, Archive, RkyvSerialize, RkyvDeserialize)]
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
            request_path: request_dir().join("env-allow.rkyv"),
        });
    }

    let token = stable_token(repo_root, operation, target_path);
    let dir = request_dir();
    fs::create_dir_all(&dir).map_err(|err| format!("failed to create permission dir: {err}"))?;
    let request_path = dir.join(format!("{token}.rkyv"));
    let grant_path = dir.join(format!("{token}.approved"));

    if approval_token == Some(token.as_str()) && grant_path.exists() {
        return Ok(PermissionGrant { approved: true, token, request_path });
    }

    let body = PermissionRequest {
        kind: "edgerun.mcp.text_edit_permission_request".to_string(),
        token: token.clone(),
        repo_root: repo_root.display().to_string(),
        operation: operation.to_string(),
        target_path: target_path.to_string(),
        requested_at_ms: now_ms(),
        approval_file: grant_path.display().to_string(),
        note: "Text edits require explicit user approval. Prefer rust_ast for Rust files.".to_string(),
    };
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&body)
        .map(|bytes| bytes.to_vec())
        .map_err(|err| err.to_string())?;
    fs::write(&request_path, bytes)
        .map_err(|err| format!("failed to write permission request: {err}"))?;

    Err(format!(
        "text edit requires user approval; request={} token={} approval_file={} then retry with approval_token",
        request_path.display(),
        token,
        grant_path.display()
    ))
}

pub fn request_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("edgerun-codelyzer")
        .join("permissions")
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
