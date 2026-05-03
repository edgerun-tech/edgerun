use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

use rayon::prelude::*;

/// Analyzer module: converts parser output into the UIR graph.
///
/// Supports two modes:
/// 1. Full analysis: scan everything, build complete graph
/// 2. Incremental update: diff vs previous snapshot, re-parse only changed
///    files
use crate::edit::{ChangeSet, FunctionId};
use crate::{
    filesystem::{self, FileInfo, ParseCache},
    git,
    parser::{self, ParserPool},
    uir::{CallEdge, CallKind, Function, Program},
};

// ─── Source reader ──────────────────────────────────────────────────────

/// Read a file for parsing.
fn read_source(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok()
}

/// Result of an analysis run, including filesystem state for incremental
/// updates.
#[allow(dead_code)]
pub struct AnalysisResult {
    pub program: Program,
    pub file_snapshot: Vec<FileInfo>, // current filesystem state
}

/// Full analysis: scan all files, build complete graph, include git info.
/// Returns the result plus a ChangeSet describing everything that was added.
pub fn analyze_full(root_dir: &str) -> (AnalysisResult, ChangeSet) {
    let file_snapshot = filesystem::scan_dir(root_dir);
    let commit_map = fetch_commit_map(root_dir);
    let (program, changes) = build_program(&file_snapshot, root_dir, &commit_map);
    (AnalysisResult { program, file_snapshot }, changes)
}

/// Build the full UIR program from a file snapshot.
/// Returns the program plus a ChangeSet describing everything that was added.
fn build_program(
    snapshot: &[FileInfo],
    root_dir: &str,
    commit_map: &HashMap<String, String>,
) -> (Program, ChangeSet) {
    let mut program = Program::default();
    let mut changes = ChangeSet::default();

    // Phase 1: Read all files in parallel (I/O bound, Send-safe)
    let file_contents: Vec<(String, String)> = snapshot
        .par_iter()
        .filter_map(|file_info| {
            let full_path = Path::new(root_dir.trim_end_matches('/')).join(&file_info.path);
            let source = read_source(&full_path)?;
            Some((file_info.path.clone(), source.clone()))
        })
        .collect();

    // Phase 2: Parse files in parallel (each thread gets its own parser)
    let parse_results: Vec<(String, Option<parser::CachedParseResult>)> = file_contents
        .into_par_iter()
        .map(|(rel_path, source)| {
            let mut pool = ParserPool::new();
            let result = pool.parse_file(&rel_path, &source);
            let cached = result.map(|r| r.to_owned());
            (rel_path, cached)
        })
        .collect();

    // Phase 3: Collect functions and calls, update cache
    let mut cache = ParseCache::load(root_dir);

    // Pre-allocate with reasonable capacity to reduce reallocations
    let mut all_functions_final: Vec<(String, String, bool)> = Vec::with_capacity(256);
    let mut all_calls_final: Vec<(String, String, String, CallKind)> = Vec::with_capacity(512);

    for (rel_path, cached) in parse_results {
        if let Some(result) = cached {
            // Find the matching FileInfo for hash/mtime
            if let Some(fi) = snapshot.iter().find(|f| f.path == rel_path) {
                cache.insert(rel_path.clone(), fi.hash, fi.modified_ts, result.clone());
            }
            for func in &result.functions {
                all_functions_final.push((rel_path.clone(), func.name.clone(), func.is_static));
            }
            let pairs = parser::build_call_pairs_owned(&result.functions, &result.calls);
            for (caller_name, raw_call) in pairs {
                all_calls_final.push((
                    rel_path.clone(),
                    caller_name,
                    raw_call.callee_name,
                    raw_call.kind,
                ));
            }
        }
    }

    // Save cache to disk
    cache.save();

    let mut seen_ids: HashSet<String> = HashSet::new();
    let mut file_name_to_id: HashMap<String, HashMap<String, FunctionId>> = HashMap::new();
    let mut name_to_ids: HashMap<String, Vec<FunctionId>> = HashMap::new();

    for (file, func_name, is_static) in &all_functions_final {
        let id = format!("{}::{}", file, func_name);
        if seen_ids.insert(id.clone()) {
            let commit = commit_map.get(file).map(|s| short_hash(s));
            let func = Function {
                id: id.clone(),
                name: func_name.clone(),
                language: lang_for_file(file).to_string(),
                file: file.clone(),
                is_static: *is_static,
                last_modified_commit: commit,
            };
            let fid = func.function_id();
            program.add_function(func.clone());
            changes.added.push((fid.clone(), id.clone()));
            name_to_ids.entry(func_name.clone()).or_default().push(fid.clone());
            file_name_to_id.entry(file.clone()).or_default().insert(func_name.clone(), fid.clone());
        }
    }

    for (file, caller_name, callee_name, kind) in &all_calls_final {
        let caller_fid = match file_name_to_id.get(file).and_then(|m| m.get(caller_name)) {
            Some(fid) => fid.clone(),
            None => continue,
        };
        let caller_id = caller_fid.to_legacy();

        let resolved = resolve_callee(callee_name, file, &file_name_to_id, &name_to_ids);
        let (callee_id, call_kind) = match kind {
            CallKind::Direct => match resolved {
                Some(fid) => (fid.to_legacy(), CallKind::Direct),
                None => (callee_name.clone(), CallKind::Unknown),
            },
            CallKind::Indirect => (format!("INDIRECT_{}", callee_name), CallKind::Indirect),
            CallKind::Macro => (format!("MACRO_{}", callee_name), CallKind::Macro),
            CallKind::Unknown => (format!("UNKNOWN_{}", callee_name), CallKind::Unknown),
        };

        program.add_edge(CallEdge {
            caller: caller_id.clone(),
            callee: callee_id.clone(),
            kind: call_kind,
            confidence: call_kind.confidence(),
            source: "static".to_string(),
        });
        changes.edges_added.push((caller_id, callee_id, call_kind));
    }

    (program, changes)
}

