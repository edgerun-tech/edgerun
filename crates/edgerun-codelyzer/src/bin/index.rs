//! Codebase indexer — builds a SQLite UIR database from source code.
//!
//! Usage:
//!   edgerun-codelyzer-index /path/to/codebase          # build index
//!   edgerun-codelyzer-index /path/to/codebase --refresh # incremental update
//!   edgerun-codelyzer-index --query "find schedule"     # query mode (future)

#![allow(clippy::unwrap_used)]

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use edgerun_codelyzer::{
    parser::{ParserPool, RawCallOwned, RawFunctionOwned},
    uir::CallKind,
};
use edgerun_crypto::sha1::{Digest, Sha1};
use rayon::prelude::*;
use rusqlite::{params, Connection, OpenFlags};

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

#[allow(dead_code)]
fn file_hash(path: &Path) -> String {
    let content = fs::read(path).unwrap_or_default();
    let mut hasher = Sha1::new();
    hasher.update(&content);
    format!("{:x}", hasher.finalize())
}

// ─── Database ────────────────────────────────────────────────────────

fn open_db(db_path: &Path) -> Connection {
    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
        | OpenFlags::SQLITE_OPEN_CREATE
        | OpenFlags::SQLITE_OPEN_FULL_MUTEX;
    let conn = Connection::open_with_flags(db_path, flags).expect("failed to open database");

    conn.pragma_update(None, "journal_mode", "WAL").ok();
    conn.pragma_update(None, "synchronous", "NORMAL").ok();
    conn.pragma_update(None, "cache_size", "-64000").ok(); // 64MB
    conn.pragma_update(None, "temp_store", "MEMORY").ok();
    conn.pragma_update(None, "mmap_size", "268435456").ok(); // 256MB

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS codebases (
            id INTEGER PRIMARY KEY,
            root_path TEXT UNIQUE NOT NULL,
            name TEXT NOT NULL,
            function_count INTEGER DEFAULT 0,
            file_count INTEGER DEFAULT 0,
            edge_count INTEGER DEFAULT 0,
            languages TEXT,
            last_indexed INTEGER,
            status TEXT DEFAULT 'idle'
        );

        CREATE TABLE IF NOT EXISTS functions (
            id INTEGER PRIMARY KEY,
            fid TEXT UNIQUE NOT NULL,
            name TEXT NOT NULL,
            file TEXT NOT NULL,
            language TEXT NOT NULL,
            start_line INTEGER NOT NULL,
            end_line INTEGER NOT NULL,
            start_byte INTEGER NOT NULL,
            end_byte INTEGER NOT NULL,
            signature TEXT NOT NULL,
            is_static INTEGER NOT NULL DEFAULT 0,
            kind TEXT NOT NULL DEFAULT 'function',
            parent TEXT,
            content_hash TEXT,
            indexed_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS edges (
            id INTEGER PRIMARY KEY,
            caller TEXT NOT NULL,
            callee TEXT NOT NULL,
            kind TEXT NOT NULL,
            confidence REAL NOT NULL DEFAULT 0,
            line INTEGER
        );

        CREATE TABLE IF NOT EXISTS files (
            path TEXT PRIMARY KEY,
            content_hash TEXT NOT NULL,
            language TEXT NOT NULL,
            line_count INTEGER NOT NULL,
            function_count INTEGER NOT NULL DEFAULT 0,
            last_modified INTEGER NOT NULL,
            indexed_at INTEGER NOT NULL,
            parse_errors TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_functions_name ON functions(name);
        CREATE INDEX IF NOT EXISTS idx_functions_file ON functions(file);
        CREATE INDEX IF NOT EXISTS idx_functions_language ON functions(language);
        CREATE INDEX IF NOT EXISTS idx_functions_fid ON functions(fid);
        CREATE INDEX IF NOT EXISTS idx_edges_caller ON edges(caller);
        CREATE INDEX IF NOT EXISTS idx_edges_callee ON edges(callee);
        ",
    )
    .expect("failed to create tables");

    conn
}

fn ensure_codebase(conn: &Connection, root_path: &str) -> i64 {
    let name = Path::new(root_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(root_path);

    conn.execute(
        "INSERT OR IGNORE INTO codebases (root_path, name) VALUES (?1, ?2)",
        params![root_path, name],
    )
    .ok();

    conn.query_row(
        "SELECT id FROM codebases WHERE root_path = ?1",
        params![root_path],
        |r| r.get::<_, i64>(0),
    )
    .expect("failed to get codebase id")
}

// ─── Indexing ────────────────────────────────────────────────────────

struct ParseResult {
    rel_path: String,
    content_hash: String,
    line_count: usize,
    language: String,
    functions: Vec<RawFunctionOwned>,
    calls: Vec<RawCallOwned>,
    parse_error: Option<String>,
}

fn parse_file(root: &Path, path: &Path) -> Option<ParseResult> {
    let content = fs::read_to_string(path).ok()?;
    let content_hash = {
        let mut h = Sha1::new();
        h.update(content.as_bytes());
        format!("{:x}", h.finalize())
    };

    let rel_path = path.strip_prefix(root).ok()?.to_str()?.to_string();
    let language = lang_for_path(path).to_string();
    let line_count = content.lines().count();

    let mut pool = ParserPool::new();
    let result = match pool.parse_file(&rel_path, &content) {
        Some(r) => r,
        None => {
            return Some(ParseResult {
                rel_path,
                content_hash,
                line_count,
                language,
                functions: Vec::new(),
                calls: Vec::new(),
                parse_error: Some("no parser for extension".to_string()),
            });
        }
    };

    let functions: Vec<RawFunctionOwned> = result.functions.iter().map(|f| f.into()).collect();
    let calls: Vec<RawCallOwned> = result.calls.iter().map(|c| c.into()).collect();

    Some(ParseResult {
        rel_path,
        content_hash,
        line_count,
        language,
        functions,
        calls,
        parse_error: None,
    })
}

fn build_call_pairs(
    functions: &[RawFunctionOwned],
    calls: &[RawCallOwned],
) -> Vec<(String, String, CallKind)> {
    let func_ranges: Vec<(usize, usize)> = functions
        .iter()
        .map(|f| (f.start_byte, f.end_byte))
        .collect();

    let mut result = Vec::new();
    for call in calls {
        // Find which function contains this call
        let containing_func = func_ranges
            .iter()
            .position(|&(start, end)| call.start_byte >= start && call.end_byte <= end);

        if let Some(idx) = containing_func {
            let caller = &functions[idx].name;
            result.push((caller.clone(), call.callee_name.clone(), call.kind));
        }
    }
    result
}

fn index_codebase(root: &Path, db_path: &Path, refresh: bool) {
    let start = Instant::now();

    println!("[index] Scanning for source files...");
    let files = find_source_files(root);
    println!("[index] Found {} source files", files.len());

    let mut conn = open_db(db_path);
    let _cb_id = ensure_codebase(&conn, root.to_str().unwrap());

    if refresh {
        println!("[index] Refresh mode — clearing old data for this codebase");
        conn.execute(
            "DELETE FROM files WHERE path LIKE ?1 || '%'",
            params![root.to_str().unwrap()],
        )
        .ok();
    }

    println!("[index] Parsing files in parallel...");
    let parse_start = Instant::now();

    let results: Vec<Option<ParseResult>> = files
        .par_iter()
        .map(|path| parse_file(root, path))
        .collect();

    let results: Vec<ParseResult> = results.into_iter().flatten().collect();
    println!(
        "[index] Parsed {} files in {:.2}s",
        results.len(),
        parse_start.elapsed().as_secs_f32()
    );

    println!("[index] Writing to database...");
    let db_start = Instant::now();

    let tx = conn.transaction().expect("failed to start transaction");

    // Clear old data for refreshed files
    if refresh {
        let root_str = root.to_str().unwrap();
        tx.execute(
            "DELETE FROM functions WHERE file LIKE ?1 || '%'",
            params![root_str],
        )
        .ok();
        tx.execute(
            "DELETE FROM edges WHERE caller LIKE ?1 || '%'",
            params![root_str],
        )
        .ok();
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let mut func_count = 0;
    let mut edge_count = 0;
    let mut file_count = 0;

    let mut file_name_to_id: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut name_to_ids: HashMap<String, Vec<String>> = HashMap::new();

    // Insert files and functions
    for result in &results {
        let abs_path = root.join(&result.rel_path).to_string_lossy().to_string();
        let content = fs::read_to_string(&abs_path).unwrap_or_default();

        tx.execute(
            "INSERT OR REPLACE INTO files (path, content_hash, language, line_count, \
             function_count, last_modified, indexed_at, parse_errors)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                abs_path,
                result.content_hash,
                result.language,
                result.line_count,
                result.functions.len(),
                fs::metadata(root.join(&result.rel_path))
                    .map(|m| m
                        .modified()
                        .unwrap()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64)
                    .unwrap_or(0),
                now,
                result.parse_error,
            ],
        )
        .ok();

        file_count += 1;

        for func in &result.functions {
            let fid = format!("{}::{}", result.rel_path, func.name);

            // Get first line as signature (reuse content read above)
            let sig_start = func.start_byte;
            let sig_end = content[sig_start..]
                .find('\n')
                .map(|n| sig_start + n)
                .unwrap_or(func.end_byte.min(content.len()));
            let signature = content[sig_start..sig_end.min(content.len())]
                .trim()
                .to_string();

            let start_line = content[..func.start_byte.min(content.len())]
                .lines()
                .count()
                + 1;
            let end_line = content[..func.end_byte.min(content.len())].lines().count() + 1;

            tx.execute(
                "INSERT OR REPLACE INTO functions (fid, name, file, language, start_line, \
                 end_line, start_byte, end_byte, signature, is_static, kind, indexed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'function', ?11)",
                params![
                    fid,
                    func.name,
                    result.rel_path,
                    result.language,
                    start_line as i64,
                    end_line as i64,
                    func.start_byte as i64,
                    func.end_byte as i64,
                    signature,
                    if func.is_static { 1 } else { 0 },
                    now,
                ],
            )
            .ok();

            func_count += 1;

            file_name_to_id
                .entry(result.rel_path.clone())
                .or_default()
                .insert(
                    func.name.clone(),
                    format!("{}::{}", result.rel_path, func.name),
                );

            name_to_ids
                .entry(func.name.clone())
                .or_default()
                .push(format!("{}::{}", result.rel_path, func.name));
        }
    }

    // Insert edges
    for result in &results {
        let call_pairs = build_call_pairs(&result.functions, &result.calls);

        for (caller_name, callee_name, kind) in call_pairs {
            let caller_fid = file_name_to_id
                .get(&result.rel_path)
                .and_then(|m| m.get(&caller_name))
                .cloned()
                .unwrap_or_else(|| format!("{}::{}", result.rel_path, caller_name));

            // Resolve callee
            let callee_fid = if let Some(file_map) = file_name_to_id.get(&result.rel_path) {
                file_map.get(&callee_name).cloned()
            } else {
                None
            }
            .or_else(|| {
                name_to_ids
                    .get(&callee_name)
                    .and_then(|ids| ids.first())
                    .cloned()
            })
            .unwrap_or_else(|| callee_name.clone());

            let confidence = match kind {
                CallKind::Direct => 1.0,
                CallKind::Indirect => 0.3,
                CallKind::Macro => 0.4,
                CallKind::Unknown => 0.0,
            };

            let line =
                content_line_at_byte(root, &result.rel_path, &result.functions, &caller_name);

            tx.execute(
                "INSERT INTO edges (caller, callee, kind, confidence, line) VALUES (?1, ?2, ?3, \
                 ?4, ?5)",
                params![
                    caller_fid,
                    callee_fid,
                    format!("{:?}", kind).to_lowercase(),
                    confidence,
                    line
                ],
            )
            .ok();

            edge_count += 1;
        }
    }

    tx.execute(
        "UPDATE codebases SET function_count = ?1, file_count = ?2, edge_count = ?3, last_indexed \
         = ?4, status = 'idle', languages = ?5
         WHERE root_path = ?6",
        params![
            func_count,
            file_count,
            edge_count,
            now,
            serde_json::to_string(&languages_used(&results)).unwrap_or_default(),
            root.to_str().unwrap(),
        ],
    )
    .ok();

    tx.commit().expect("failed to commit transaction");

    println!(
        "[index] Wrote {} files, {} functions, {} edges in {:.2}s",
        file_count,
        func_count,
        edge_count,
        db_start.elapsed().as_secs_f32()
    );

    println!(
        "[index] Total indexing time: {:.2}s",
        start.elapsed().as_secs_f32()
    );
}

fn content_line_at_byte(
    root: &Path,
    rel_path: &str,
    functions: &[RawFunctionOwned],
    caller_name: &str,
) -> Option<i64> {
    let func = functions.iter().find(|f| f.name == caller_name)?;
    let content = fs::read_to_string(root.join(rel_path)).ok()?;
    Some(
        content[..func.start_byte.min(content.len())]
            .lines()
            .count() as i64
            + 1,
    )
}

fn languages_used(results: &[ParseResult]) -> Vec<String> {
    let mut langs: Vec<String> = results.iter().map(|r| r.language.clone()).collect();
    langs.sort();
    langs.dedup();
    langs
}

// ─── Query mode ──────────────────────────────────────────────────────

fn query_db(db_path: &Path, args: &[String]) {
    if !db_path.exists() {
        eprintln!("[query] Database not found at {:?}", db_path);
        std::process::exit(1);
    }

    let conn = Connection::open(db_path).expect("failed to open database");

    if args.is_empty() {
        // List codebases
        let mut stmt = conn
            .prepare(
                "SELECT name, root_path, function_count, file_count, edge_count, last_indexed \
                 FROM codebases ORDER BY name",
            )
            .expect("failed to prepare");

        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, i64>(5)?,
                ))
            })
            .expect("failed to query");

        println!("Indexed codebases:");
        for row in rows {
            let (name, path, funcs, files, edges, indexed) = row.unwrap();
            println!(
                "  {} ({}) — {} functions, {} files, {} edges, indexed: {}",
                name, path, funcs, files, edges, indexed
            );
        }
        return;
    }

    let query: &str = &args[0];

    match query {
        "find" | "symbol" => {
            let pattern = args.get(1).expect("pattern required: find <pattern>");
            let kind = args.get(2).map(|s| s.as_str()).unwrap_or("function");
            let limit = args
                .get(3)
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(20);

            let mut stmt = conn
                .prepare(
                    "SELECT f.fid, f.name, f.file, f.language, f.start_line, f.end_line, \
                     f.signature, f.is_static
                     FROM functions f
                     WHERE f.name LIKE ?1 AND f.kind = ?2
                     ORDER BY f.name
                     LIMIT ?3",
                )
                .expect("failed to prepare");

            let pattern_with_wildcards = format!("%{}%", pattern);
            let rows: Vec<_> = stmt
                .query_map(params![pattern_with_wildcards, kind, limit], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, i64>(4)?,
                        r.get::<_, i64>(5)?,
                        r.get::<_, String>(6)?,
                        r.get::<_, i64>(7)?,
                    ))
                })
                .expect("failed to query")
                .filter_map(|r| r.ok())
                .collect();

            println!("Found {} symbol(s) matching \"{}\":\n", rows.len(), pattern);

            for (_fid, name, file, lang, start, end, sig, is_static) in &rows {
                println!(
                    "  {} ({}{}) @ {}:{}-{}",
                    name,
                    lang,
                    if *is_static != 0 { ", static" } else { "" },
                    file,
                    start,
                    end
                );
                println!("    {}", sig);
            }
        }

        "callers" => {
            let func = args.get(1).expect("function name required: callers <name>");
            let mut stmt = conn
                .prepare(
                    "SELECT DISTINCT f.fid, f.name, f.file, f.start_line, f.signature
                     FROM edges e
                     JOIN functions f ON f.fid = e.caller
                     WHERE e.callee = ?1 OR e.callee LIKE '%::' || ?1
                     ORDER BY f.name",
                )
                .expect("failed to prepare");

            let rows: Vec<_> = stmt
                .query_map(params![func], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, i64>(3)?,
                        r.get::<_, String>(4)?,
                    ))
                })
                .expect("failed to query")
                .filter_map(|r| r.ok())
                .collect();

            println!("Callers of \"{}\":\n", func);
            let count = rows.len();
            for (_, name, file, line, sig) in &rows {
                println!("  {} @ {}:{} — {}", name, file, line, sig);
            }
            if count == 0 {
                println!("  (none found)");
            }
        }

        "callees" => {
            let func = args.get(1).expect("function name required: callees <name>");
            let mut stmt = conn
                .prepare(
                    "SELECT DISTINCT f.fid, f.name, f.file, f.start_line, f.signature
                     FROM edges e
                     JOIN functions f ON f.fid = e.callee
                     WHERE e.caller = ?1 OR e.caller LIKE '%::' || ?1
                     ORDER BY f.name",
                )
                .expect("failed to prepare");

            let rows: Vec<_> = stmt
                .query_map(params![func], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, i64>(3)?,
                        r.get::<_, String>(4)?,
                    ))
                })
                .expect("failed to query")
                .filter_map(|r| r.ok())
                .collect();

            println!("Functions called by \"{}\":\n", func);
            let count = rows.len();
            for (_, name, file, line, sig) in &rows {
                println!("  {} @ {}:{} — {}", name, file, line, sig);
            }
            if count == 0 {
                println!("  (none found)");
            }
        }

        "stats" => {
            let mut stmt = conn
                .prepare(
                    "SELECT COUNT(*), COUNT(DISTINCT file), COUNT(DISTINCT language) FROM \
                     functions",
                )
                .expect("failed to prepare");

            let (total_funcs, total_files, total_langs): (i64, i64, i64) = stmt
                .query_row([], |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, i64>(1)?,
                        r.get::<_, i64>(2)?,
                    ))
                })
                .expect("failed to query");

            println!("Database statistics:");
            println!("  Functions: {}", total_funcs);
            println!("  Files: {}", total_files);
            println!("  Languages: {}", total_langs);

            // Top 10 most-connected functions
            let mut stmt = conn
                .prepare(
                    "SELECT f.name, f.file, COUNT(*) as edge_count
                     FROM edges e
                     JOIN functions f ON f.fid = e.caller
                     GROUP BY e.caller
                     ORDER BY edge_count DESC
                     LIMIT 10",
                )
                .expect("failed to prepare");

            let rows: Vec<_> = stmt
                .query_map([], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, i64>(2)?,
                    ))
                })
                .expect("failed to query")
                .filter_map(|r| r.ok())
                .collect();

            println!("\nTop 10 most-connected functions:");
            for (name, file, count) in &rows {
                println!("  {} ({}) — {} edges", name, file, count);
            }
        }

        _ => {
            eprintln!("Unknown query: {}", query);
            eprintln!(
                "Available: find <pattern> [kind] [limit], callers <name>, callees <name>, stats"
            );
        }
    }
}

