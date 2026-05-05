//! Runtime call trace ingestion and merge utilities for codealyzer reports.
//!
//! The trace format is intentionally permissive:
//! - JSON array of objects
//! - or JSONL (one JSON object per line)
//!
//! Accepted object schema:
//! - caller_id / callee_id (legacy full ids like `src/lib.rs::foo`)
//! - or caller_file + caller_name, callee_file + callee_name
//! - count (default: 1)
//! - optional file + line (for fallback reporting)

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::codealyzer::crate_model::{CallGraphEdge, Confidence};
use serde::Deserialize;

#[derive(Debug)]
pub struct RuntimeCallEvent {
    pub caller_id: String,
    pub callee_id: String,
    pub count: u64,
    pub file: Option<String>,
    pub line: Option<usize>,
}

#[derive(Debug, Clone)]
struct RawRuntimeEvent {
    caller_id: Option<String>,
    callee_id: Option<String>,
    caller_file: Option<String>,
    caller_name: Option<String>,
    callee_file: Option<String>,
    callee_name: Option<String>,
    count: Option<u64>,
    file: Option<String>,
    line: Option<usize>,
}

#[derive(Debug)]
struct RuntimeCallAggregate {
    count: u64,
    file: Option<String>,
    line: Option<usize>,
}

impl RuntimeCallAggregate {
    fn add(&mut self, count: u64, file: Option<&String>, line: Option<usize>) {
        self.count = self.count.saturating_add(count);
        if self.file.is_none() {
            self.file = file.cloned();
        }
        if self.line.is_none() {
            self.line = line;
        }
    }
}

impl Default for RuntimeCallAggregate {
    fn default() -> Self {
        Self {
            count: 0,
            file: None,
            line: None,
        }
    }
}

fn normalize_id(
    explicit: Option<String>,
    file: Option<String>,
    name: Option<String>,
) -> Option<String> {
    if let Some(value) = explicit {
        let value = value.trim();
        if !value.is_empty() {
            return Some(value.replace('\\', "/"));
        }
    }

    match (file, name) {
        (Some(file), Some(name)) => {
            let file = file.replace('\\', "/").trim().to_string();
            let name = name.trim().to_string();
            if file.is_empty() || name.is_empty() {
                None
            } else {
                Some(format!("{file}::{name}"))
            }
        }
        (None, Some(name)) => {
            let name = name.trim().to_string();
            if name.is_empty() {
                None
            } else {
                Some(name)
            }
        }
        _ => None,
    }
}

fn split_function_id(id: &str) -> Option<(&str, &str)> {
    id.rfind("::").map(|idx| (&id[..idx], &id[idx + 2..]))
}

fn base_function_name(id: &str) -> &str {
    split_function_id(id).map(|(_, name)| name).unwrap_or(id)
}

fn function_matches_crate_path(function_id: &str, crate_dir: &Path) -> bool {
    let file = match split_function_id(function_id) {
        Some((file, _)) => file,
        None => return false,
    };

    let crate_path = crate_dir.to_string_lossy().replace('\\', "/");
    let file = file.replace('\\', "/");

    if file.starts_with(&crate_path) {
        return true;
    }

    match (crate_dir.file_name().and_then(|name| name.to_str()), file.rfind('/')) {
        (Some(crate_name), Some(pos)) => {
            let crate_suffix = &file[pos + 1..];
            crate_suffix == crate_name
        }
        (Some(crate_name), None) => file == crate_name,
        _ => false,
    }
}

/// Parse a runtime event payload from either JSON array or JSONL (line-based JSON).
pub fn load_runtime_events(path: &Path) -> Result<Vec<RuntimeCallEvent>, String> {
    let content = std::fs::read_to_string(path).map_err(|err| err.to_string())?;
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    let raw_events = match serde_json::from_str::<Vec<RawRuntimeEvent>>(&content) {
        Ok(raw_events) => raw_events,
        Err(_) => content
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }
                serde_json::from_str::<RawRuntimeEvent>(line).ok()
            })
            .collect::<Vec<_>>(),
    };

    let events = raw_events
        .into_iter()
        .filter_map(|raw| {
            let caller_id = normalize_id(raw.caller_id, raw.caller_file, raw.caller_name)?;
            let callee_id = normalize_id(raw.callee_id, raw.callee_file, raw.callee_name)?;
            let count = raw.count.unwrap_or(1).max(1);
            Some(RuntimeCallEvent {
                caller_id,
                callee_id,
                count,
                file: raw.file,
                line: raw.line,
            })
        })
        .collect::<Vec<_>>();

    Ok(events)
}

