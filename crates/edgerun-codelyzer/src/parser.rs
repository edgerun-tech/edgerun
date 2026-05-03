use std::{borrow::Cow, collections::HashMap, fs};

use serde::{Deserialize, Serialize};
use tree_sitter::{Language, Node, Parser};

// Language grammars - feature gated
#[cfg(feature = "lang-c")]
use tree_sitter_c::LANGUAGE as TS_C;
#[cfg(feature = "lang-go")]
use tree_sitter_go::LANGUAGE as TS_GO;
#[cfg(feature = "lang-java")]
use tree_sitter_java::LANGUAGE as TS_JAVA;
#[cfg(feature = "lang-javascript")]
use tree_sitter_javascript::LANGUAGE as TS_JS;
#[cfg(feature = "lang-python")]
use tree_sitter_python::LANGUAGE as TS_PYTHON;
#[cfg(feature = "lang-rust")]
use tree_sitter_rust::LANGUAGE as TS_RUST;
#[cfg(feature = "lang-typescript")]
use tree_sitter_typescript::LANGUAGE_TSX as TS_TS;

/// Parser module using tree-sitter to extract functions and call relationships.
///
/// Supports Rust, TypeScript, C, Python, Go, Java, and JavaScript files.
/// For C: handles function definitions (including static), direct calls,
/// indirect calls via function pointers, and preprocessor macro detection.
use crate::uir::CallKind;

/// A raw function definition found in source code.
#[derive(Debug, Clone)]
pub struct RawFunction<'a> {
    pub name: &'a str,
    pub start_byte: usize,
    pub end_byte: usize,
    pub is_static: bool, // C: `static` keyword present
    #[allow(dead_code)]
    pub is_macro_def: bool, // C: this is a preproc_function_def, not a real function
}

/// Owned version of RawFunction for caching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawFunctionOwned {
    pub name: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub is_static: bool,
    pub is_macro_def: bool,
}

/// A raw call found in source code.
#[derive(Debug, Clone)]
pub struct RawCall<'a> {
    pub callee_name: Cow<'a, str>,
    pub start_byte: usize,
    pub end_byte: usize,
    pub kind: CallKind,
}

/// Owned version of RawCall for caching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawCallOwned {
    pub callee_name: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub kind: CallKind,
}

impl<'a> From<&RawFunction<'a>> for RawFunctionOwned {
    fn from(rf: &RawFunction<'a>) -> Self {
        RawFunctionOwned {
            name: rf.name.to_string(),
            start_byte: rf.start_byte,
            end_byte: rf.end_byte,
            is_static: rf.is_static,
            is_macro_def: rf.is_macro_def,
        }
    }
}

impl<'a> From<&RawCall<'a>> for RawCallOwned {
    fn from(rc: &RawCall<'a>) -> Self {
        RawCallOwned {
            callee_name: rc.callee_name.clone().into_owned(),
            start_byte: rc.start_byte,
            end_byte: rc.end_byte,
            kind: rc.kind,
        }
    }
}

/// Cached parse result that can be serialized.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedParseResult {
    pub functions: Vec<RawFunctionOwned>,
    pub calls: Vec<RawCallOwned>,
}

impl<'a> ParseResult<'a> {
    /// Convert a borrowed ParseResult to an owned CachedParseResult.
    pub fn to_owned(&self) -> CachedParseResult {
        CachedParseResult {
            functions: self.functions.iter().map(|f| f.into()).collect(),
            calls: self.calls.iter().map(|c| c.into()).collect(),
        }
    }
}

/// Result from parsing a single file.
#[derive(Debug)]
pub struct ParseResult<'a> {
    pub functions: Vec<RawFunction<'a>>,
    pub calls: Vec<RawCall<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Lang {
    Rust,
    TypeScript,
    C,
    Python,
    Go,
    Java,
    JavaScript,
}

/// A pool of tree-sitter parsers, one per language.
/// Reusing parsers avoids repeated language initialization overhead.
#[derive(Default)]
pub struct ParserPool {
    parsers: HashMap<Lang, Parser>,
}

impl ParserPool {
    /// Create a new empty parser pool.
    pub fn new() -> Self {
        Self { parsers: HashMap::new() }
    }

