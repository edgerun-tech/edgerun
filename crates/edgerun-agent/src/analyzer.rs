//! Code analyzer module: tree-sitter based Rust parsing and call graph construction.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use tree_sitter::{Language, Parser, Tree};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageType {
    Rust,
    Unknown,
}

impl LanguageType {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "rs" => Some(Self::Rust),
            _ => None,
        }
    }

    pub fn grammar(&self) -> Option<Language> {
        match self {
            Self::Rust => Some(tree_sitter_rust::language()),
            Self::Unknown => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub file: PathBuf,
    pub language: LanguageType,
    pub start_line: usize,
    pub end_line: usize,
    pub parameters: Vec<String>,
    pub return_type: Option<String>,
    pub calls: Vec<FunctionCall>,
}

#[derive(Debug, Clone)]
pub struct FunctionCall {
    pub callee: String,
    pub line: usize,
    pub kind: CallKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallKind {
    Direct,
    Method,
    Trait,
    Async,
}

#[derive(Debug, Clone, Default)]
pub struct ProgramGraph {
    pub functions: Vec<FunctionInfo>,
    pub edges: Vec<CallEdge>,
    pub files: HashSet<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct CallEdge {
    pub source: String,
    pub target: String,
    pub kind: CallKind,
}

pub struct AnalyzerPool {
    parser: Parser,
}

impl AnalyzerPool {
    pub fn new() -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::language())
            .expect("Failed to set Rust grammar");
        Self { parser }
    }

    pub fn parse(&mut self, source: &str) -> Option<Tree> {
        self.parser.parse(source, None)
    }
}

impl Default for AnalyzerPool {
    fn default() -> Self {
        Self::new()
    }
}

pub fn analyze_file(
    path: &Path,
    source: &str,
    pool: &mut AnalyzerPool,
) -> Option<Vec<FunctionInfo>> {
    let ext = path.extension()?.to_str()?;
    let lang = LanguageType::from_extension(ext)?;

    let grammar = lang.grammar()?;
    let mut parser = Parser::new();
    parser.set_language(&grammar).ok()?;
    let tree = parser.parse(source, None)?;
    let root = tree.root_node();

    let query = tree_sitter::Query::new(&grammar, RUST_FUNCTION_QUERY).ok()?;
    let mut cursor = tree_sitter::QueryCursor::new();
    let matches = cursor.matches(&query, root, source.as_bytes());

    let mut functions = Vec::new();
    for m in matches {
        let mut name = None;
        let mut params = Vec::new();
        let mut ret_type = None;

        for capture in m.captures {
            let node = capture.node;
            let capture_name = &query.capture_names()[capture.index as usize];
            let text = node.utf8_text(source.as_bytes()).unwrap_or("");

            match *capture_name {
                "name" => name = Some(text.to_string()),
                "params" => params.push(text.to_string()),
                "ret" => ret_type = Some(text.to_string()),
                _ => {}
            }
        }

        if let Some(name) = name {
            functions.push(FunctionInfo {
                name,
                file: path.to_path_buf(),
                language: lang,
                start_line: root.start_position().row + 1,
                end_line: root.end_position().row + 1,
                parameters: params,
                return_type: ret_type,
                calls: Vec::new(),
            });
        }
    }

    Some(functions)
}

const RUST_FUNCTION_QUERY: &str = r#"
    (function_item
        name: (identifier) @name
        parameters: (parameters) @params
        return_type: (type_identifier)? @ret)
"#;

pub fn analyze_directory(root: &Path) -> Result<ProgramGraph, String> {
    let mut graph = ProgramGraph::default();
    let files = collect_source_files(root)?;

    let mut pool = AnalyzerPool::new();
    let mut func_names: HashSet<String> = HashSet::new();

    for path in &files {
        if let Ok(source) = std::fs::read_to_string(path) {
            if let Some(funcs) = analyze_file(path, &source, &mut pool) {
                graph.files.insert(path.clone());
                for func in &funcs {
                    func_names.insert(func.name.clone());
                }
                graph.functions.extend(funcs);
            }
        }
    }

    // Extract call edges from function bodies
    for func in &mut graph.functions {
        let source = std::fs::read_to_string(&func.file).unwrap_or_default();
        for line_num in func.start_line..=func.end_line {
            if let Some(line) = source.lines().nth(line_num.saturating_sub(1)) {
                for word in line.split_whitespace() {
                    let word = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                    if func_names.contains(word) && word != func.name {
                        func.calls.push(FunctionCall {
                            callee: word.to_string(),
                            line: line_num,
                            kind: CallKind::Direct,
                        });
                        graph.edges.push(CallEdge {
                            source: func.name.clone(),
                            target: word.to_string(),
                            kind: CallKind::Direct,
                        });
                    }
                }
            }
        }
    }

    Ok(graph)
}

fn collect_source_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    walk_source(root, &mut files)?;
    Ok(files)
}

fn walk_source(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if path.is_dir() {
            if path.file_name().map_or(false, |n| {
                n == ".git" || n == "target" || n == "node_modules"
            }) {
                continue;
            }
            walk_source(&path, files)?;
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .map_or(false, |ext| LanguageType::from_extension(ext).is_some())
        {
            files.push(path);
        }
    }
    Ok(())
}

pub fn graph_to_json(graph: &ProgramGraph) -> String {
    let mut json = String::from("{\n  \"files\": [\n");
    let files: Vec<_> = graph.files.iter().collect();
    for (i, file) in files.iter().enumerate() {
        json.push_str(&format!("    \"{}\"", file.display()));
        if i < files.len() - 1 {
            json.push(',');
        }
        json.push('\n');
    }

    json.push_str("  ],\n  \"functions\": [\n");
    for (i, func) in graph.functions.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"name\": \"{}\", \"file\": \"{}\", \"language\": \"{:?}\"}}",
            func.name,
            func.file.display(),
            func.language
        ));
        if i < graph.functions.len() - 1 {
            json.push(',');
        }
        json.push('\n');
    }

    json.push_str("  ],\n  \"edges\": [\n");
    for (i, edge) in graph.edges.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"source\": \"{}\", \"target\": \"{}\"}}",
            edge.source, edge.target
        ));
        if i < graph.edges.len() - 1 {
            json.push(',');
        }
        json.push('\n');
    }

    json.push_str("  ]\n}");
    json
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_detection() {
        assert_eq!(LanguageType::from_extension("rs"), Some(LanguageType::Rust));
        assert_eq!(LanguageType::from_extension("py"), None);
        assert_eq!(LanguageType::from_extension("txt"), None);
    }
}
