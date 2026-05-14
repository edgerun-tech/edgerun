use std::{borrow::Cow, fs};

use crate::uir::CallKind;

/// A raw function definition found in source code.
#[derive(Debug, Clone)]
pub struct RawFunction<'a> {
    pub name: &'a str,
    pub start_byte: usize,
    pub end_byte: usize,
    pub is_static: bool,
    #[allow(dead_code)]
    pub is_macro_def: bool,
}

/// Owned version of RawFunction for caching.
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lang {
    Rust,
    TypeScript,
    C,
    Python,
    Go,
    Java,
    JavaScript,
}

/// Parser pool kept for API stability. The local parser has no external
/// grammar state to cache.
#[derive(Default)]
pub struct ParserPool;

impl ParserPool {
    pub fn new() -> Self {
        Self
    }

    pub fn parse_file<'a>(&mut self, file_path: &str, source: &'a str) -> Option<ParseResult<'a>> {
        parse_file(file_path, source)
    }
}

/// Parse a file and return extracted functions and call pairs.
#[allow(dead_code)]
pub fn parse_file<'a>(file_path: &str, source: &'a str) -> Option<ParseResult<'a>> {
    let lang = lang_for_path(file_path)?;
    let clean = mask_comments_and_strings(source, lang);
    let mut functions = collect_functions(source, &clean, lang);
    finalize_function_ranges(source, &clean, lang, &mut functions);
    let calls = collect_calls(source, &clean, lang);
    Some(ParseResult { functions, calls })
}

fn lang_for_path(path: &str) -> Option<Lang> {
    if path.ends_with(".rs") {
        #[cfg(feature = "lang-rust")]
        return Some(Lang::Rust);
    } else if path.ends_with(".ts") || path.ends_with(".tsx") {
        #[cfg(feature = "lang-typescript")]
        return Some(Lang::TypeScript);
    } else if path.ends_with(".c") || path.ends_with(".h") {
        #[cfg(feature = "lang-c")]
        return Some(Lang::C);
    } else if path.ends_with(".py") {
        #[cfg(feature = "lang-python")]
        return Some(Lang::Python);
    } else if path.ends_with(".go") {
        #[cfg(feature = "lang-go")]
        return Some(Lang::Go);
    } else if path.ends_with(".java") {
        #[cfg(feature = "lang-java")]
        return Some(Lang::Java);
    } else if path.ends_with(".js") || path.ends_with(".jsx") || path.ends_with(".mjs") {
        #[cfg(feature = "lang-javascript")]
        return Some(Lang::JavaScript);
    }
    None
}

fn collect_functions<'a>(source: &'a str, clean: &str, lang: Lang) -> Vec<RawFunction<'a>> {
    let mut functions = Vec::new();

    match lang {
        Lang::Rust => collect_keyword_functions(source, clean, "fn ", false, &mut functions),
        Lang::TypeScript | Lang::JavaScript => {
            collect_keyword_functions(source, clean, "function ", false, &mut functions);
            collect_js_arrow_functions(source, clean, &mut functions);
            collect_js_methods(source, clean, &mut functions);
        }
        Lang::Python => collect_keyword_functions(source, clean, "def ", false, &mut functions),
        Lang::Go => collect_go_functions(source, clean, &mut functions),
        Lang::Java => collect_java_methods(source, clean, &mut functions),
        Lang::C => collect_c_functions(source, clean, &mut functions),
    }

    functions.sort_by_key(|f| f.start_byte);
    functions.dedup_by(|a, b| a.start_byte == b.start_byte && a.name == b.name);
    functions
}

fn collect_keyword_functions<'a>(
    source: &'a str,
    clean: &str,
    keyword: &str,
    is_static: bool,
    out: &mut Vec<RawFunction<'a>>,
) {
    let mut offset = 0;
    while let Some(rel) = clean[offset..].find(keyword) {
        let pos = offset + rel;
        if pos > 0 && is_ident_byte(clean.as_bytes()[pos - 1]) {
            offset = pos + keyword.len();
            continue;
        }
        let name_start = skip_ws(clean, pos + keyword.len());
        let Some(name_end) = read_ident_end(clean, name_start) else {
            offset = pos + keyword.len();
            continue;
        };
        out.push(RawFunction {
            name: &source[name_start..name_end],
            start_byte: pos,
            end_byte: source.len(),
            is_static,
            is_macro_def: false,
        });
        offset = name_end;
    }
}