// ─── Main ────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage:");
        eprintln!("  codeanalyzer-index <path> [options]     # index a codebase");
        eprintln!("  codeanalyzer-index --query [args...]     # query the database");
        eprintln!();
        eprintln!("Index options:");
        eprintln!("  --refresh    Incremental update (re-parse changed files only)");
        eprintln!("  --db <path>  Database path (default: ~/.local/share/codeanalyzer/uirs.db)");
        eprintln!();
        eprintln!("Query commands:");
        eprintln!("  find <pattern> [kind] [limit]   Search symbols");
        eprintln!("  callers <name>                  Find who calls a function");
        eprintln!("  callees <name>                  Find what a function calls");
        eprintln!("  stats                           Database statistics");
        std::process::exit(1);
    }

    // Find custom db path
    let mut db_path_override: Option<String> = None;
    let mut refresh = false;
    let mut query_mode = false;
    let mut query_args: Vec<String> = Vec::new();
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
                db_path_override = args.get(i).cloned();
            }
            _ if codebase_path.is_none() => {
                codebase_path = Some(args[i].clone());
            }
            _ => {}
        }
        i += 1;
    }

    let db_path = db_path_override.map(PathBuf::from).unwrap_or_else(|| {
        let mut p = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        p.push("codeanalyzer");
        fs::create_dir_all(&p).ok();
        p.push("uirs.db");
        p
    });

    if query_mode {
        query_db(&db_path, &query_args);
    } else if let Some(path) = codebase_path {
        let root = Path::new(&path);
        if !root.is_dir() {
            eprintln!("Error: '{}' is not a directory", path);
            std::process::exit(1);
        }
        index_codebase(root, &db_path, refresh);
    } else {
        eprintln!("Error: no codebase path specified");
        std::process::exit(1);
    }
}