/// Merge runtime observations into existing call edges and append synthetic runtime edges
/// for observed call paths that static analysis does not resolve.
pub fn merge_runtime_calls_into_edges(
    call_graph: &mut Vec<CallGraphEdge>,
    crate_dir: &Path,
    runtime_events: &[RuntimeCallEvent],
) -> u64 {
    if runtime_events.is_empty() {
        return 0;
    }

    let crate_dir = crate_dir.to_path_buf();
    let crate_file_names: HashSet<String> = call_graph
        .iter()
        .flat_map(|edge| {
            [
                base_function_name(&edge.caller).to_string(),
                base_function_name(&edge.callee).to_string(),
            ]
        })
        .collect();

    let mut exact: HashMap<(String, String), RuntimeCallAggregate> = HashMap::new();
    let mut by_name: HashMap<(String, String), RuntimeCallAggregate> = HashMap::new();

    for event in runtime_events {
        let exact_key = (event.caller_id.clone(), event.callee_id.clone());
        exact
            .entry(exact_key)
            .or_default()
            .add(event.count, event.file.as_ref(), event.line);

        let name_key = (
            base_function_name(&event.caller_id).to_string(),
            base_function_name(&event.callee_id).to_string(),
        );
        by_name
            .entry(name_key)
            .or_default()
            .add(event.count, event.file.as_ref(), event.line);
    }

    let mut matched_runtime_calls = 0u64;

    for edge in call_graph.iter_mut() {
        let exact_key = (edge.caller.clone(), edge.callee.clone());
        if let Some(profile) = exact.remove(&exact_key) {
            edge.runtime_count = edge.runtime_count.saturating_add(profile.count);
            matched_runtime_calls = matched_runtime_calls.saturating_add(profile.count);
            by_name.remove(&(
                base_function_name(&edge.caller).to_string(),
                base_function_name(&edge.callee).to_string(),
            ));
            continue;
        }

        let fallback_key = (
            base_function_name(&edge.caller).to_string(),
            base_function_name(&edge.callee).to_string(),
        );
        if let Some(profile) = by_name.remove(&fallback_key) {
            edge.runtime_count = edge.runtime_count.saturating_add(profile.count);
            matched_runtime_calls = matched_runtime_calls.saturating_add(profile.count);
        }
    }

    let synthetic = exact
        .into_iter()
        .filter_map(|((caller, callee), profile)| {
            if function_matches_crate_path(&caller, &crate_dir)
                || function_matches_crate_path(&callee, &crate_dir)
            {
                return Some(CallGraphEdge {
                    caller,
                    callee,
                    file: profile
                        .file
                        .as_ref()
                        .map(Path::new)
                        .unwrap_or_else(|| Path::new("<runtime>"))
                        .to_path_buf(),
                    line: profile.line.unwrap_or(0),
                    confidence: Confidence::Ambiguous,
                    runtime_count: profile.count,
                });
            }

            if crate_file_names.contains(&base_function_name(&caller))
                || crate_file_names.contains(&base_function_name(&callee))
            {
                return Some(CallGraphEdge {
                    caller,
                    callee,
                    file: Path::new("<runtime>").to_path_buf(),
                    line: profile.line.unwrap_or(0),
                    confidence: Confidence::Ambiguous,
                    runtime_count: profile.count,
                });
            }

            None
        })
        .collect::<Vec<_>>();

    matched_runtime_calls = synthetic
        .iter()
        .fold(matched_runtime_calls, |acc, edge| {
            acc.saturating_add(edge.runtime_count)
        });

    call_graph.extend(synthetic);

    // Handle name-only events not attached by exact file path.
    for ((caller, callee), profile) in by_name {
        if caller.is_empty() || callee.is_empty() {
            continue;
        }
        if !crate_file_names.contains(&caller) && !crate_file_names.contains(&callee) {
            continue;
        }
        matched_runtime_calls = matched_runtime_calls.saturating_add(profile.count);
        call_graph.push(CallGraphEdge {
            caller,
            callee,
            file: Path::new(profile.file.as_deref().unwrap_or("<runtime>")).to_path_buf(),
            line: profile.line.unwrap_or(0),
            confidence: Confidence::Ambiguous,
            runtime_count: profile.count,
        });
    }

    matched_runtime_calls
}
