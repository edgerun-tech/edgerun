//! Codebase indexer — writes rkyv-normalized cache data for source analysis.
//! Supports direct file-backed indexing and simple query mode.
//!
//! Usage:
//!   edgerun-codelyzer-index <path/to/codebase>
//!   edgerun-codelyzer-index <path/to/codebase> --refresh  # rebuild index
//!   edgerun-codelyzer-index --query stats                # inspect cache

#![allow(clippy::unwrap_used)]

use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    time::Instant,
};

use edgerun_codelyzer::{
    parser::{ParserPool, RawCallOwned, RawFunctionOwned},
    uir::CallKind,
};
use edgerun_crypto::sha1::{Digest, Sha1};
use rayon::prelude::*;

const INDEX_MAGIC: &[u8; 8] = b"EDGIDX01";
const INDEX_VERSION: u32 = 1;
const NO_STRING: u32 = u32::MAX;
const NO_LINE: u32 = u32::MAX;

const EDGE_KIND_DIRECT: u8 = 0;
const EDGE_KIND_INDIRECT: u8 = 1;
const EDGE_KIND_MACRO: u8 = 2;
const EDGE_KIND_UNKNOWN: u8 = 3;

#[derive(Clone, Default)]
struct StringInterner {
    map: HashMap<String, u32>,
    values: Vec<String>,
}

impl StringInterner {
    fn intern(&mut self, value: &str) -> u32 {
        if let Some(idx) = self.map.get(value) {
            return *idx;
        }
        let idx = self.values.len() as u32;
        self.values.push(value.to_string());
        self.map.insert(value.to_string(), idx);
        idx
    }
}

#[derive(Clone, Debug)]
struct ParsedFile {
    path: String,
    content: String,
    content_hash: String,
    line_count: u32,
    language: String,
    parse_error: Option<String>,
    functions: Vec<RawFunctionOwned>,
    calls: Vec<RawCallOwned>,
}

#[derive(Clone, Debug)]
struct FileRecord {
    path: String,
    content_hash: String,
    language: String,
    line_count: u32,
    function_count: u32,
    last_modified: u64,
    parse_error: Option<String>,
}

#[derive(Clone, Debug)]
struct FunctionRecord {
    id: String,
    name: String,
    file: String,
    language: String,
    start_line: u32,
    end_line: u32,
    start_byte: u32,
    end_byte: u32,
    signature: String,
    is_static: bool,
    kind: u8,
}

#[derive(Clone, Debug)]
struct EdgeRecord {
    caller: String,
    callee: String,
    kind: u8,
    line: Option<u32>,
}

#[derive(Clone, Debug)]
struct DiskIndex {
    generated_at: u64,
    root_path: String,
    files: Vec<FileRecord>,
    functions: Vec<FunctionRecord>,
    edges: Vec<EdgeRecord>,
}

// ─── File discovery ──────────────────────────────────────────────────

const SOURCE_EXTENSIONS: &[&str] = &[
    "c", "h", "rs", "ts", "tsx", "js", "jsx", "mjs", "py", "go", "java",
];

fn find_source_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let skip_dirs = &[
        "node_modules",
        ".git",
        "target",
        "dist",
        "build",
        "__pycache__",
        ".venv",
    ];

    fn walk(dir: &Path, files: &mut Vec<PathBuf>, skip: &[&str]) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if skip.contains(&name) {
                            continue;
                        }
                    }
                    walk(&path, files, skip);
                } else if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if SOURCE_EXTENSIONS.contains(&ext) {
                            files.push(path);
                        }
                    }
                }
            }
        }
    }

    walk(root, &mut files, skip_dirs);
    files.sort();
    files
}

fn lang_for_path(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => "rust",
        Some("ts") | Some("tsx") => "typescript",
        Some("js") | Some("jsx") | Some("mjs") => "javascript",
        Some("c") | Some("h") => "c",
        Some("py") => "python",
        Some("go") => "go",
        Some("java") => "java",
        _ => "unknown",
    }
}

