use std::collections::{HashMap, HashSet};

use crate::{
    edit::{ChangeSet, FunctionId},
    generated::codeanalyzer::{GraphData, GraphEdge, GraphNode},
    parser::{self, ParserPool},
    uir::{CallEdge, CallKind, Function, Program},
    xray_wire::SourceSnapshot,
};

/// Browser/WASM-friendly source entry. The caller owns discovery and file
/// access; codelyzer owns parsing and graph construction.
pub struct SourceFile<'a> {
    pub path: &'a str,
    pub source: &'a str,
}

pub struct SourceAnalysisResult {
    pub program: Program,
    pub changes: ChangeSet,
}

pub fn analyze_sources(files: &[SourceFile<'_>]) -> SourceAnalysisResult {
    let mut pool = ParserPool::new();
    let parse_results: Vec<(&str, Option<parser::CachedParseResult>)> = files
        .iter()
        .map(|file| {
            let parsed = pool
                .parse_file(file.path, file.source)
                .map(|result| result.to_owned());
            (file.path, parsed)
        })
        .collect();

    let mut all_functions: Vec<(String, String, bool)> = Vec::new();
    let mut all_calls: Vec<(String, String, String, CallKind)> = Vec::new();

    for (path, parsed) in parse_results {
        let Some(result) = parsed else {
            continue;
        };
        for func in &result.functions {
            all_functions.push((path.to_string(), func.name.clone(), func.is_static));
        }
        for (caller_name, raw_call) in
            parser::build_call_pairs_owned(&result.functions, &result.calls)
        {
            all_calls.push((
                path.to_string(),
                caller_name,
                raw_call.callee_name,
                raw_call.kind,
            ));
        }
    }

    let mut program = Program::default();
    let mut changes = ChangeSet::default();
    let mut seen_ids: HashSet<String> = HashSet::new();
    let mut file_name_to_id: HashMap<String, HashMap<String, FunctionId>> = HashMap::new();
    let mut name_to_ids: HashMap<String, Vec<FunctionId>> = HashMap::new();

    for (file, func_name, is_static) in &all_functions {
        let id = format!("{file}::{func_name}");
        if !seen_ids.insert(id.clone()) {
            continue;
        }
        let func = Function {
            id: id.clone(),
            name: func_name.clone(),
            language: lang_for_file(file).to_string(),
            file: file.clone(),
            is_static: *is_static,
            last_modified_commit: None,
        };
        let fid = func.function_id();
        program.add_function(func);
        changes.added.push((fid.clone(), id));
        name_to_ids
            .entry(func_name.clone())
            .or_default()
            .push(fid.clone());
        file_name_to_id
            .entry(file.clone())
            .or_default()
            .insert(func_name.clone(), fid);
    }

    for (file, caller_name, callee_name, kind) in &all_calls {
        let Some(caller_fid) = file_name_to_id
            .get(file)
            .and_then(|names| names.get(caller_name))
        else {
            continue;
        };
        let caller_id = caller_fid.to_legacy();
        let (callee_id, call_kind) = match kind {
            CallKind::Direct => {
                match resolve_callee(callee_name, file, &file_name_to_id, &name_to_ids) {
                    Some(fid) => (fid.to_legacy(), CallKind::Direct),
                    None => (callee_name.clone(), CallKind::Unknown),
                }
            }
            CallKind::Indirect => (format!("INDIRECT_{callee_name}"), CallKind::Indirect),
            CallKind::Macro => (format!("MACRO_{callee_name}"), CallKind::Macro),
            CallKind::Unknown => (format!("UNKNOWN_{callee_name}"), CallKind::Unknown),
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

    SourceAnalysisResult { program, changes }
}

pub fn analyze_snapshot(snapshot: &SourceSnapshot) -> SourceAnalysisResult {
    let files: Vec<SourceFile<'_>> = snapshot
        .files
        .iter()
        .map(|file| SourceFile {
            path: &file.path,
            source: &file.source,
        })
        .collect();
    analyze_sources(&files)
}

pub fn graph_data_from_snapshot(snapshot: &SourceSnapshot) -> GraphData {
    let result = analyze_snapshot(snapshot);
    graph_data_from_program(&result.program, snapshot.total_bytes)
}

pub fn graph_data_from_program(program: &Program, total_bytes: u64) -> GraphData {
    let connectivity = program.compute_connectivity();

    let mut nodes: Vec<GraphNode> = program
        .functions_ordered()
        .into_iter()
        .map(|func| GraphNode {
            id: func.id.clone(),
            name: func.name.clone(),
            file: func.file.clone(),
            language: func.language.clone(),
            is_static: func.is_static,
            connections: connectivity.get(&func.id).copied().unwrap_or(0) as u32,
            tags: tags_for_function(func),
            commit: func.last_modified_commit.clone(),
        })
        .collect();

    let mut edges: Vec<GraphEdge> = program
        .edges
        .iter()
        .map(|edge| GraphEdge {
            source: edge.caller.clone(),
            target: edge.callee.clone(),
            kind: edge.kind.label().to_string(),
        })
        .collect();

    nodes.sort_by(|a, b| a.id.cmp(&b.id));
    edges.sort_by(|a, b| a.source.cmp(&b.source).then(a.target.cmp(&b.target)));

    GraphData {
        node_count: nodes.len() as u32,
        edge_count: edges.len() as u32,
        nodes,
        edges,
        tag_groups: Vec::new(),
        total_bytes,
    }
}

fn tags_for_function(func: &Function) -> Vec<String> {
    let mut tags = vec![func.language.clone()];
    if func.is_static {
        tags.push("static".to_string());
    }
    if let Some(commit) = &func.last_modified_commit {
        tags.push(format!("commit:{commit}"));
    }
    if func.file.contains("components") || func.file.contains("/ui/") {
        tags.push("ui".to_string());
    }
    if func.file.contains("runtime") {
        tags.push("runtime".to_string());
    }
    if func.file.contains("storage") || func.file.contains("store") {
        tags.push("storage".to_string());
    }
    tags
}

fn resolve_callee(
    callee_name: &str,
    caller_file: &str,
    file_name_to_id: &HashMap<String, HashMap<String, FunctionId>>,
    name_to_ids: &HashMap<String, Vec<FunctionId>>,
) -> Option<FunctionId> {
    file_name_to_id
        .get(caller_file)
        .and_then(|names| names.get(callee_name))
        .cloned()
        .or_else(|| {
            name_to_ids
                .get(callee_name)
                .and_then(|ids| ids.first())
                .cloned()
        })
}

fn lang_for_file(file: &str) -> &'static str {
    if file.ends_with(".rs") {
        "rust"
    } else if file.ends_with(".ts") || file.ends_with(".tsx") {
        "typescript"
    } else if file.ends_with(".js") || file.ends_with(".jsx") || file.ends_with(".mjs") {
        "javascript"
    } else if file.ends_with(".py") {
        "python"
    } else if file.ends_with(".go") {
        "go"
    } else if file.ends_with(".java") {
        "java"
    } else if file.ends_with(".c") || file.ends_with(".h") {
        "c"
    } else {
        "unknown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyzes_in_memory_sources() {
        let files = vec![SourceFile {
            path: "src/lib.rs",
            source: "pub fn a() { b(); }\nfn b() {}",
        }];
        #[cfg(feature = "lang-typescript")]
        let files = {
            let mut files = files;
            files.push(SourceFile {
                path: "src/net.ts",
                source: "export function send() { socket.write(); }",
            });
            files
        };

        let result = analyze_sources(&files);
        #[cfg(feature = "lang-typescript")]
        assert_eq!(result.program.functions.len(), 3);
        #[cfg(not(feature = "lang-typescript"))]
        assert_eq!(result.program.functions.len(), 2);
        assert!(
            result
                .program
                .edges
                .iter()
                .any(|edge| edge.caller == "src/lib.rs::a" && edge.callee == "src/lib.rs::b")
        );
        #[cfg(feature = "lang-typescript")]
        assert!(
            result
                .changes
                .edges_added
                .iter()
                .any(|(_, callee, _)| callee == "write")
        );
    }

    #[test]
    fn emits_graph_data_from_rkyv_snapshot() {
        let snapshot = SourceSnapshot {
            files: vec![crate::xray_wire::SourceBlob {
                path: "src/lib.rs".into(),
                source: "pub fn a() { b(); }\nfn b() {}".into(),
            }],
            total_bytes: 32,
        };
        let encoded = snapshot.encode_rkyv().expect("encode snapshot");
        let decoded = SourceSnapshot::decode_rkyv(&encoded).expect("decode snapshot");
        let graph = graph_data_from_snapshot(&decoded);
        assert_eq!(graph.node_count, 2);
        assert!(
            graph
                .edges
                .iter()
                .any(|edge| edge.source == "src/lib.rs::a" && edge.target == "src/lib.rs::b")
        );
    }
}