fn collect_js_arrow_functions<'a>(source: &'a str, clean: &str, out: &mut Vec<RawFunction<'a>>) {
    let mut offset = 0;
    while let Some(rel) = clean[offset..].find("=>") {
        let arrow = offset + rel;
        let Some(eq) = clean[..arrow].rfind('=') else {
            offset = arrow + 2;
            continue;
        };
        let Some((name_start, name_end)) = previous_ident(clean, eq) else {
            offset = arrow + 2;
            continue;
        };
        out.push(RawFunction {
            name: &source[name_start..name_end],
            start_byte: name_start,
            end_byte: source.len(),
            is_static: false,
            is_macro_def: false,
        });
        offset = arrow + 2;
    }
}

fn collect_js_methods<'a>(source: &'a str, clean: &str, out: &mut Vec<RawFunction<'a>>) {
    let bytes = clean.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if !is_ident_start(bytes[i]) {
            i += 1;
            continue;
        }
        let start = i;
        i += 1;
        while i < bytes.len() && is_call_path_byte(bytes[i]) {
            i += 1;
        }
        let end = i;
        let name = &clean[start..end];
        if is_keyword(name) {
            continue;
        }
        if start > 0 && matches!(bytes[start - 1], b'.' | b':') {
            continue;
        }
        let paren = skip_ws(clean, end);
        if bytes.get(paren) == Some(&b'(') {
            let after = find_matching(clean, paren, b'(', b')')
                .map(|p| skip_ws(clean, p + 1))
                .unwrap_or(paren);
            if bytes.get(after) == Some(&b'{') {
                out.push(RawFunction {
                    name: &source[start..end],
                    start_byte: start,
                    end_byte: source.len(),
                    is_static: false,
                    is_macro_def: false,
                });
            }
        }
    }
}

fn collect_go_functions<'a>(source: &'a str, clean: &str, out: &mut Vec<RawFunction<'a>>) {
    let mut offset = 0;
    while let Some(rel) = clean[offset..].find("func ") {
        let pos = offset + rel;
        let mut cursor = skip_ws(clean, pos + 5);
        if clean.as_bytes().get(cursor) == Some(&b'(') {
            if let Some(end) = find_matching(clean, cursor, b'(', b')') {
                cursor = skip_ws(clean, end + 1);
            }
        }
        let Some(name_end) = read_ident_end(clean, cursor) else {
            offset = pos + 5;
            continue;
        };
        out.push(RawFunction {
            name: &source[cursor..name_end],
            start_byte: pos,
            end_byte: source.len(),
            is_static: false,
            is_macro_def: false,
        });
        offset = name_end;
    }
}

fn collect_java_methods<'a>(source: &'a str, clean: &str, out: &mut Vec<RawFunction<'a>>) {
    for (line_start, line) in lines_with_offsets(clean) {
        let Some(paren_rel) = line.find('(') else {
            continue;
        };
        let global_paren = line_start + paren_rel;
        if !has_body_after_signature(clean, global_paren) {
            continue;
        }
        let Some((name_start, name_end)) = previous_ident(line, paren_rel) else {
            continue;
        };
        let name = &line[name_start..name_end];
        if is_keyword(name) {
            continue;
        }
        out.push(RawFunction {
            name: &source[line_start + name_start..line_start + name_end],
            start_byte: line_start,
            end_byte: source.len(),
            is_static: line[..paren_rel].contains(" static "),
            is_macro_def: false,
        });
    }
}