    /// Get or create a parser for the given language.
    fn get_or_insert(&mut self, lang: Lang) -> &mut Parser {
        self.parsers.entry(lang).or_insert_with(|| {
            let mut parser = Parser::new();
            parser.set_language(&lang_to_tree_sitter(lang)).expect("failed to set language");
            parser
        })
    }

    /// Parse a file using the pooled parser.
    /// Returns None if the file extension is not supported.
    pub fn parse_file<'a>(&mut self, file_path: &str, source: &'a str) -> Option<ParseResult<'a>> {
        let lang = lang_for_path(file_path)?;
        let parser = self.get_or_insert(lang);
        let tree = parser.parse(source, None)?;
        let root = tree.root_node();

        let functions = extract_functions(root, source, lang);
        let calls = extract_calls(root, source, lang);

        Some(ParseResult { functions, calls })
    }
}

/// Determine the language from a file path.
fn lang_for_path(path: &str) -> Option<Lang> {
    if path.ends_with(".rs") {
        #[cfg(feature = "lang-rust")]
        {
            return Some(Lang::Rust);
        }
        #[cfg(not(feature = "lang-rust"))]
        {
            return None;
        }
    } else if path.ends_with(".ts") || path.ends_with(".tsx") {
        #[cfg(feature = "lang-typescript")]
        {
            return Some(Lang::TypeScript);
        }
        #[cfg(not(feature = "lang-typescript"))]
        {
            return None;
        }
    } else if path.ends_with(".c") || path.ends_with(".h") {
        #[cfg(feature = "lang-c")]
        {
            return Some(Lang::C);
        }
        #[cfg(not(feature = "lang-c"))]
        {
            return None;
        }
    } else if path.ends_with(".py") {
        #[cfg(feature = "lang-python")]
        {
            return Some(Lang::Python);
        }
        #[cfg(not(feature = "lang-python"))]
        {
            return None;
        }
    } else if path.ends_with(".go") {
        #[cfg(feature = "lang-go")]
        {
            return Some(Lang::Go);
        }
        #[cfg(not(feature = "lang-go"))]
        {
            return None;
        }
    } else if path.ends_with(".java") {
        #[cfg(feature = "lang-java")]
        {
            return Some(Lang::Java);
        }
        #[cfg(not(feature = "lang-java"))]
        {
            return None;
        }
    } else if path.ends_with(".js") || path.ends_with(".jsx") || path.ends_with(".mjs") {
        #[cfg(feature = "lang-javascript")]
        {
            return Some(Lang::JavaScript);
        }
        #[cfg(not(feature = "lang-javascript"))]
        {
            return None;
        }
    } else {
        None
    }
}

fn lang_to_tree_sitter(lang: Lang) -> Language {
    match lang {
        #[cfg(feature = "lang-rust")]
        Lang::Rust => TS_RUST.into(),
        #[cfg(feature = "lang-typescript")]
        Lang::TypeScript => TS_TS.into(),
        #[cfg(feature = "lang-c")]
        Lang::C => TS_C.into(),
        #[cfg(feature = "lang-python")]
        Lang::Python => TS_PYTHON.into(),
        #[cfg(feature = "lang-go")]
        Lang::Go => TS_GO.into(),
        #[cfg(feature = "lang-java")]
        Lang::Java => TS_JAVA.into(),
        #[cfg(feature = "lang-javascript")]
        Lang::JavaScript => TS_JS.into(),
        #[allow(unreachable_patterns)]
        _ => panic!("Language not enabled via feature flag"),
    }
}

/// Parse a file and return extracted functions and call pairs.
///
/// This is a convenience function that creates a temporary parser
/// for single-file use. For parsing multiple files, use `ParserPool`
/// directly to avoid repeated parser initialization.
#[allow(dead_code)]
pub fn parse_file<'a>(file_path: &str, source: &'a str) -> Option<ParseResult<'a>> {
    let lang = lang_for_path(file_path)?;

    let mut parser = Parser::new();
    parser.set_language(&lang_to_tree_sitter(lang)).ok()?;

    let tree = parser.parse(source, None)?;
    let root = tree.root_node();

    let functions = extract_functions(root, source, lang);
    let calls = extract_calls(root, source, lang);

    Some(ParseResult { functions, calls })
}