fn parse_file(root: &Path, path: &Path) -> Option<ParsedFile> {
    let content = fs::read_to_string(path).ok()?;
    let mut hasher = Sha1::new();
    hasher.update(content.as_bytes());
    let content_hash = format!("{:x}", hasher.finalize());

    let rel_path = path
        .strip_prefix(root)
        .ok()?
        .to_string_lossy()
        .to_string();
    let language = lang_for_path(path).to_string();
    let line_count = content.lines().count() as u32;

    let mut pool = ParserPool::new();
    let parsed = match pool.parse_file(&rel_path, &content) {
        Some(result) => result.to_owned(),
        None => {
            return Some(ParsedFile {
                path: rel_path,
                content,
                content_hash,
                line_count,
                language,
                parse_error: Some("no parser for extension".to_string()),
                functions: Vec::new(),
                calls: Vec::new(),
            });
        }
    };

    Some(ParsedFile {
        path: rel_path,
        content,
        content_hash,
        line_count,
        language,
        parse_error: None,
        functions: parsed.functions,
        calls: parsed.calls,
    })
}

fn build_call_pairs(
    content: &str,
    functions: &[RawFunctionOwned],
    calls: &[RawCallOwned],
) -> Vec<(String, String, u8, Option<u32>)> {
    let func_ranges: Vec<(usize, usize)> = functions
        .iter()
        .map(|f| (f.start_byte, f.end_byte))
        .collect();

    let mut result = Vec::new();
    for call in calls {
        let containing_func = func_ranges
            .iter()
            .position(|&(start, end)| call.start_byte >= start && call.end_byte <= end);

        if let Some(idx) = containing_func {
            let caller = &functions[idx].name;
            let kind = match call.kind {
                CallKind::Direct => EDGE_KIND_DIRECT,
                CallKind::Indirect => EDGE_KIND_INDIRECT,
                CallKind::Macro => EDGE_KIND_MACRO,
                CallKind::Unknown => EDGE_KIND_UNKNOWN,
            };
            result.push((
                caller.clone(),
                call.callee_name.clone(),
                kind,
                line_at_byte(content, call.start_byte),
            ));
        }
    }

    result
}

fn signature_from_function(content: &str, function: &RawFunctionOwned) -> String {
    let start = function.start_byte.min(content.len());
    let end = function.end_byte.min(content.len());
    let first_line_end = content[start..end].find('\n').map(|offset| start + offset).unwrap_or(end);
    content[start..first_line_end].trim().to_string()
}

fn line_at_byte(content: &str, byte_offset: usize) -> Option<u32> {
    if byte_offset > content.len() {
        return None;
    }
    let prefix = &content[..byte_offset];
    Some(prefix.lines().count() as u32 + 1)
}

fn write_u8(output: &mut File, value: u8) -> io::Result<()> {
    output.write_all(&[value])
}

fn write_u32(output: &mut File, value: u32) -> io::Result<()> {
    output.write_all(&value.to_le_bytes())
}

fn write_u64(output: &mut File, value: u64) -> io::Result<()> {
    output.write_all(&value.to_le_bytes())
}

fn write_str(output: &mut File, value: &str) -> io::Result<()> {
    write_u32(output, value.len() as u32)?;
    output.write_all(value.as_bytes())
}

fn read_u8(input: &mut File) -> io::Result<u8> {
    let mut buf = [0u8; 1];
    input.read_exact(&mut buf)?;
    Ok(buf[0])
}