fn collect_c_functions<'a>(source: &'a str, clean: &str, out: &mut Vec<RawFunction<'a>>) {
    for (line_start, line) in lines_with_offsets(clean) {
        let trimmed = line.trim_start();
        let leading_ws = line.len() - trimmed.len();
        if let Some(rest) = trimmed.strip_prefix("#define ") {
            if let Some(name_end) = read_ident_end(rest, 0) {
                out.push(RawFunction {
                    name: &source
                        [line_start + leading_ws + 8..line_start + leading_ws + 8 + name_end],
                    start_byte: line_start,
                    end_byte: line_start + line.len(),
                    is_static: false,
                    is_macro_def: true,
                });
            }
            continue;
        }
    }

    let bytes = clean.as_bytes();
    let mut cursor = 0;
    while cursor < bytes.len() {
        let Some(rel) = clean[cursor..].find('(') else {
            break;
        };
        let paren = cursor + rel;
        cursor = paren + 1;

        if !has_body_after_signature(clean, paren) {
            continue;
        }
        let Some((name_start, name_end)) = previous_ident(clean, paren) else {
            continue;
        };
        let name = &clean[name_start..name_end];
        if is_keyword(name) {
            continue;
        }
        if name_start > 0 && matches!(bytes[name_start - 1], b'.' | b'>' | b':') {
            continue;
        }
        let decl_start = declaration_start(clean, name_start);
        let prefix = clean[decl_start..name_start].trim();
        if prefix.is_empty()
            || prefix.ends_with('=')
            || prefix.ends_with(',')
            || prefix.ends_with("return")
            || prefix.split_whitespace().any(is_control_keyword)
        {
            continue;
        }

        out.push(RawFunction {
            name: &source[name_start..name_end],
            start_byte: decl_start,
            end_byte: source.len(),
            is_static: prefix.split_whitespace().any(|part| part == "static"),
            is_macro_def: false,
        });
    }
}

fn finalize_function_ranges(
    source: &str,
    clean: &str,
    lang: Lang,
    functions: &mut [RawFunction<'_>],
) {
    for i in 0..functions.len() {
        if functions[i].is_macro_def {
            continue;
        }
        let next_start = functions
            .get(i + 1)
            .map(|f| f.start_byte)
            .unwrap_or(source.len());
        functions[i].end_byte = match lang {
            Lang::Python => next_start,
            _ => clean[functions[i].start_byte..next_start]
                .find('{')
                .and_then(|rel| find_matching(clean, functions[i].start_byte + rel, b'{', b'}'))
                .map(|end| end + 1)
                .unwrap_or(next_start),
        };
    }
}

fn collect_calls<'a>(source: &'a str, clean: &str, lang: Lang) -> Vec<RawCall<'a>> {
    let bytes = clean.as_bytes();
    let mut calls = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if !is_ident_start(bytes[i]) {
            i += 1;
            continue;
        }
        let name_start = i;
        i += 1;
        while i < bytes.len() && is_ident_byte(bytes[i]) {
            i += 1;
        }
        let name_end = i;
        let name = call_leaf_name(&clean[name_start..name_end]);
        let paren = skip_ws(clean, name_end);
        if bytes.get(paren) != Some(&b'(') || is_keyword(name) {
            continue;
        }
        if is_declaration_context(clean, lang, name_start) {
            continue;
        }
        let kind = if lang == Lang::C && is_likely_macro(name) {
            CallKind::Macro
        } else {
            CallKind::Direct
        };
        calls.push(RawCall {
            callee_name: borrow_call_leaf(source, name_start, name_end),
            start_byte: name_start,
            end_byte: find_matching(clean, paren, b'(', b')')
                .map(|p| p + 1)
                .unwrap_or(paren + 1),
            kind,
        });
    }
    calls
}

fn is_declaration_context(clean: &str, lang: Lang, name_start: usize) -> bool {
    let prefix = &clean[..name_start];
    let tail = prefix
        .rsplit_once(|c: char| ['\n', ';', '{', '}'].contains(&c))
        .map(|(_, tail)| tail)
        .unwrap_or(prefix)
        .trim_end();
    match lang {
        Lang::Rust => tail.ends_with("fn") || tail.ends_with("async fn"),
        Lang::TypeScript | Lang::JavaScript => {
            tail.ends_with("function") || tail.ends_with("if") || tail.ends_with("for")
        }
        Lang::Python => tail.ends_with("def") || tail.ends_with("class"),
        Lang::Go => tail.ends_with("func"),
        Lang::Java | Lang::C => {
            tail.ends_with("if")
                || tail.ends_with("for")
                || tail.ends_with("while")
                || tail.ends_with("switch")
                || tail.contains("return")
        }
    }
}