// ─── Function extraction ─────────────────────────────────────────────

fn extract_functions<'a>(node: Node, source: &'a str, lang: Lang) -> Vec<RawFunction<'a>> {
    let mut results = Vec::new();
    match lang {
        #[cfg(feature = "lang-rust")]
        Lang::Rust => collect_functions_rust(node, source, &mut results),
        #[cfg(feature = "lang-typescript")]
        Lang::TypeScript => collect_functions_ts(node, source, &mut results),
        #[cfg(feature = "lang-c")]
        Lang::C => collect_functions_c(node, source, &mut results),
        #[cfg(feature = "lang-python")]
        Lang::Python => collect_functions_python(node, source, &mut results),
        #[cfg(feature = "lang-go")]
        Lang::Go => collect_functions_go(node, source, &mut results),
        #[cfg(feature = "lang-java")]
        Lang::Java => collect_functions_java(node, source, &mut results),
        #[cfg(feature = "lang-javascript")]
        Lang::JavaScript => collect_functions_js(node, source, &mut results),
        #[allow(unreachable_patterns)]
        _ => {}
    }
    results
}

fn collect_functions_rust<'a>(node: Node, source: &'a str, out: &mut Vec<RawFunction<'a>>) {
    if node.kind() == "function_item" {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = node_text(name_node, source);
            out.push(RawFunction {
                name,
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
                is_static: false,
                is_macro_def: false,
            });
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_functions_rust(child, source, out);
    }
}

fn collect_functions_ts<'a>(node: Node, source: &'a str, out: &mut Vec<RawFunction<'a>>) {
    if node.kind() == "function_declaration" || node.kind() == "method_definition" {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = node_text(name_node, source);
            out.push(RawFunction {
                name,
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
                is_static: false,
                is_macro_def: false,
            });
        }
    }
    // Arrow functions: const foo = () => ...
    if node.kind() == "lexical_declaration" {
        for child in node.children(&mut node.walk()) {
            if child.kind() == "variable_declarator" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    if let Some(value_node) = child.child_by_field_name("value") {
                        if value_node.kind() == "arrow_function" || value_node.kind() == "function"
                        {
                            let name = node_text(name_node, source);
                            out.push(RawFunction {
                                name,
                                start_byte: child.start_byte(),
                                end_byte: child.end_byte(),
                                is_static: false,
                                is_macro_def: false,
                            });
                        }
                    }
                }
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_functions_ts(child, source, out);
    }
}

fn collect_functions_c<'a>(node: Node, source: &'a str, out: &mut Vec<RawFunction<'a>>) {
    // Regular function definition
    if node.kind() == "function_definition" {
        if let Some(declarator) = find_function_declarator(node) {
            let name = node_text(declarator, source);
            // Check for `static` storage class specifier
            let is_static = has_static_specifier(node, source);
            out.push(RawFunction {
                name,
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
                is_static,
                is_macro_def: false,
            });
        }
    }
    // Preprocessor function definitions: #define FOO(x) ...
    // These look like functions but are macros
    if node.kind() == "preproc_function_def" {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = node_text(name_node, source);
            out.push(RawFunction {
                name,
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
                is_static: false,
                is_macro_def: true,
            });
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_functions_c(child, source, out);
    }
}

fn collect_functions_python<'a>(node: Node, source: &'a str, out: &mut Vec<RawFunction<'a>>) {
    // function_definition (top-level or nested functions)
    if node.kind() == "function_definition" {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = node_text(name_node, source);
            out.push(RawFunction {
                name,
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
                is_static: false,
                is_macro_def: false,
            });
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_functions_python(child, source, out);
    }
}

fn collect_functions_go<'a>(node: Node, source: &'a str, out: &mut Vec<RawFunction<'a>>) {
    // function_declaration: func foo() { ... }
    // method_declaration: func (r Receiver) foo() { ... }
    if node.kind() == "function_declaration" || node.kind() == "method_declaration" {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = node_text(name_node, source);
            out.push(RawFunction {
                name,
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
                is_static: false,
                is_macro_def: false,
            });
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_functions_go(child, source, out);
    }
}