/// Fetch git commits and build file→commit map. Returns empty map if not a git
/// repo.
fn fetch_commit_map(root_dir: &str) -> HashMap<String, String> {
    let commits = git::fetch_commits(root_dir, Some(100));
    if commits.is_empty() {
        return HashMap::new();
    }
    git::build_file_commit_map(&commits)
}

/// Take first 8 chars of a commit hash for display.
fn short_hash(full: &str) -> String {
    if full.len() >= 8 {
        full[..8].to_string()
    } else {
        full.to_string()
    }
}

/// Try to resolve a callee name to a FunctionId.
fn resolve_callee(
    callee_name: &str,
    caller_file: &str,
    file_name_to_id: &HashMap<String, HashMap<String, FunctionId>>,
    name_to_ids: &HashMap<String, Vec<FunctionId>>,
) -> Option<FunctionId> {
    // Same-file first
    if let Some(file_map) = file_name_to_id.get(caller_file) {
        if let Some(fid) = file_map.get(callee_name) {
            return Some(fid.clone());
        }
    }
    // Any file
    if let Some(ids) = name_to_ids.get(callee_name) {
        if let Some(first) = ids.first() {
            return Some(first.clone());
        }
    }
    None
}

fn lang_for_file(path: &str) -> &'static str {
    if path.ends_with(".rs") {
        "rust"
    } else if path.ends_with(".ts") || path.ends_with(".tsx") {
        "typescript"
    } else if path.ends_with(".js") || path.ends_with(".jsx") || path.ends_with(".mjs") {
        "javascript"
    } else if path.ends_with(".c") || path.ends_with(".h") {
        "c"
    } else if path.ends_with(".py") {
        "python"
    } else if path.ends_with(".go") {
        "go"
    } else if path.ends_with(".java") {
        "java"
    } else {
        "unknown"
    }
}
