//! Runtime call trace ingestion and merge utilities for codealyzer reports.
//!
//! Runtime events are loaded from a single rkyv-archived `Vec<RuntimeCallEvent>` payload.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::codealyzer::crate_model::{CallGraphEdge, Confidence};
#[cfg(feature = "wire-rkyv")]
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "wire-rkyv", derive(Archive, RkyvSerialize, RkyvDeserialize))]
pub struct RuntimeCallEvent {
    pub caller_id: String,
    pub callee_id: String,
    pub count: u64,
    pub file: Option<String>,
    pub line: Option<usize>,
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

    match (
        crate_dir.file_name().and_then(|name| name.to_str()),
        file.rfind('/'),
    ) {
        (Some(crate_name), Some(pos)) => {
            let crate_suffix = &file[pos + 1..];
            crate_suffix == crate_name
        }
        (Some(crate_name), None) => file == crate_name,
        _ => false,
    }
}

/// Load runtime event observations from a rkyv payload.
#[cfg(feature = "wire-rkyv")]
pub fn load_runtime_events(path: &Path) -> Result<Vec<RuntimeCallEvent>, String> {
    let bytes = std::fs::read(path).map_err(|err| err.to_string())?;
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    let archived =
        rkyv::access::<rkyv::Archived<Vec<RuntimeCallEvent>>, rkyv::rancor::Error>(&bytes)
            .map_err(|err| err.to_string())?;
    rkyv::deserialize::<Vec<RuntimeCallEvent>, rkyv::rancor::Error>(archived)
        .map_err(|err| err.to_string())
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

            if crate_file_names.contains(base_function_name(&caller))
                || crate_file_names.contains(base_function_name(&callee))
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

    matched_runtime_calls = synthetic.iter().fold(matched_runtime_calls, |acc, edge| {
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