fn collect_functions_java<'a>(node: Node, source: &'a str, out: &mut Vec<RawFunction<'a>>) {
    // method_declaration: void foo() { ... }
    // constructor_declaration: ClassName() { ... }
    if node.kind() == "method_declaration" {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = node_text(name_node, source);
            out.push(RawFunction {
                name,
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
                is_static: false,
                is_macro_def: false,
            });
        }
    }
    if node.kind() == "constructor_declaration" {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = node_text(name_node, source);
            out.push(RawFunction {
                name,
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
                is_static: false,
                is_macro_def: false,
            });
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_functions_java(child, source, out);
    }
}

fn collect_functions_js<'a>(node: Node, source: &'a str, out: &mut Vec<RawFunction<'a>>) {
    // function_declaration: function foo() { ... }
    if node.kind() == "function_declaration" {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = node_text(name_node, source);
            out.push(RawFunction {
                name,
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
                is_static: false,
                is_macro_def: false,
            });
        }
    }
    // function_expression: const foo = function() { ... }
    if node.kind() == "function_expression" {
        // Named function expressions
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = node_text(name_node, source);
            out.push(RawFunction {
                name,
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
                is_static: false,
                is_macro_def: false,
            });
        }
    }
    // Arrow functions assigned to variables: const foo = () => ...
    if node.kind() == "lexical_declaration" || node.kind() == "variable_declaration" {
        for child in node.children(&mut node.walk()) {
            if child.kind() == "variable_declarator" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    if let Some(value_node) = child.child_by_field_name("value") {
                        if value_node.kind() == "arrow_function"
                            || value_node.kind() == "function_expression"
                        {
                            let name = node_text(name_node, source);
                            out.push(RawFunction {
                                name,
                                start_byte: child.start_byte(),
                                end_byte: child.end_byte(),
                                is_static: false,
                                is_macro_def: false,
                            });
                        }
                    }
                }
            }
        }
    }
    // method_definition in classes: class Foo { bar() { ... } }
    if node.kind() == "method_definition" {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = node_text(name_node, source);
            out.push(RawFunction {
                name,
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
                is_static: false,
                is_macro_def: false,
            });
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_functions_js(child, source, out);
    }
}

/// Find the declarator (name) of a C function_definition node.
fn find_function_declarator(node: Node) -> Option<Node> {
    // The declarator is a direct child, but tree-sitter-c wraps it
    // in pointer_declarator for pointer return types. We need to
    // dig down to find the identifier.
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "function_declarator" {
            // Inside function_declarator, the first child is the identifier
            if let Some(ident) = child.child(0) {
                if ident.kind() == "identifier" {
                    return Some(ident);
                }
            }
        }
        // Sometimes it's nested: pointer_declarator -> function_declarator
        if child.kind() == "pointer_declarator" {
            if let Some(inner) = child.child_by_field_name("declarator") {
                if inner.kind() == "function_declarator" {
                    if let Some(ident) = inner.child(0) {
                        if ident.kind() == "identifier" {
                            return Some(ident);
                        }
                    }
                }
            }
        }
        // Also: parenthesized_declarator -> function_declarator
        if child.kind() == "parenthesized_declarator" {
            if let Some(inner) = child.child(0) {
                if inner.kind() == "function_declarator" {
                    if let Some(ident) = inner.child(0) {
                        if ident.kind() == "identifier" {
                            return Some(ident);
                        }
                    }
                }
            }
        }
    }
    None
}

/// Check if a C function_definition has `static` storage class specifier.
fn has_static_specifier(node: Node, source: &str) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "storage_class_specifier" {
            let text = node_text(child, source);
            if text == "static" {
                return true;
            }
        }
    }
    false
}

// ─── Call extraction ─────────────────────────────────────────────────