fn read_u32(input: &mut File) -> io::Result<u32> {
    let mut buf = [0u8; 4];
    input.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_u64(input: &mut File) -> io::Result<u64> {
    let mut buf = [0u8; 8];
    input.read_exact(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

fn read_str(input: &mut File) -> io::Result<String> {
    let len = read_u32(input)? as usize;
    let mut buf = vec![0u8; len];
    input.read_exact(&mut buf)?;
    Ok(String::from_utf8_lossy(&buf).to_string())
}

fn write_disk_index(path: &Path, root_path: &Path, generated_at: u64, index: &DiskIndex) -> io::Result<()> {
    let mut interner = StringInterner::default();

    let root_path_index = interner.intern(&root_path.to_string_lossy());
    for file in &index.files {
        interner.intern(&file.path);
        interner.intern(&file.content_hash);
        interner.intern(&file.language);
        if let Some(error) = &file.parse_error {
            interner.intern(error);
        }
    }
    for function in &index.functions {
        interner.intern(&function.id);
        interner.intern(&function.name);
        interner.intern(&function.file);
        interner.intern(&function.language);
        interner.intern(&function.signature);
    }
    for edge in &index.edges {
        interner.intern(&edge.caller);
        interner.intern(&edge.callee);
    }

    // We keep root path interned with the table for consistency.
    let _ = root_path_index;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }

    let mut output = File::create(path)?;
    output.write_all(INDEX_MAGIC)?;
    write_u32(&mut output, INDEX_VERSION)?;
    write_u64(&mut output, generated_at)?;
    write_str(&mut output, &interner.values[root_path_index as usize])?;

    write_u32(&mut output, interner.values.len() as u32)?;
    for value in &interner.values {
        write_str(&mut output, value)?;
    }

    write_u32(&mut output, index.files.len() as u32)?;
    for file in &index.files {
        let path_idx = interner.map[&file.path];
        let hash_idx = interner.map[&file.content_hash];
        let lang_idx = interner.map[&file.language];
        let error_idx = file.parse_error.as_ref().map_or(NO_STRING, |error| interner.map[error]);

        write_u32(&mut output, path_idx)?;
        write_u32(&mut output, hash_idx)?;
        write_u32(&mut output, lang_idx)?;
        write_u32(&mut output, file.line_count)?;
        write_u32(&mut output, file.function_count)?;
        write_u64(&mut output, file.last_modified)?;
        write_u32(&mut output, error_idx)?;
    }

    write_u32(&mut output, index.functions.len() as u32)?;
    for function in &index.functions {
        let id_idx = interner.map[&function.id];
        let name_idx = interner.map[&function.name];
        let file_idx = interner.map[&function.file];
        let lang_idx = interner.map[&function.language];
        let signature_idx = interner.map[&function.signature];
        write_u32(&mut output, id_idx)?;
        write_u32(&mut output, name_idx)?;
        write_u32(&mut output, file_idx)?;
        write_u32(&mut output, lang_idx)?;
        write_u32(&mut output, function.start_line)?;
        write_u32(&mut output, function.end_line)?;
        write_u32(&mut output, function.start_byte)?;
        write_u32(&mut output, function.end_byte)?;
        write_u32(&mut output, signature_idx)?;
        write_u8(&mut output, u8::from(function.is_static))?;
        write_u8(&mut output, function.kind)?;
    }

    write_u32(&mut output, index.edges.len() as u32)?;
    for edge in &index.edges {
        let caller_idx = interner.map[&edge.caller];
        let callee_idx = interner.map[&edge.callee];
        let line = edge.line.unwrap_or(NO_LINE);
        write_u32(&mut output, caller_idx)?;
        write_u32(&mut output, callee_idx)?;
        write_u8(&mut output, edge.kind)?;
        write_u32(&mut output, line)?;
    }
    Ok(())
}

fn load_disk_index(path: &Path) -> io::Result<DiskIndex> {
    let mut input = File::open(path)?;
    let mut magic = [0u8; 8];
    input.read_exact(&mut magic)?;
    if &magic != INDEX_MAGIC {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "not a valid edgerun codelyzer index",
        ));
    }

    let version = read_u32(&mut input)?;
    if version != INDEX_VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unsupported index version",
        ));
    }

    let generated_at = read_u64(&mut input)?;
    let root_path = read_str(&mut input)?;

    let strings_count = read_u32(&mut input)? as usize;
    let mut strings = Vec::with_capacity(strings_count);
    for _ in 0..strings_count {
        strings.push(read_str(&mut input)?);
    }

    let file_count = read_u32(&mut input)? as usize;
    let mut files = Vec::with_capacity(file_count);
    for _ in 0..file_count {
        let path_idx = read_u32(&mut input)? as usize;
        let hash_idx = read_u32(&mut input)? as usize;
        let lang_idx = read_u32(&mut input)? as usize;
        let line_count = read_u32(&mut input)?;
        let function_count = read_u32(&mut input)?;
        let last_modified = read_u64(&mut input)?;
        let error_idx = read_u32(&mut input)?;
        files.push(FileRecord {
            path: strings.get(path_idx).cloned().unwrap_or_default(),
            content_hash: strings.get(hash_idx).cloned().unwrap_or_default(),
            language: strings.get(lang_idx).cloned().unwrap_or_default(),
            line_count,
            function_count,
            last_modified,
            parse_error: if error_idx == NO_STRING {
                None
            } else {
                strings.get(error_idx as usize).cloned()
            },
        });
    }

    let function_count = read_u32(&mut input)? as usize;
    let mut functions = Vec::with_capacity(function_count);
    for _ in 0..function_count {
        let id = read_u32(&mut input)? as usize;
        let name = read_u32(&mut input)? as usize;
        let file = read_u32(&mut input)? as usize;
        let lang = read_u32(&mut input)? as usize;
        let start_line = read_u32(&mut input)?;
        let end_line = read_u32(&mut input)?;
        let start_byte = read_u32(&mut input)?;
        let end_byte = read_u32(&mut input)?;
        let signature = read_u32(&mut input)? as usize;
        let is_static = read_u8(&mut input)?;
        let kind = read_u8(&mut input)?;
        functions.push(FunctionRecord {
            id: strings.get(id).cloned().unwrap_or_default(),
            name: strings.get(name).cloned().unwrap_or_default(),
            file: strings.get(file).cloned().unwrap_or_default(),
            language: strings.get(lang).cloned().unwrap_or_default(),
            start_line,
            end_line,
            start_byte,
            end_byte,
            signature: strings.get(signature).cloned().unwrap_or_default(),
            is_static: is_static != 0,
            kind,
        });
    }

    let edge_count = read_u32(&mut input)? as usize;
    let mut edges = Vec::with_capacity(edge_count);
    for _ in 0..edge_count {
        let caller = strings
            .get(read_u32(&mut input)? as usize)
            .cloned()
            .unwrap_or_default();
        let callee = strings
            .get(read_u32(&mut input)? as usize)
            .cloned()
            .unwrap_or_default();
        let kind = read_u8(&mut input)?;
        let line_raw = read_u32(&mut input)?;
        edges.push(EdgeRecord {
            caller,
            callee,
            kind,
            line: if line_raw == NO_LINE { None } else { Some(line_raw) },
        });
    }

    Ok(DiskIndex {
        generated_at,
        root_path,
        files,
        functions,
        edges,
    })
}