fn mask_comments_and_strings(source: &str, lang: Lang) -> String {
    let mut out = source.as_bytes().to_vec();
    let bytes = source.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'"' || bytes[i] == b'\'' || bytes[i] == b'`' {
            let quote = bytes[i];
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\n' && quote != b'`' {
                    break;
                }
                out[i] = mask_byte(bytes[i]);
                if bytes[i] == b'\\' {
                    i += 2;
                    continue;
                }
                if bytes[i] == quote {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }

        if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'/') && lang != Lang::Python {
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                out[i] = b' ';
                i += 1;
            }
            continue;
        }
        if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'*') && lang != Lang::Python {
            i += 2;
            while i + 1 < bytes.len() {
                out[i] = mask_byte(bytes[i]);
                if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    out[i + 1] = b' ';
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }
        if bytes[i] == b'#' && lang == Lang::Python {
            i += 1;
            while i < bytes.len() && bytes[i] != b'\n' {
                out[i] = b' ';
                i += 1;
            }
            continue;
        }
        i += 1;
    }
    String::from_utf8(out).unwrap_or_default()
}

fn mask_byte(byte: u8) -> u8 {
    if byte == b'\n' { b'\n' } else { b' ' }
}

fn skip_ws(input: &str, mut pos: usize) -> usize {
    let bytes = input.as_bytes();
    while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
        pos += 1;
    }
    pos
}

fn read_ident_end(input: &str, start: usize) -> Option<usize> {
    let bytes = input.as_bytes();
    if !bytes.get(start).copied().is_some_and(is_ident_start) {
        return None;
    }
    let mut end = start + 1;
    while end < bytes.len() && is_ident_byte(bytes[end]) {
        end += 1;
    }
    Some(end)
}

fn previous_ident(input: &str, before: usize) -> Option<(usize, usize)> {
    let bytes = input.as_bytes();
    let mut end = before;
    while end > 0 && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    let mut start = end;
    while start > 0 && is_ident_byte(bytes[start - 1]) {
        start -= 1;
    }
    if start < end && is_ident_start(bytes[start]) {
        Some((start, end))
    } else {
        None
    }
}

fn has_body_after_signature(clean: &str, paren: usize) -> bool {
    let Some(close) = find_matching(clean, paren, b'(', b')') else {
        return false;
    };
    for byte in clean.as_bytes().iter().skip(close + 1) {
        match *byte {
            b'{' => return true,
            b';' => return false,
            b'-' | b'>' | b'[' | b']' | b'_' | b':' | b',' | b'<' | b' ' | b'\t' | b'\n'
            | b'\r' => {}
            byte if byte.is_ascii_alphanumeric() => {}
            _ => {}
        }
    }
    false
}

fn declaration_start(clean: &str, before: usize) -> usize {
    clean[..before]
        .rfind(|c: char| [';', '{', '}'].contains(&c))
        .map(|idx| idx + 1)
        .unwrap_or(0)
}

fn find_matching(input: &str, open_pos: usize, open: u8, close: u8) -> Option<usize> {
    let bytes = input.as_bytes();
    if bytes.get(open_pos) != Some(&open) {
        return None;
    }
    let mut depth = 0usize;
    for (idx, byte) in bytes.iter().enumerate().skip(open_pos) {
        if *byte == open {
            depth += 1;
        } else if *byte == close {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(idx);
            }
        }
    }
    None
}

fn lines_with_offsets(input: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut offset = 0usize;
    input.lines().map(move |line| {
        let start = offset;
        offset += line.len() + 1;
        (start, line)
    })
}

fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_ident_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn is_call_path_byte(byte: u8) -> bool {
    is_ident_byte(byte) || byte == b'.' || byte == b':'
}

fn call_leaf_name(name: &str) -> &str {
    name.rsplit(['.', ':'])
        .find(|part| !part.is_empty())
        .unwrap_or(name)
}

fn borrow_call_leaf<'a>(source: &'a str, start: usize, end: usize) -> Cow<'a, str> {
    let text = &source[start..end];
    match text.rfind(['.', ':']) {
        Some(pos) => Cow::Borrowed(&text[pos + 1..]),
        None => Cow::Borrowed(text),
    }
}

fn is_keyword(name: &str) -> bool {
    matches!(
        name,
        "if" | "for"
            | "while"
            | "switch"
            | "match"
            | "loop"
            | "return"
            | "sizeof"
            | "typeof"
            | "function"
            | "fn"
            | "def"
            | "func"
            | "class"
            | "struct"
            | "enum"
            | "trait"
            | "impl"
            | "new"
            | "catch"
    )
}

fn is_control_keyword(name: &str) -> bool {
    matches!(
        name,
        "if" | "for" | "while" | "switch" | "return" | "sizeof"
    )
}

fn is_likely_macro(name: &str) -> bool {
    name.len() > 1
        && name.chars().all(|c| c.is_ascii_uppercase() || c == '_')
        && !name.starts_with('_')
}

/// For each function, find calls that fall within its byte range and create
/// `(caller_name, RawCall)` pairs.
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
                if matches!(
                    ext,
                    "rs" | "ts" | "tsx" | "c" | "h" | "py" | "go" | "java" | "js" | "jsx" | "mjs"
                ) {
                    files.push(path.to_string_lossy().to_string());
                }
            }
        }
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;

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
    pub async fn bar(&self) {
        self.baz();
        crate::net::send();
    }
}
"#;
        let result = parse_file("test.rs", source).expect("parse failed");
        assert_eq!(result.functions.len(), 1);
        assert_eq!(result.functions[0].name, "bar");
        assert!(result.calls.iter().any(|call| call.callee_name == "baz"));
        assert!(result.calls.iter().any(|call| call.callee_name == "send"));
    }

    #[test]
    #[cfg(feature = "lang-c")]
    fn test_parse_c_simple_function() {
        let source = "void foo() { bar(); }";
        let result = parse_file("test.c", source).expect("parse failed");
        assert_eq!(result.functions.len(), 1);
        assert_eq!(result.functions[0].name, "foo");
        assert!(!result.functions[0].is_static);
    }

    #[test]
    #[cfg(feature = "lang-c")]
    fn test_parse_c_macro_definition() {
        let source = "#define FOO(x) ((x) + 1)\nvoid bar() { int y = FOO(5); }";
        let result = parse_file("test.c", source).expect("parse failed");
        assert!(result.functions.iter().any(|f| f.is_macro_def));
        assert!(result.calls.iter().any(|c| c.kind == CallKind::Macro));
    }

    #[test]
    #[cfg(feature = "lang-c")]
    fn test_parse_c_prototype_and_multiline_definition() {
        let source = r#"
int helper(void);

static int
worker(void)
{
    return helper();
}
"#;
        let result = parse_file("test.c", source).expect("parse failed");
        assert_eq!(
            result.functions.iter().filter(|f| !f.is_macro_def).count(),
            1
        );
        assert_eq!(
            result
                .functions
                .iter()
                .find(|f| !f.is_macro_def)
                .unwrap()
                .name,
            "worker"
        );
        assert!(
            result
                .functions
                .iter()
                .find(|f| f.name == "worker")
                .unwrap()
                .is_static
        );
        assert!(result.calls.iter().any(|call| call.callee_name == "helper"));
    }

    #[test]
    #[cfg(feature = "lang-typescript")]
    fn test_parse_typescript_function() {
        let source = "function greet(name: string): void { console.log(name); }\nconst arrow = () => service.run();";
        let result = parse_file("test.ts", source).expect("parse failed");
        assert_eq!(result.functions.len(), 2);
        assert!(result.calls.iter().any(|call| call.callee_name == "log"));
        assert!(result.calls.iter().any(|call| call.callee_name == "run"));
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
    }
}