fn extract_calls<'a>(node: Node, source: &'a str, lang: Lang) -> Vec<RawCall<'a>> {
    let mut results = Vec::new();
    match lang {
        #[cfg(any(feature = "lang-rust", feature = "lang-typescript"))]
        Lang::Rust | Lang::TypeScript => collect_calls_rust_ts(node, source, &mut results),
        #[cfg(feature = "lang-c")]
        Lang::C => collect_calls_c(node, source, &mut results),
        #[cfg(feature = "lang-python")]
        Lang::Python => collect_calls_python(node, source, &mut results),
        #[cfg(feature = "lang-go")]
        Lang::Go => collect_calls_go(node, source, &mut results),
        #[cfg(feature = "lang-java")]
        Lang::Java => collect_calls_java(node, source, &mut results),
        #[cfg(feature = "lang-javascript")]
        Lang::JavaScript => collect_calls_js(node, source, &mut results),
        #[allow(unreachable_patterns)]
        _ => {}
    }
    results
}

fn collect_calls_rust_ts<'a>(node: Node, source: &'a str, out: &mut Vec<RawCall<'a>>) {
    if node.kind() == "call_expression" {
        if let Some(func_node) = node.child_by_field_name("function") {
            if func_node.kind() == "identifier" {
                let name = node_text(func_node, source);
                out.push(RawCall {
                    callee_name: Cow::Borrowed(name),
                    start_byte: node.start_byte(),
                    end_byte: node.end_byte(),
                    kind: CallKind::Direct,
                });
            } else if func_node.kind() == "member_expression" {
                if let Some(prop) = func_node.child_by_field_name("property") {
                    let name = node_text(prop, source);
                    out.push(RawCall {
                        callee_name: Cow::Borrowed(name),
                        start_byte: node.start_byte(),
                        end_byte: node.end_byte(),
                        kind: CallKind::Direct,
                    });
                }
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_calls_rust_ts(child, source, out);
    }
}

fn collect_calls_c<'a>(node: Node, source: &'a str, out: &mut Vec<RawCall<'a>>) {
    // Direct call: foo(args)
    if node.kind() == "call_expression" {
        let function = node.child_by_field_name("function");
        if let Some(func_node) = function {
            match func_node.kind() {
                "identifier" => {
                    let name = node_text(func_node, source);
                    let kind = if is_likely_macro(name, source, node) {
                        CallKind::Macro
                    } else {
                        CallKind::Direct
                    };
                    out.push(RawCall {
                        callee_name: Cow::Borrowed(name),
                        start_byte: node.start_byte(),
                        end_byte: node.end_byte(),
                        kind,
                    });
                }
                // Indirect: ptr->method(args) or obj.method(args)
                "field_expression" => {
                    if let Some(prop) = func_node.child_by_field_name("field") {
                        let name = node_text(prop, source);
                        out.push(RawCall {
                            callee_name: Cow::Borrowed(name),
                            start_byte: node.start_byte(),
                            end_byte: node.end_byte(),
                            kind: CallKind::Indirect,
                        });
                    }
                }
                // Indirect: (*fp)(args)
                "parenthesized_expression" => {
                    let inner_text = node_text(func_node, source);
                    let synthesized =
                        format!("INDIRECT_{}", inner_text.replace(['(', ')', '*', ' '], "_"));
                    out.push(RawCall {
                        callee_name: Cow::Owned(synthesized),
                        start_byte: node.start_byte(),
                        end_byte: node.end_byte(),
                        kind: CallKind::Indirect,
                    });
                }
                _ => {
                    let name = node_text(func_node, source);
                    out.push(RawCall {
                        callee_name: Cow::Borrowed(name),
                        start_byte: node.start_byte(),
                        end_byte: node.end_byte(),
                        kind: CallKind::Unknown,
                    });
                }
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_calls_c(child, source, out);
    }
}

fn collect_calls_python<'a>(node: Node, source: &'a str, out: &mut Vec<RawCall<'a>>) {
    // call_expression: foo(), obj.method(), self.helper()
    if node.kind() == "call" {
        if let Some(func_node) = node.child_by_field_name("function") {
            match func_node.kind() {
                "identifier" => {
                    let name = node_text(func_node, source);
                    out.push(RawCall {
                        callee_name: Cow::Borrowed(name),
                        start_byte: node.start_byte(),
                        end_byte: node.end_byte(),
                        kind: CallKind::Direct,
                    });
                }
                "attribute" => {
                    // obj.method() or self.method()
                    if let Some(attr_node) = func_node.child_by_field_name("attribute") {
                        let name = node_text(attr_node, source);
                        out.push(RawCall {
                            callee_name: Cow::Borrowed(name),
                            start_byte: node.start_byte(),
                            end_byte: node.end_byte(),
                            kind: CallKind::Direct,
                        });
                    }
                }
                _ => {
                    let name = node_text(func_node, source);
                    out.push(RawCall {
                        callee_name: Cow::Borrowed(name),
                        start_byte: node.start_byte(),
                        end_byte: node.end_byte(),
                        kind: CallKind::Unknown,
                    });
                }
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_calls_python(child, source, out);
    }
}

fn collect_calls_go<'a>(node: Node, source: &'a str, out: &mut Vec<RawCall<'a>>) {
    // call_expression: foo(), pkg.Func(), m.Method()
    if node.kind() == "call_expression" {
        if let Some(func_node) = node.child_by_field_name("function") {
            match func_node.kind() {
                "identifier" => {
                    let name = node_text(func_node, source);
                    out.push(RawCall {
                        callee_name: Cow::Borrowed(name),
                        start_byte: node.start_byte(),
                        end_byte: node.end_byte(),
                        kind: CallKind::Direct,
                    });
                }
                "selector_expression" => {
                    // pkg.Func() or m.Method()
                    if let Some(field_node) = func_node.child_by_field_name("field") {
                        let name = node_text(field_node, source);
                        out.push(RawCall {
                            callee_name: Cow::Borrowed(name),
                            start_byte: node.start_byte(),
                            end_byte: node.end_byte(),
                            kind: CallKind::Direct,
                        });
                    }
                }
                _ => {
                    let name = node_text(func_node, source);
                    out.push(RawCall {
                        callee_name: Cow::Borrowed(name),
                        start_byte: node.start_byte(),
                        end_byte: node.end_byte(),
                        kind: CallKind::Unknown,
                    });
                }
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_calls_go(child, source, out);
    }
}

fn collect_calls_java<'a>(node: Node, source: &'a str, out: &mut Vec<RawCall<'a>>) {
    // method_invocation: foo(), obj.method(), System.out.println()
    if node.kind() == "method_invocation" {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = node_text(name_node, source);
            out.push(RawCall {
                callee_name: Cow::Borrowed(name),
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
                kind: CallKind::Direct,
            });
        }
    }
    // object_creation: new Foo()
    if node.kind() == "object_creation" {
        // The type being constructed
        if let Some(type_node) = node.child_by_field_name("type") {
            // For constructor calls, the constructor name is part of the type
            if let Some(ident) = type_node.descendant_for_byte_range(
                type_node.start_byte(),
                type_node.start_byte().saturating_add(1),
            ) {
                let name = node_text(ident, source);
                out.push(RawCall {
                    callee_name: Cow::Borrowed(name),
                    start_byte: node.start_byte(),
                    end_byte: node.end_byte(),
                    kind: CallKind::Direct,
                });
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_calls_java(child, source, out);
    }
}

fn collect_calls_js<'a>(node: Node, source: &'a str, out: &mut Vec<RawCall<'a>>) {
    // call_expression: foo(), obj.method()
    if node.kind() == "call_expression" {
        if let Some(func_node) = node.child_by_field_name("function") {
            match func_node.kind() {
                "identifier" => {
                    let name = node_text(func_node, source);
                    out.push(RawCall {
                        callee_name: Cow::Borrowed(name),
                        start_byte: node.start_byte(),
                        end_byte: node.end_byte(),
                        kind: CallKind::Direct,
                    });
                }
                "member_expression" => {
                    if let Some(prop) = func_node.child_by_field_name("property") {
                        let name = node_text(prop, source);
                        out.push(RawCall {
                            callee_name: Cow::Borrowed(name),
                            start_byte: node.start_byte(),
                            end_byte: node.end_byte(),
                            kind: CallKind::Direct,
                        });
                    }
                }
                _ => {
                    let name = node_text(func_node, source);
                    out.push(RawCall {
                        callee_name: Cow::Borrowed(name),
                        start_byte: node.start_byte(),
                        end_byte: node.end_byte(),
                        kind: CallKind::Unknown,
                    });
                }
            }
        }
    }
    // new_expression: new Foo()
    if node.kind() == "new_expression" {
        if let Some(ctor_node) = node.child_by_field_name("constructor") {
            if ctor_node.kind() == "identifier" {
                let name = node_text(ctor_node, source);
                out.push(RawCall {
                    callee_name: Cow::Borrowed(name),
                    start_byte: node.start_byte(),
                    end_byte: node.end_byte(),
                    kind: CallKind::Direct,
                });
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_calls_js(child, source, out);
    }
}

/// Heuristic: a call `NAME(args)` where NAME is uppercase and short
/// is likely a macro in kernel code.
fn is_likely_macro(name: &str, _source: &str, _call_node: Node) -> bool {
    // Kernel convention: ALL_CAPS names are usually macros
    if name.len() > 1
        && name.chars().all(|c| c.is_ascii_uppercase() || c == '_')
        && !name.starts_with('_')
    {
        return true;
    }
    false
}

// ─── Call-to-function pairing ────────────────────────────────────────

/// For each function, find calls that fall within its byte range and
/// create (caller_name, RawCall) pairs.
/// Owned version of build_call_pairs for use with cached data.
/// Returns Vec<(caller_name, RawCallOwned)> pairs.
pub fn build_call_pairs_owned(
    functions: &[RawFunctionOwned],
    all_calls: &[RawCallOwned],
) -> Vec<(String, RawCallOwned)> {
    let mut pairs = Vec::new();

    let mut call_indices: Vec<usize> = (0..all_calls.len()).collect();
    call_indices.sort_by_key(|&i| all_calls[i].start_byte);

    for func in functions {
        let start = call_indices.partition_point(|&i| all_calls[i].start_byte < func.start_byte);

        for &ci in &call_indices[start..] {
            let call = &all_calls[ci];
            if call.start_byte > func.end_byte {
                break;
            }
            if call.end_byte <= func.end_byte && call.callee_name != func.name {
                pairs.push((func.name.clone(), call.clone()));
            }
        }
    }

    pairs
}

// ─── Directory scanning ──────────────────────────────────────────────

/// Recursively collect all .rs, .ts, .tsx, .c, .h, .py, .go, .java, .js, .jsx,
/// .mjs files from a directory.
#[allow(dead_code)]
pub fn collect_source_files(dir: &str) -> Vec<String> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with('.')
                        || name == "target"
                        || name == "node_modules"
                        || name == ".git"
                        || name == "__pycache__"
                        || name == ".venv"
                    {
                        continue;
                    }
                }
                let sub = path.to_string_lossy().to_string();
                files.extend(collect_source_files(&sub));
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext == "rs"
                    || ext == "ts"
                    || ext == "tsx"
                    || ext == "c"
                    || ext == "h"
                    || ext == "py"
                    || ext == "go"
                    || ext == "java"
                    || ext == "js"
                    || ext == "jsx"
                    || ext == "mjs"
                {
                    files.push(path.to_string_lossy().to_string());
                }
            }
        }
    }
    files
}

