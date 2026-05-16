//! MCP JSON-RPC helper payloads only; not an internal codelyzer wire format.

use std::fs;
use std::path::{Path, PathBuf};

use edgerun_json::{Value, json};

use crate::{filesystem, rust_edit};

pub fn call(root: &Path, args: Value) -> Result<Value, String> {
    let op = args.get("op").and_then(Value::as_str).ok_or("missing op")?;
    match op {
        "list_file" => list_file(root, args),
        "find_fn" => find_fn(root, args),
        "replace_fn_body" => replace_fn_body(root, args),
        "add_fn" => add_fn(root, args),
        "remove_fn" => remove_fn(root, args),
        "add_use" => add_use(root, args),
        "add_derive" => add_derive(root, args),
        "rename_type" => rename_type(root, args),
        "new_file" => new_file(root, args),
        "remove_file" => remove_file(root, args),
        "incoming_refs" => incoming_refs(root, args),
        _ => Err(format!("unknown rust_ast op: {op}")),
    }
}

fn list_file(root: &Path, args: Value) -> Result<Value, String> {
    let path = rust_path(root, &args)?;
    Ok(json!({ "items": rust_edit::list_file(&path)? }))
}

fn find_fn(root: &Path, args: Value) -> Result<Value, String> {
    let path = rust_path(root, &args)?;
    let name = str_arg(&args, "name")?;
    let source = rust_edit::find_fn(&path, name)?;
    Ok(json!({ "name": name, "found": source.is_some(), "source": source }))
}

fn replace_fn_body(root: &Path, args: Value) -> Result<Value, String> {
    let path = rust_path(root, &args)?;
    let name = str_arg(&args, "name")?;
    let body = str_arg(&args, "body")?;
    backup(&path)?;
    let changed = rust_edit::replace_fn_body(&path, name, body)?;
    if !changed {
        return Err(format!("function not found: {name}"));
    }
    Ok(json!({ "changed": true, "op": "replace_fn_body", "name": name }))
}

fn add_fn(root: &Path, args: Value) -> Result<Value, String> {
    let path = rust_path(root, &args)?;
    let name = str_arg(&args, "name")?;
    let fn_args = args.get("args").and_then(Value::as_str).unwrap_or("");
    let ret = args.get("ret").and_then(Value::as_str).unwrap_or("");
    let body = str_arg(&args, "body")?;
    backup(&path)?;
    rust_edit::add_fn(&path, name, fn_args, ret, body)?;
    Ok(json!({ "changed": true, "op": "add_fn", "name": name }))
}

fn remove_fn(root: &Path, args: Value) -> Result<Value, String> {
    let path = rust_path(root, &args)?;
    let name = str_arg(&args, "name")?;
    backup(&path)?;
    let changed = rust_edit::remove_fn(&path, name)?;
    if !changed {
        return Err(format!("function not found: {name}"));
    }
    Ok(json!({ "changed": true, "op": "remove_fn", "name": name }))
}

fn add_use(root: &Path, args: Value) -> Result<Value, String> {
    let path = rust_path(root, &args)?;
    let use_path = str_arg(&args, "use_path")?;
    backup(&path)?;
    rust_edit::add_use(&path, use_path)?;
    Ok(json!({ "changed": true, "op": "add_use", "use_path": use_path }))
}

fn add_derive(root: &Path, args: Value) -> Result<Value, String> {
    let path = rust_path(root, &args)?;
    let name = str_arg(&args, "name")?;
    let derive = str_arg(&args, "derive")?;
    backup(&path)?;
    let changed = rust_edit::add_derive(&path, name, derive)?;
    if !changed {
        return Err(format!("struct or enum not found: {name}"));
    }
    Ok(json!({ "changed": true, "op": "add_derive", "name": name, "derive": derive }))
}

fn rename_type(root: &Path, args: Value) -> Result<Value, String> {
    let old = str_arg(&args, "old")?;
    let new = str_arg(&args, "new")?;
    let mut changed = Vec::new();
    for rel in rust_files(root) {
        let path = root.join(&rel);
        if !rust_edit::file_contains_identifier(&path, old)? {
            continue;
        }
        backup(&path)?;
        if rust_edit::rename_identifier_in_file(&path, old, new)? {
            changed.push(rel.to_string_lossy().to_string());
        }
    }
    Ok(json!({ "changed": !changed.is_empty(), "op": "rename_type", "files": changed }))
}

fn new_file(root: &Path, args: Value) -> Result<Value, String> {
    let rel = str_arg(&args, "path")?;
    if !rel.ends_with(".rs") {
        return Err("rust_ast new_file requires .rs path".to_string());
    }
    let path = safe_join_for_create(root, rel)?;
    let content = args.get("content").and_then(Value::as_str).unwrap_or("");
    rust_edit::new_file(&path, content)?;
    Ok(json!({ "changed": true, "op": "new_file", "path": rel }))
}

fn remove_file(root: &Path, args: Value) -> Result<Value, String> {
    let path = rust_path(root, &args)?;
    backup(&path)?;
    rust_edit::remove_file(&path)?;
    Ok(json!({ "changed": true, "op": "remove_file" }))
}

fn incoming_refs(root: &Path, args: Value) -> Result<Value, String> {
    let path = rust_path(root, &args)?;
    let files: Vec<PathBuf> = rust_files(root)
        .into_iter()
        .map(|rel| root.join(rel))
        .collect();
    Ok(json!({ "incoming_refs": rust_edit::incoming_refs(&files, &path) }))
}

fn rust_files(root: &Path) -> Vec<PathBuf> {
    filesystem::scan_dir(root.to_str().unwrap_or("."))
        .into_iter()
        .filter(|file| file.language == "rust" || file.path.ends_with(".rs"))
        .map(|file| PathBuf::from(file.path))
        .collect()
}

fn rust_path(root: &Path, args: &Value) -> Result<PathBuf, String> {
    let rel = str_arg(args, "path")?;
    if !rel.ends_with(".rs") {
        return Err("rust_ast tools require .rs files".to_string());
    }
    resolve_under_root(root, rel)
}

fn str_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing {key}"))
}

fn backup(path: &Path) -> Result<PathBuf, String> {
    let backup_path = path.with_extension(format!(
        "{}.bak",
        path.extension().and_then(|e| e.to_str()).unwrap_or("file")
    ));
    fs::copy(path, &backup_path).map_err(|err| format!("backup failed: {err}"))?;
    Ok(backup_path)
}

fn resolve_under_root(root: &Path, path: &str) -> Result<PathBuf, String> {
    let joined = root.join(path.trim_start_matches('/'));
    let canonical = joined
        .canonicalize()
        .map_err(|err| format!("invalid path: {err}"))?;
    if !canonical.starts_with(root) {
        return Err("path escapes repository root".to_string());
    }
    Ok(canonical)
}

fn safe_join_for_create(root: &Path, path: &str) -> Result<PathBuf, String> {
    let joined = root.join(path.trim_start_matches('/'));
    let parent = joined.parent().ok_or("invalid path")?;
    fs::create_dir_all(parent).map_err(|err| format!("mkdir failed: {err}"))?;
    let canonical_parent = parent
        .canonicalize()
        .map_err(|err| format!("invalid parent: {err}"))?;
    if !canonical_parent.starts_with(root) {
        return Err("path escapes repository root".to_string());
    }
    Ok(joined)
}