fn build_index_records(root: &Path, files: Vec<ParsedFile>) -> DiskIndex {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut file_records = Vec::new();
    let mut function_records = Vec::new();
    let mut edge_records = Vec::new();

    let mut local_lookup: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut name_to_ids: HashMap<String, Vec<String>> = HashMap::new();

    for parsed in &files {
        let abs_path = root.join(&parsed.path);
        let last_modified = fs::metadata(&abs_path)
            .ok()
            .and_then(|meta| meta.modified().ok())
            .and_then(|mtime| {
                mtime
                    .duration_since(std::time::UNIX_EPOCH)
                    .ok()
                    .map(|dur| dur.as_secs())
            })
            .unwrap_or_default();

        file_records.push(FileRecord {
            path: parsed.path.clone(),
            content_hash: parsed.content_hash.clone(),
            language: parsed.language.clone(),
            line_count: parsed.line_count,
            function_count: parsed.functions.len() as u32,
            last_modified,
            parse_error: parsed.parse_error.clone(),
        });

        for function in &parsed.functions {
            let start_line = line_at_byte(&parsed.content, function.start_byte).unwrap_or(0);
            let end_line = line_at_byte(&parsed.content, function.end_byte).unwrap_or(start_line);
            let signature = signature_from_function(&parsed.content, function);
            let fid = format!("{}::{}", parsed.path, function.name);
            let kind = EDGE_KIND_UNKNOWN;
            function_records.push(FunctionRecord {
                id: fid.clone(),
                name: function.name.clone(),
                file: parsed.path.clone(),
                language: parsed.language.clone(),
                start_line,
                end_line,
                start_byte: function.start_byte as u32,
                end_byte: function.end_byte as u32,
                signature,
                is_static: function.is_static,
                kind,
            });
            local_lookup
                .entry(parsed.path.clone())
                .or_default()
                .insert(function.name.clone(), fid.clone());
            name_to_ids
                .entry(function.name.clone())
                .or_default()
                .push(fid);
        }
    }

    for parsed in &files {
        for (caller_name, callee_name, kind, call_line) in
            build_call_pairs(&parsed.content, &parsed.functions, &parsed.calls)
        {
            let caller = local_lookup
                .get(&parsed.path)
                .and_then(|file_map| file_map.get(&caller_name))
                .cloned()
                .unwrap_or_else(|| format!("{}::{}", parsed.path, caller_name));

            let callee = local_lookup
                .get(&parsed.path)
                .and_then(|file_map| file_map.get(&callee_name))
                .cloned()
                .or_else(|| {
                    name_to_ids
                        .get(&callee_name)
                        .and_then(|ids| ids.first())
                        .cloned()
                })
                .unwrap_or_else(|| callee_name.clone());

            let line = call_line.or_else(|| {
                parsed
                    .functions
                    .iter()
                    .find(|function| function.name == caller_name)
                    .and_then(|function| line_at_byte(&parsed.content, function.start_byte))
            });

            edge_records.push(EdgeRecord {
                caller,
                callee,
                kind,
                line,
            });
        }
    }

    DiskIndex {
        generated_at: now,
        root_path: root.to_string_lossy().to_string(),
        files: file_records,
        functions: function_records,
        edges: edge_records,
    }
}

