use std::path::Path;
use crate::codealyzer::crate_model::*;

pub fn analyze_source_files(files: &[std::path::PathBuf]) -> (Vec<ApiItem>, Vec<CallGraphEdge>) {
    let mut api_items = Vec::new();
    let mut call_edges = Vec::new();

    for file in files {
        if let Some(ext) = file.extension() {
            if ext == "rs" {
                if let Ok(content) = std::fs::read_to_string(file) {
                    let (items, edges) = analyze_file(file, &content);
                    api_items.extend(items);
                    call_edges.extend(edges);
                }
            }
        }
    }

    (api_items, call_edges)
}

fn analyze_file(file: &Path, content: &str) -> (Vec<ApiItem>, Vec<CallGraphEdge>) {
    let mut api_items = Vec::new();
    let mut call_edges = Vec::new();
    let mut current_fn: Option<String> = None;
    let mut line_num = 0;

    for line in content.lines() {
        line_num += 1;
        let trimmed = line.trim();

        if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("//!") {
            continue;
        }

        if trimmed.contains("pub fn ") || trimmed.contains("pub async fn ") {
            if let Some(name) = extract_fn_name(trimmed) {
                api_items.push(ApiItem {
                    name: name.clone(),
                    kind: ApiItemKind::Function,
                    file: file.to_path_buf(),
                    line: line_num,
                    docs: has_doc_comment(content, line_num),
                    feature_gate: None,
                    visibility: Visibility::Public,
                });
                current_fn = Some(name);
            }
        } else if trimmed.contains("pub struct ") {
            if let Some(name) = extract_struct_name(trimmed) {
                api_items.push(ApiItem {
                    name, kind: ApiItemKind::Struct,
                    file: file.to_path_buf(), line: line_num,
                    docs: has_doc_comment(content, line_num),
                    feature_gate: None, visibility: Visibility::Public,
                });
            }
        } else if trimmed.contains("pub enum ") {
            if let Some(name) = extract_enum_name(trimmed) {
                api_items.push(ApiItem {
                    name, kind: ApiItemKind::Enum,
                    file: file.to_path_buf(), line: line_num,
                    docs: has_doc_comment(content, line_num),
                    feature_gate: None, visibility: Visibility::Public,
                });
            }
        } else if trimmed.contains("pub trait ") {
            if let Some(name) = extract_trait_name(trimmed) {
                api_items.push(ApiItem {
                    name, kind: ApiItemKind::Trait,
                    file: file.to_path_buf(), line: line_num,
                    docs: has_doc_comment(content, line_num),
                    feature_gate: None, visibility: Visibility::Public,
                });
            }
        } else if trimmed.contains("pub mod ") {
            if let Some(name) = extract_mod_name(trimmed) {
                api_items.push(ApiItem {
                    name, kind: ApiItemKind::Module,
                    file: file.to_path_buf(), line: line_num,
                    docs: has_doc_comment(content, line_num),
                    feature_gate: None, visibility: Visibility::Public,
                });
            }
        } else if trimmed.contains("impl ") {
            api_items.push(ApiItem {
                name: format!("impl at line {}", line_num),
                kind: ApiItemKind::Impl,
                file: file.to_path_buf(), line: line_num,
                docs: false, feature_gate: None,
                visibility: Visibility::Public,
            });
        }

        if let Some(ref caller) = current_fn {
            for callee in extract_fn_calls(trimmed) {
                call_edges.push(CallGraphEdge {
                    caller: caller.clone(),
                    callee,
                    file: file.to_path_buf(),
                    line: line_num,
                    confidence: Confidence::Likely,
                    runtime_count: 0,
                });
            }
        }
    }

    (api_items, call_edges)
}

fn extract_fn_name(line: &str) -> Option<String> {
    let start = line.find("fn ")? + 3;
    let end = line[start..].find('(').unwrap_or(line.len() - start);
    Some(line[start..start+end].trim().to_string())
}

fn extract_struct_name(line: &str) -> Option<String> {
    let start = line.find("struct ")? + 7;
    let rest = &line[start..];
    let end = rest.find(|c: char| c == '{' || c == ';' || c.is_whitespace()).unwrap_or(rest.len());
    Some(rest[..end].trim().to_string())
}

fn extract_enum_name(line: &str) -> Option<String> {
    let start = line.find("enum ")? + 5;
    let rest = &line[start..];
    let end = rest.find(|c: char| c == '{' || c == ';' || c.is_whitespace()).unwrap_or(rest.len());
    Some(rest[..end].trim().to_string())
}

fn extract_trait_name(line: &str) -> Option<String> {
    let start = line.find("trait ")? + 6;
    let rest = &line[start..];
    let end = rest.find(|c: char| c == '{' || c == ';' || c.is_whitespace()).unwrap_or(rest.len());
    Some(rest[..end].trim().to_string())
}

fn extract_mod_name(line: &str) -> Option<String> {
    let start = line.find("mod ")? + 4;
    let rest = &line[start..];
    let end = rest.find(|c: char| c == '{' || c == ';' || c.is_whitespace()).unwrap_or(rest.len());
    Some(rest[..end].trim().to_string())
}

fn extract_fn_calls(line: &str) -> Vec<String> {
    let mut calls = Vec::new();
    let mut rest = line;
    while let Some(pos) = rest.find('(') {
        let before = &rest[..pos];
        if let Some(start) = before.rfind(|c: char| !c.is_alphanumeric() && c != '_' && c != ':') {
            let name = &before[start+1..];
            if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == ':' || c == '_') {
                calls.push(name.trim().to_string());
            }
        }
        rest = &rest[pos+1..];
    }
    calls
}

fn has_doc_comment(content: &str, line_num: usize) -> bool {
    let lines: Vec<&str> = content.lines().collect();
    if line_num == 0 { return false; }
    for i in (0..line_num.saturating_sub(3)).rev() {
        let line = lines.get(i).unwrap_or(&"").trim();
        if line.starts_with("///") || line.starts_with("//!") {
            return true;
        }
        if !line.is_empty() && !line.starts_with("//") {
            break;
        }
    }
    false
}