// ─── Helpers ─────────────────────────────────────────────────────────

fn node_text<'a>(node: Node, source: &'a str) -> &'a str {
    node.utf8_text(source.as_bytes()).unwrap_or("")
}

// ─── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_c_simple_function() {
        let source = r#"
void foo() {
    bar();
}
"#;
        let result = parse_file("test.c", source).expect("parse failed");
        assert_eq!(result.functions.len(), 1);
        assert_eq!(result.functions[0].name, "foo");
        assert!(!result.functions[0].is_static);
    }

    #[test]
    fn test_parse_c_static_function() {
        let source = r#"
static int helper(void) {
    return 42;
}
"#;
        let result = parse_file("test.c", source).expect("parse failed");
        assert_eq!(result.functions.len(), 1);
        assert_eq!(result.functions[0].name, "helper");
        assert!(result.functions[0].is_static);
    }

    #[test]
    fn test_parse_c_call_detection() {
        let source = r#"
void caller() {
    callee();
    another_call(1, 2);
}
"#;
        let result = parse_file("test.c", source).expect("parse failed");
        assert_eq!(result.calls.len(), 2);
    }

    #[test]
    fn test_parse_c_macro_definition() {
        let source = r#"
#define FOO(x) ((x) + 1)

void bar() {
    int y = FOO(5);
}
"#;
        let result = parse_file("test.c", source).expect("parse failed");
        // Should detect the macro def
        assert!(result.functions.iter().any(|f| f.is_macro_def));
        // Should detect the macro call in bar
        assert!(result.calls.iter().any(|c| c.kind == CallKind::Macro));
    }

    #[test]
    fn test_parse_c_indirect_call() {
        let source = r#"
void test() {
    fn_ptr(arg);
}
"#;
        let result = parse_file("test.c", source).expect("parse failed");
        assert!(!result.functions.is_empty());
    }

    #[test]
    fn test_parse_rust_function() {
        let source = r#"
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

fn helper() {}
"#;
        let result = parse_file("test.rs", source).expect("parse failed");
        assert_eq!(result.functions.len(), 2);
        assert_eq!(result.functions[0].name, "greet");
        assert_eq!(result.functions[1].name, "helper");
    }

    #[test]
    fn test_parse_rust_method() {
        let source = r#"
impl Foo {
    pub fn bar(&self) {
        self.baz();
    }
}
"#;
        let result = parse_file("test.rs", source).expect("parse failed");
        assert_eq!(result.functions.len(), 1);
        assert_eq!(result.functions[0].name, "bar");
    }

    #[test]
    fn test_parse_typescript_function() {
        let source = r#"
function greet(name: string): void {
    console.log(name);
}

const arrow = () => { return 42; };
"#;
        let result = parse_file("test.ts", source).expect("parse failed");
        assert_eq!(result.functions.len(), 2);
    }

    #[test]
    fn test_build_call_pairs() {
        let funcs = vec![
            RawFunctionOwned {
                name: "foo".into(),
                start_byte: 0,
                end_byte: 30,
                is_static: false,
                is_macro_def: false,
            },
            RawFunctionOwned {
                name: "bar".into(),
                start_byte: 30,
                end_byte: 60,
                is_static: false,
                is_macro_def: false,
            },
        ];
        let calls = vec![
            RawCallOwned {
                callee_name: "bar".into(),
                start_byte: 10,
                end_byte: 15,
                kind: CallKind::Direct,
            },
            RawCallOwned {
                callee_name: "external".into(),
                start_byte: 20,
                end_byte: 30,
                kind: CallKind::Direct,
            },
        ];
        let pairs = build_call_pairs_owned(&funcs, &calls);
        assert_eq!(pairs.len(), 2);
        // foo -> bar (resolved)
        let foo_bar =
            pairs.iter().find(|(caller, call)| *caller == "foo" && call.callee_name == "bar");
        assert!(foo_bar.is_some());
        // foo -> external (unresolved)
        let foo_ext =
            pairs.iter().find(|(caller, call)| *caller == "foo" && call.callee_name == "external");
        assert!(foo_ext.is_some());
    }

    #[test]
    fn test_parser_pool() {
        let mut pool = ParserPool::new();
        let source = "void foo() { bar(); }";

        // First parse
        let r1 = pool.parse_file("test.c", source);
        assert!(r1.is_some());
        let funcs1 = r1.unwrap();
        assert_eq!(funcs1.functions.len(), 1);

        // Reuse pool — should reuse parser
        let r2 = pool.parse_file("test2.c", source);
        assert!(r2.is_some());
    }

    #[test]
    fn test_empty_source() {
        let result = parse_file("empty.c", "");
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.functions.len(), 0);
        assert_eq!(r.calls.len(), 0);
    }

    #[test]
    fn test_multiple_c_functions() {
        let source = r#"
int main() { return 0; }
static void init() {}
void cleanup(void) {}
"#;
        let result = parse_file("test.c", source).expect("parse failed");
        assert_eq!(result.functions.len(), 3);
        assert!(result.functions.iter().any(|f| f.name == "main" && !f.is_static));
        assert!(result.functions.iter().any(|f| f.name == "init" && f.is_static));
        assert!(result.functions.iter().any(|f| f.name == "cleanup" && !f.is_static));
    }
}