fn index_codebase(root: &Path, index_path: &Path, refresh: bool) {
    let start = Instant::now();

    if refresh {
        println!("[index] Refresh flag set — rebuilding full index");
    }

    println!("[index] Scanning for source files...");
    let source_files = find_source_files(root);
    println!("[index] Found {} source files", source_files.len());

    println!("[index] Parsing files in parallel...");
    let parse_start = Instant::now();
    let parse_results: Vec<ParsedFile> = source_files
        .par_iter()
        .filter_map(|path| parse_file(root, path))
        .collect();
    println!(
        "[index] Parsed {} files in {:.2}s",
        parse_results.len(),
        parse_start.elapsed().as_secs_f32()
    );

    let index = build_index_records(root, parse_results);

    println!(
        "[index] Writing compact cache to {}",
        index_path.display()
    );
    let write_start = Instant::now();
    if let Err(err) = write_disk_index(index_path, root, index.generated_at, &index) {
        eprintln!("[index] Failed to write index: {err}");
        std::process::exit(1);
    }

    println!(
        "[index] Wrote {} files, {} functions, {} edges in {:.2}s",
        index.files.len(),
        index.functions.len(),
        index.edges.len(),
        write_start.elapsed().as_secs_f32()
    );
    println!(
        "[index] Total indexing time: {:.2}s",
        start.elapsed().as_secs_f32()
    );
}

fn id_matches_query(target: &str, query: &str) -> bool {
    target == query || target.ends_with(&format!("::{}", query))
}

