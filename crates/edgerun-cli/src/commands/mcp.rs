// SPDX-License-Identifier: Apache-2.0
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use edgerun_storage::StorageEngine;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const REGISTRY_FILE: &str = "registry.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StorageRecord {
    pub storage_id: String,
    pub name: String,
    pub tier: String,
    pub created_at_unix_ms: u128,
    pub data_dir: String,
    pub segment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Registry {
    storages: Vec<StorageRecord>,
}

#[derive(Debug, Deserialize)]
struct RpcRequest {
    jsonrpc: Option<String>,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

pub(crate) fn run_mcp_server(_root: &Path, state_dir: Option<PathBuf>) -> Result<()> {
    let base = state_dir.unwrap_or_else(|| PathBuf::from("out/mcp-storage"));
    fs::create_dir_all(&base)?;
    let registry_path = base.join(REGISTRY_FILE);
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();

    for line in stdin.lock().lines() {
        let line = line.context("failed to read MCP input line")?;
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<RpcRequest>(&line) {
            Ok(req) => handle_request(&base, &registry_path, req),
            Err(err) => rpc_error(None, -32700, &format!("parse error: {err}")),
        };
        let encoded = serde_json::to_string(&response)?;
        writeln!(stdout, "{encoded}")?;
        stdout.flush()?;
    }

    Ok(())
}

fn handle_request(base: &Path, registry_path: &Path, req: RpcRequest) -> Value {
    let id = req.id.clone();
    if req.jsonrpc.as_deref() != Some("2.0") {
        return rpc_error(id, -32600, "invalid request: jsonrpc must be '2.0'");
    }
    match req.method.as_str() {
        "create_storage" => {
            let params = req.params.unwrap_or_else(|| json!({}));
            match create_storage(base, registry_path, params) {
                Ok(result) => rpc_ok(id, result),
                Err(err) => rpc_error(id, -32000, &err.to_string()),
            }
        }
        "list_storages" => match load_registry(registry_path) {
            Ok(reg) => rpc_ok(id, json!({ "storages": reg.storages })),
            Err(err) => rpc_error(id, -32000, &err.to_string()),
        },
        "query_storage" => {
            let params = req.params.unwrap_or_else(|| json!({}));
            let storage_id = params
                .get("storage_id")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("");
            if storage_id.is_empty() {
                return rpc_error(id, -32602, "missing required param: storage_id");
            }
            match load_registry(registry_path) {
                Ok(reg) => {
                    let found = reg.storages.iter().find(|s| s.storage_id == storage_id);
                    match found {
                        Some(storage) => rpc_ok(id, json!({ "storage": storage })),
                        None => rpc_error(id, -32004, "storage not found"),
                    }
                }
                Err(err) => rpc_error(id, -32000, &err.to_string()),
            }
        }
        _ => rpc_error(id, -32601, "method not found"),
    }
}

fn create_storage(base: &Path, registry_path: &Path, params: Value) -> Result<Value> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("");
    if name.is_empty() {
        anyhow::bail!("missing required param: name");
    }
    let tier = params
        .get("tier")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("ops");
    let segment = params
        .get("segment")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("events.seg")
        .to_string();

    let mut registry = load_registry(registry_path).unwrap_or_default();
    let storage_id = format!("stg-{}-{}", now_unix_ms(), registry.storages.len() + 1);
    let data_dir = base.join(&storage_id);
    let _engine = StorageEngine::new(data_dir.clone())?;
    let _session = _engine.create_append_session(&segment, 128 * 1024 * 1024)?;
    let record = StorageRecord {
        storage_id: storage_id.clone(),
        name: name.to_string(),
        tier: tier.to_string(),
        created_at_unix_ms: now_unix_ms(),
        data_dir: data_dir.display().to_string(),
        segment,
    };
    registry.storages.push(record.clone());
    save_registry(registry_path, &registry)?;
    Ok(json!({ "storage": record }))
}

fn load_registry(path: &Path) -> Result<Registry> {
    if !path.exists() {
        return Ok(Registry::default());
    }
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read registry {}", path.display()))?;
    let parsed = serde_json::from_str::<Registry>(&raw)
        .with_context(|| format!("invalid registry JSON {}", path.display()))?;
    Ok(parsed)
}

fn save_registry(path: &Path, registry: &Registry) -> Result<()> {
    let parent = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    fs::create_dir_all(parent)?;
    let encoded = serde_json::to_vec_pretty(registry)?;
    fs::write(path, encoded).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn rpc_ok(id: Option<Value>, result: Value) -> Value {
    json!({
      "jsonrpc": "2.0",
      "id": id.unwrap_or(Value::Null),
      "result": result
    })
}

fn rpc_error(id: Option<Value>, code: i32, message: &str) -> Value {
    json!({
      "jsonrpc": "2.0",
      "id": id.unwrap_or(Value::Null),
      "error": {
        "code": code,
        "message": message
      }
    })
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}
