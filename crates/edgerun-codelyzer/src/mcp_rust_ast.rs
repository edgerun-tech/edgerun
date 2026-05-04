use std::fs;
use std::path::{Path, PathBuf};

use edgerun_edit::edit_ops;
use serde_json::{json, Value};

use crate::filesystem;

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
    Ok(json!({ "items": edit_ops::list_file(&path)? }))
}

fn find_fn(root: &Path, args: Value) -> Result<Value, String> {
    let path = rust_path(root, &args)?;
    let name = str_arg(&args, "name")?;
    let file = edit_ops::parse_file(&path)?;
    let source = edit_ops::find_fn(&file, name);
    Ok(json!({ "name": name, "found": source.is_some(), "source": source }))
}

fn replace_fn_body(root: &Path, args: Value) -> Result<Value, String> {
    let path = rust_path(root, &args)?;
    let name = str_arg(&args, "name")?;
    let body = str_arg(&args, "body")?;
    backup(&path)?;
    let mut file = edit_ops::parse_file(&path)?;
    let changed = edit_ops::replace_fn_body(&mut file, name, body)?;
    if !changed { return Err(format!("function not found: {name}")); }
    edit_ops::write_file(&path, &file)?;
    Ok(json!({ "changed": true, "op": "replace_fn_body", "name": name }))
}

fn add_fn(root: &Path, args: Value) -> Result<Value, String> {
    let path = rust_path(root, &args)?;
    let name = str_arg(&args, "name")?;
    let fn_args = args.get("args").and_then(Value::as_str).unwrap_or("");
    let ret = args.get("ret").and_then(Value::as_str).unwrap_or("");
    let body = str_arg(&args, "body")?;
    backup(&path)?;
    let mut file = edit_ops::parse_file(&path)?;
    edit_ops::add_fn(&mut file, name, fn_args, ret, body)?;
    edit_ops::write_file(&path, &file)?;
    Ok(json!({ "changed": true, "op": "add_fn", "name": name }))
}

fn remove_fn(root: &Path, args: Value) -> Result<Value, String> {
    let path = rust_path(root, &args)?;
    let name = str_arg(&args, "name")?;
    backup(&path)?;
    let mut file = edit_ops::parse_file(&path)?;
    let changed = edit_ops::remove_fn(&mut file, name);
    if !changed { return Err(format!("function not found: {name}")); }
    edit_ops::write_file(&path, &file)?;
    Ok(json!({ "changed": true, "op": "remove_fn", "name": name }))
}

fn add_use(root: &Path, args: Value) -> Result<Value, String> {
    let path = rust_path(root, &args)?;
    let use_path = str_arg(&args, "use_path")?;
    backup(&path)?;
    let mut file = edit_ops::parse_file(&path)?;
    edit_ops::add_use(&mut file, use_path)?;
    edit_ops::write_file(&path, &file)?;
    Ok(json!({ "changed": true, "op": "add_use", "use_path": use_path }))
}

fn add_derive(root: &Path, args: Value) -> Result<Value, String> {
    let path = rust_path(root, &args)?;
    let name = str_arg(&args, "name")?;
    let derive = str_arg(&args, "derive")?;
    backup(&path)?;
    let mut file = edit_ops::parse_file(&path)?;
    let changed = edit_ops::add_derive(&mut file, name, derive)?;
    if !changed { return Err(format!("struct or enum not found: {name}")); }
    edit_ops::write_file(&path, &file)?;
    Ok(json!({ "changed": true, "op": "add_derive", "name": name, "derive": derive }))
}

fn rename_type(root: &Path, args: Value) -> Result<Value, String> {
    let old = str_arg(&args, "old")?;
    let new = str_arg(&args, "new")?;
    let mut changed = Vec::new();
    for rel in rust_files(root) {
        let path = root.join(&rel);
        let before = fs::read_to_string(&path).map_err(|err| format!("read failed: {err}"))?;
        let mut file = match edit_ops::parse_file(&path) { Ok(file) => file, Err(_) => continue };
        edit_ops::rename_type_in_file(&mut file, old, new);
        let temp = std::env::temp_dir().join("edgerun_codelyzer_rename_probe.rs");
        edit_ops::write_file(&temp, &file)?;
        let rendered = fs::read_to_string(&temp).map_err(|err| format!("read temp failed: {err}"))?;
        let _ = fs::remove_file(&temp);
        if rendered != before {
            backup(&path)?;
            fs::write(&path, rendered).map_err(|err| format!("write failed: {err}"))?;
            changed.push(rel.to_string_lossy().to_string());
        }
    }
    Ok(json!({ "changed": !changed.is_empty(), "op": "rename_type", "files": changed }))
}

fn new_file(root: &Path, args: Value) -> Result<Value, String> {
    let rel = str_arg(&args, "path")?;
    if !rel.ends_with(".rs") { return Err("rust_ast new_file requires .rs path".to_string()); }
    let path = safe_join_for_create(root, rel)?;
    let content = args.get("content").and_then(Value::as_str).unwrap_or("");
    edit_ops::new_file(&path, content)?;
    Ok(json!({ "changed": true, "op": "new_file", "path": rel }))
}

fn remove_file(root: &Path, args: Value) -> Result<Value, String> {
    let path = rust_path(root, &args)?;
    backup(&path)?;
    edit_ops::remove_file(&path)?;
    Ok(json!({ "changed": true, "op": "remove_file" }))
}

fn incoming_refs(root: &Path, args: Value) -> Result<Value, String> {
    let path = rust_path(root, &args)?;
    let files: Vec<PathBuf> = rust_files(root).into_iter().map(|rel| root.join(rel)).collect();
    Ok(json!({ "incoming_refs": edit_ops::incoming_refs(&files, &path) }))
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
    if !rel.ends_with(".rs") { return Err("rust_ast tools require .rs files".to_string()); }
    resolve_under_root(root, rel)
}

fn str_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key).and_then(Value::as_str).ok_or_else(|| format!("missing {key}"))
}

fn backup(path: &Path) -> Result<PathBuf, String> {
    let backup_path = path.with_extension(format!("{}.bak", path.extension().and_then(|e| e.to_str()).unwrap_or("file")));
    fs::copy(path, &backup_path).map_err(|err| format!("backup failed: {err}"))?;
    Ok(backup_path)
}

fn resolve_under_root(root: &Path, path: &str) -> Result<PathBuf, String> {
    let joined = root.join(path.trim_start_matches('/'));
    let canonical = joined.canonicalize().map_err(|err| format!("invalid path: {err}"))?;
    if !canonical.starts_with(root) { return Err("path escapes repository root".to_string()); }
    Ok(canonical)
}

fn safe_join_for_create(root: &Path, path: &str) -> Result<PathBuf, String> {
    let joined = root.join(path.trim_start_matches('/'));
    let parent = joined.parent().ok_or("invalid path")?;
    fs::create_dir_all(parent).map_err(|err| format!("mkdir failed: {err}"))?;
    let canonical_parent = parent.canonicalize().map_err(|err| format!("invalid parent: {err}"))?;
    if !canonical_parent.starts_with(root) { return Err("path escapes repository root".to_string()); }
    Ok(joined)
}