fn query_index(index_path: &Path, args: &[String]) {
    if !index_path.exists() {
        eprintln!("[query] Index not found at {:?}", index_path);
        std::process::exit(1);
    }

    let index = match load_disk_index(index_path) {
        Ok(index) => index,
        Err(err) => {
            eprintln!("[query] Failed to read index: {err}");
            std::process::exit(1);
        }
    };

    let mut function_by_id = HashMap::new();
    for function in &index.functions {
        function_by_id.insert(function.id.clone(), function.clone());
    }

    if args.is_empty() {
        let parse_error_count = index.files.iter().filter(|f| f.parse_error.is_some()).count();
        let file_count = index.files.len();
        let function_count = index.functions.len();
        let lang_count = index
            .functions
            .iter()
            .map(|f| f.language.clone())
            .collect::<HashSet<_>>()
            .len();
        println!("Indexed codebase:");
        println!("  root_path: {}", index.root_path);
        println!("  files: {}", file_count);
        println!("  functions: {}", function_count);
        println!("  edges: {}", index.edges.len());
        println!("  languages: {}", lang_count);
        println!("  parse_errors: {}", parse_error_count);
        println!("  generated_at: {}", index.generated_at);
        return;
    }

    let query = args[0].as_str();
    match query {
        "find" | "symbol" => {
            let Some(pattern) = args.get(1) else {
                eprintln!("Usage: find <pattern> [limit]");
                std::process::exit(1);
            };
            let pattern = pattern.as_str();
            let limit = args
                .get(2)
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(20);

            let matches: Vec<_> = index
                .functions
                .iter()
                .filter(|function| function.name.contains(pattern))
                .take(limit)
                .collect();

            println!(
                "Found {} symbol(s) matching \"{}\":\n",
                matches.len(),
                pattern
            );
            for function in matches {
                let static_marker = if function.is_static { ", static" } else { "" };
                println!(
                    "  {} ({}){} @ {}:{}-{}",
                    function.name,
                    function.language,
                    static_marker,
                    function.file,
                    function.start_line,
                    function.end_line
                );
                println!("    {}", function.signature);
            }
        }

        "callers" => {
            let Some(symbol) = args.get(1) else {
                eprintln!("Usage: callers <name>");
                std::process::exit(1);
            };
            let symbol = symbol.as_str();
            let mut seen = HashSet::new();
            let mut entries = Vec::new();
            for edge in &index.edges {
                if id_matches_query(&edge.callee, symbol) && seen.insert(edge.caller.clone()) {
                    if let Some(caller) = function_by_id.get(&edge.caller) {
                        entries.push((caller.clone(), edge.line));
                    } else {
                        entries.push((FunctionRecord {
                            id: edge.caller.clone(),
                            name: edge.caller.clone(),
                            file: "".to_string(),
                            language: "unknown".to_string(),
                            start_line: 0,
                            end_line: 0,
                            start_byte: 0,
                            end_byte: 0,
                            signature: "".to_string(),
                            is_static: false,
                            kind: EDGE_KIND_UNKNOWN,
                        }, edge.line));
                    }
                }
            }
            println!("Callers of \"{symbol}\":\n");
            if entries.is_empty() {
                println!("  (none found)");
                return;
            }
            for (caller, call_line) in entries {
                match call_line {
                    Some(line) => println!(
                        "  {} ({}) @ {}:{} (call site: line {})",
                        caller.name, caller.file, caller.start_line, caller.end_line, line
                    ),
                    None => println!(
                        "  {} ({}) @ {}:{}",
                        caller.name, caller.file, caller.start_line, caller.end_line
                    ),
                }
                if !caller.signature.is_empty() {
                    println!("    {}", caller.signature);
                }
            }
        }
        "callees" => {
            let Some(symbol) = args.get(1) else {
                eprintln!("Usage: callees <name>");
                std::process::exit(1);
            };
            let symbol = symbol.as_str();
            let mut seen = HashSet::new();
            let mut entries = Vec::new();
            for edge in &index.edges {
                if id_matches_query(&edge.caller, symbol) && seen.insert(edge.callee.clone()) {
                    if let Some(callee) = function_by_id.get(&edge.callee) {
                        entries.push((callee.clone(), edge.line));
                    } else {
                        entries.push((FunctionRecord {
                            id: edge.callee.clone(),
                            name: edge.callee.clone(),
                            file: "".to_string(),
                            language: "unknown".to_string(),
                            start_line: 0,
                            end_line: 0,
                            start_byte: 0,
                            end_byte: 0,
                            signature: "".to_string(),
                            is_static: false,
                            kind: EDGE_KIND_UNKNOWN,
                        }, edge.line));
                    }
                }
            }
            println!("Functions called by \"{symbol}\":\n");
            if entries.is_empty() {
                println!("  (none found)");
                return;
            }
            for (callee, call_line) in entries {
                match call_line {
                    Some(line) => println!(
                        "  {} ({}) @ {}:{} (call site: line {})",
                        callee.name, callee.file, callee.start_line, callee.end_line, line
                    ),
                    None => println!(
                        "  {} ({}) @ {}:{}",
                        callee.name, callee.file, callee.start_line, callee.end_line
                    ),
                }
                if !callee.signature.is_empty() {
                    println!("    {}", callee.signature);
                }
            }
        }

        "stats" => {
            let lang_count = index
                .functions
                .iter()
                .map(|f| f.language.clone())
                .collect::<HashSet<_>>()
                .len();
            let file_count = index.files.len();
            let function_count = index.functions.len();
            let edge_count = index.edges.len();

            let mut by_caller = HashMap::new();
            for edge in &index.edges {
                *by_caller.entry(edge.caller.clone()).or_insert(0usize) += 1;
            }

            let mut top: Vec<_> = by_caller.into_iter().collect();
            top.sort_by_key(|(_, count)| *count);
            top.reverse();

            println!("Index statistics:");
            println!("  Files: {}", file_count);
            println!("  Functions: {}", function_count);
            println!("  Edges: {}", edge_count);
            println!("  Languages: {}", lang_count);
            println!("  Average edges/function: {:.1}", edge_count as f64 / function_count.max(1) as f64);
            println!("\nTop 10 most-connected functions:");
            for (id, count) in top.iter().take(10) {
                if let Some(function) = function_by_id.get(id) {
                    println!(
                        "  {} ({}) - {} edges",
                        function.name,
                        function.file,
                        count
                    );
                } else {
                    println!("  {} - {} edges", id, count);
                }
            }
            println!("\nEdge kinds:");
            let mut edge_kind_counts = HashMap::new();
            for edge in &index.edges {
                *edge_kind_counts.entry(edge.kind).or_insert(0u32) += 1;
            }
            for (kind, label) in [
                (EDGE_KIND_DIRECT, "direct"),
                (EDGE_KIND_INDIRECT, "indirect"),
                (EDGE_KIND_MACRO, "macro"),
                (EDGE_KIND_UNKNOWN, "unknown"),
            ] {
                if let Some(count) = edge_kind_counts.get(&kind) {
                    println!("  {label}: {count}", label = label, count = count);
                }
            }
        }

        _ => {
            eprintln!(
                "Unknown query: {query}. Available: find <pattern> [limit], callers <name>, callees <name>, stats"
            );
        }
    }
}

