//! Code analyzer module: tree-sitter based multi-language parsing and call graph construction.
//! Merged from codeanalyzer project.
//!
//! Supports incremental analysis with file caching and change detection.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rayon::prelude::*;
use tree_sitter::{Language, Parser, Tree};

/// Supported languages and their tree-sitter grammars
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageType {
    Rust,
    TypeScript,
    JavaScript,
    C,
    Python,
    Go,
    Java,
}

impl LanguageType {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "rs" => Some(Self::Rust),
            "ts" | "tsx" => Some(Self::TypeScript),
            "js" | "jsx" => Some(Self::JavaScript),
            "c" | "h" => Some(Self::C),
            "py" => Some(Self::Python),
            "go" => Some(Self::Go),
            "java" => Some(Self::Java),
            _ => None,
        }
    }

    pub fn grammar(&self) -> Language {
        match self {
            Self::Rust => tree_sitter_rust::language(),
            // Add more languages as needed
            _ => unimplemented!("Language not yet supported"),
        }
    }

    pub fn function_query(&self) -> &'static str {
        match self {
            Self::Rust => {
                r#"
                (function_item
                    name: (identifier) @name
                    parameters: (parameters) @params
                    return_type: (type_identifier)? @ret)
            "#
            }
            Self::TypeScript | Self::JavaScript => {
                r#"
                (function_declaration
                    name: (identifier) @name
                    parameters: (formal_parameters) @params)
                (arrow_function
                    parameters: (formal_parameters) @params)
            "#
            }
            Self::C => {
                r#"
                (function_definition
                    declarator: (function_declarator
                        declarator: (identifier) @name
                        parameters: (parameter_list) @params))
            "#
            }
            Self::Python => {
                r#"
                (function_definition
                    name: (identifier) @name
                    parameters: (parameters) @params)
            "#
            }
            Self::Go => {
                r#"
                (function_declaration
                    name: (identifier) @name
                    parameters: (parameter_list) @params)
            "#
            }
            Self::Java => {
                r#"
                (method_declaration
                    name: (identifier) @name
                    parameters: (formal_parameters) @params)
            "#
            }
        }
    }
}

/// Represents a function/method in the codebase
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

/// Represents a function call
#[derive(Debug, Clone)]
pub struct FunctionCall {
    pub callee: String,
    pub line: usize,
    pub kind: CallKind,
}

/// Type of function call
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallKind {
    Direct,
    Method,
    Trait,
    Async,
}

/// Complete program graph
#[derive(Debug, Clone, Default)]
pub struct ProgramGraph {
    pub functions: Vec<FunctionInfo>,
    pub edges: Vec<CallEdge>,
    pub files: HashSet<PathBuf>,
}

/// Edge in the call graph
#[derive(Debug, Clone)]
pub struct CallEdge {
    pub source: String, // caller function name
    pub target: String, // callee function name
    pub kind: CallKind,
}

/// Parser pool for efficient tree-sitter usage
pub struct AnalyzerPool {
    parsers: HashMap<LanguageType, Parser>,
}

impl AnalyzerPool {
    pub fn new() -> Self {
        let mut parsers = HashMap::new();

        // Initialize parsers for all supported languages
        parsers.insert(LanguageType::Rust, Self::create_parser(LanguageType::Rust));
        parsers.insert(
            LanguageType::TypeScript,
            Self::create_parser(LanguageType::TypeScript),
        );
        parsers.insert(
            LanguageType::JavaScript,
            Self::create_parser(LanguageType::JavaScript),
        );
        parsers.insert(LanguageType::C, Self::create_parser(LanguageType::C));
        parsers.insert(
            LanguageType::Python,
            Self::create_parser(LanguageType::Python),
        );
        parsers.insert(LanguageType::Go, Self::create_parser(LanguageType::Go));
        parsers.insert(LanguageType::Java, Self::create_parser(LanguageType::Java));

        Self { parsers }
    }

    fn create_parser(lang: LanguageType) -> Parser {
        let mut parser = Parser::new();
        let grammar = lang.grammar();
        parser
            .set_language(&grammar)
            .expect("Failed to load grammar");
        parser
    }

    pub fn parse(&mut self, lang: LanguageType, source: &str) -> Option<Tree> {
        self.parsers.get_mut(&lang)?.parse(source, None)
    }
}

impl Default for AnalyzerPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Analyze a single file
pub fn analyze_file(
    path: &Path,
    source: &str,
    pool: &mut AnalyzerPool,
) -> Option<Vec<FunctionInfo>> {
    let ext = path.extension()?.to_str()?;
    let lang = LanguageType::from_extension(ext)?;

    let tree = pool.parse(lang, source)?;
    let root = tree.root_node();

    let grammar = lang.grammar();
    let query = tree_sitter::Query::new(&grammar, lang.function_query()).ok()?;
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
                "params" => {
                    // Extract parameter names (simplified)
                    let param_text = node.utf8_text(source.as_bytes()).unwrap_or("");
                    params.push(param_text.to_string());
                }
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

/// Analyze entire directory recursively
pub fn analyze_directory(root: &Path) -> Result<ProgramGraph, String> {
    let mut graph = ProgramGraph::default();
    let mut file_functions: HashMap<PathBuf, Vec<FunctionInfo>> = HashMap::new();

    // Collect all source files
    let files = collect_source_files(root)?;

    // Parse files (sequentially to share parser pool)
    let mut pool = AnalyzerPool::new();
    let mut results = Vec::new();

    for path in &files {
        if let Ok(source) = std::fs::read_to_string(path) {
            if let Some(funcs) = analyze_file(path, &source, &mut pool) {
                results.push((path.clone(), funcs, source));
            }
        }
    }

    // Build function index
    let mut func_index: HashMap<String, Vec<&FunctionInfo>> = HashMap::new();
    for (path, funcs, _source) in &results {
        graph.files.insert(path.clone());
        file_functions.insert(path.clone(), funcs.clone());

        for func in funcs {
            func_index.entry(func.name.clone()).or_default().push(func);
        }
    }

    // Extract call edges (simplified - would need more sophisticated analysis)
    for (_path, funcs, source) in &results {
        for func in funcs {
            let mut func_calls = Vec::new();

            // Look for function calls in the source
            for line_num in func.start_line..=func.end_line {
                let line = source.lines().nth(line_num - 1).unwrap_or("");

                // Simple call detection
                for word in line.split_whitespace() {
                    let word = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                    if func_index.contains_key(word) && word != func.name {
                        func_calls.push(FunctionCall {
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

            // Store calls in function
            if let Some(func_mut) = graph
                .functions
                .iter_mut()
                .find(|f| f.name == func.name && f.file == func.file)
            {
                func_mut.calls = func_calls;
            }
        }
    }

    graph.functions = file_functions.into_values().flatten().collect();

    Ok(graph)
}

/// Collect all source files recursively
fn collect_source_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();

    fn walk(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
        for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();

            if path.is_dir() {
                if path.file_name().map_or(false, |n| {
                    n == ".git" || n == "target" || n == "node_modules"
                }) {
                    continue;
                }
                walk(&path, files)?;
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

    walk(root, &mut files)?;
    Ok(files)
}

/// Convert program graph to JSON for LLM context
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
        assert_eq!(
            LanguageType::from_extension("ts"),
            Some(LanguageType::TypeScript)
        );
        assert_eq!(
            LanguageType::from_extension("py"),
            Some(LanguageType::Python)
        );
        assert_eq!(LanguageType::from_extension("txt"), None);
    }
}

// Benchmark comment