fn default_index_path_for_repo(root: &Path) -> PathBuf {
    let mut path = root.to_path_buf();
    path.push(".codeanalyzer");
    path.push("uirs.bin");
    path
}

fn default_query_path() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    cwd.join(".codeanalyzer").join("uirs.bin")
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage:");
        eprintln!("  edgerun-codelyzer-index <path> [--refresh] [--db <path>]");
        eprintln!("  edgerun-codelyzer-index --query [args...] [--db <path>]");
        eprintln!();
        eprintln!("Query commands:");
        eprintln!("  find <pattern> [limit]   Search symbols");
        eprintln!("  callers <name>            Find callers of a function");
        eprintln!("  callees <name>            Find callees of a function");
        eprintln!("  stats                    Index statistics");
        std::process::exit(1);
    }

    let mut query_mode = false;
    let mut query_args: Vec<String> = Vec::new();
    let mut refresh = false;
    let mut override_path: Option<PathBuf> = None;
    let mut codebase_path: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--query" | "-q" => {
                query_mode = true;
                query_args = args[i + 1..].to_vec();
                break;
            }
            "--refresh" | "-r" => refresh = true,
            "--db" => {
                i += 1;
                if let Some(path) = args.get(i) {
                    override_path = Some(PathBuf::from(path));
                }
            }
            _ if codebase_path.is_none() && !query_mode => {
                codebase_path = Some(args[i].clone());
            }
            _ => {}
        }
        i += 1;
    }

    if query_mode {
        let index_path = override_path.unwrap_or_else(default_query_path);
        query_index(&index_path, &query_args);
        return;
    }

    if let Some(path) = codebase_path {
        let root = Path::new(&path);
        if !root.is_dir() {
            eprintln!("Error: '{}' is not a directory", path);
            std::process::exit(1);
        }
        let index_path = override_path.unwrap_or_else(|| default_index_path_for_repo(root));
        index_codebase(root, &index_path, refresh);
        return;
    }

    eprintln!("Error: no codebase path specified");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_index_path(prefix: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        path.push(format!(
            "edgerun-codelyzer-index-{prefix}-{}-{}.bin",
            std::process::id(),
            stamp
        ));
        path
    }

    #[test]
    fn write_and_read_disk_index_round_trip() {
        let index_path = temp_index_path("round-trip");
        let root = Path::new("/tmp/repo-root");

        let index = DiskIndex {
            generated_at: 1_650_000_000,
            root_path: root.to_string_lossy().to_string(),
            files: vec![FileRecord {
                path: "src/main.rs".to_string(),
                content_hash: "abc123".to_string(),
                language: "rust".to_string(),
                line_count: 3,
                function_count: 2,
                last_modified: 1_234_567_890,
                parse_error: None,
            }],
            functions: vec![
                FunctionRecord {
                    id: "src/main.rs::handle".to_string(),
                    name: "handle".to_string(),
                    file: "src/main.rs".to_string(),
                    language: "rust".to_string(),
                    start_line: 1,
                    end_line: 3,
                    start_byte: 10,
                    end_byte: 120,
                    signature: "fn handle()".to_string(),
                    is_static: false,
                    kind: EDGE_KIND_DIRECT,
                },
                FunctionRecord {
                    id: "src/main.rs::init".to_string(),
                    name: "init".to_string(),
                    file: "src/main.rs".to_string(),
                    language: "rust".to_string(),
                    start_line: 5,
                    end_line: 9,
                    start_byte: 130,
                    end_byte: 220,
                    signature: "fn init()".to_string(),
                    is_static: true,
                    kind: EDGE_KIND_DIRECT,
                },
            ],
            edges: vec![EdgeRecord {
                caller: "src/main.rs::init".to_string(),
                callee: "src/main.rs::handle".to_string(),
                kind: EDGE_KIND_DIRECT,
                line: Some(7),
            }],
        };

        write_disk_index(&index_path, root, index.generated_at, &index).expect("write should succeed");
        let loaded = load_disk_index(&index_path).expect("load should succeed");

        assert_eq!(loaded.generated_at, index.generated_at);
        assert_eq!(loaded.root_path, index.root_path);
        assert_eq!(loaded.files.len(), 1);
        assert_eq!(loaded.functions.len(), 2);
        assert_eq!(loaded.edges.len(), 1);
        assert_eq!(loaded.files[0].path, index.files[0].path);
        assert_eq!(loaded.functions[0].id, index.functions[0].id);
        assert_eq!(loaded.functions[1].name, index.functions[1].name);
        assert_eq!(loaded.edges[0].line, index.edges[0].line);

        let _ = fs::remove_file(index_path);
    }

    #[test]
    fn load_disk_index_rejects_invalid_magic() {
        let index_path = temp_index_path("invalid-magic");
        let mut output = File::create(&index_path).expect("create temp index file");
        output
            .write_all(b"NOTINDEX")
            .expect("write bad magic");

        let error = load_disk_index(&index_path).expect_err("should reject invalid header");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        let _ = fs::remove_file(index_path);
    }

    #[test]
    fn build_call_pairs_match_query_id_pattern() {
        let index = build_index_records(
            Path::new("/tmp/repo"),
            vec![ParsedFile {
                path: "src/main.rs".to_string(),
                content: "fn alpha() {}\nfn beta() {\n    alpha();\n}\n".to_string(),
                content_hash: "hash".to_string(),
                line_count: 4,
                language: "rust".to_string(),
                parse_error: None,
                functions: vec![
                    RawFunctionOwned {
                        name: "alpha".to_string(),
                        start_byte: 0,
                        end_byte: 11,
                        is_static: false,
                        is_macro_def: false,
                    },
                    RawFunctionOwned {
                        name: "beta".to_string(),
                        start_byte: 13,
                        end_byte: 40,
                        is_static: false,
                        is_macro_def: false,
                    },
                ],
                calls: vec![RawCallOwned {
                    callee_name: "alpha".to_string(),
                    start_byte: 28,
                    end_byte: 33,
                    kind: CallKind::Direct,
                }],
            }],
        );

        assert_eq!(index.functions.len(), 2);
        assert_eq!(index.edges.len(), 1);
        assert_eq!(index.edges[0].caller, "src/main.rs::beta");
        assert_eq!(index.edges[0].callee, "src/main.rs::alpha");
        assert_eq!(index.edges[0].line, Some(4));
        assert!(id_matches_query("src/main.rs::alpha", "alpha"));
        assert!(id_matches_query("src/main.rs::alpha", "src/main.rs::alpha"));
        assert!(!id_matches_query("src/main.rs::alpha", "beta"));
    }
}
