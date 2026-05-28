use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use crate::pipeline;

const MAX_FILE_BYTES: usize = 256 * 1024;
const MAX_TOTAL_BYTES: usize = 32 * 1024 * 1024;
const MAX_FILES: usize = 8192;
const MAX_REPO_MAP_DEFINITIONS: usize = 220;
const MAX_REPO_MAP_IMPORTS: usize = 160;
const MAX_REVEAL_BYTES: usize = 16 * 1024;

#[derive(Clone, Debug)]
pub struct RepoWorkspace {
    root: PathBuf,
    files: Vec<RepoFile>,
    skipped_files: usize,
    skipped_bytes: usize,
    index: RepoIndex,
    repo_map: String,
}

#[derive(Clone, Debug)]
pub struct RepoFile {
    path: String,
    bytes: Vec<u8>,
    text: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct RepoIndex {
    definitions: Vec<Definition>,
    imports: Vec<Import>,
    by_path: BTreeMap<String, usize>,
}

#[derive(Clone, Debug)]
pub struct Definition {
    pub path: String,
    pub name: String,
    pub kind: DefinitionKind,
    pub line: usize,
    pub byte_start: usize,
    pub byte_end: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DefinitionKind {
    Function,
    Struct,
    Enum,
    Trait,
    Impl,
    Const,
    Type,
}

#[derive(Clone, Debug)]
pub struct Import {
    pub path: String,
    pub target: String,
    pub line: usize,
}

impl pipeline::RepoRevealer for RepoWorkspace {
    fn reveal_file(&self, path: &str) -> Option<String> {
        self.reveal_file(path)
    }

    fn reveal_definition(&self, name: &str) -> Option<String> {
        self.reveal_definition(name)
    }
}

impl RepoWorkspace {
    pub fn load(root: impl Into<PathBuf>) -> io::Result<Self> {
        let root = root.into();
        let mut workspace = Self {
            root,
            files: Vec::new(),
            skipped_files: 0,
            skipped_bytes: 0,
            index: RepoIndex::default(),
            repo_map: String::new(),
        };
        let root = workspace.root.clone();
        workspace.load_dir(&root)?;
        workspace.files.sort_by(|left, right| left.path.cmp(&right.path));
        workspace.rebuild_index();
        Ok(workspace)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub fn total_bytes(&self) -> usize {
        self.files.iter().map(|file| file.bytes.len()).sum()
    }

    pub fn repo_map(&self) -> &str {
        &self.repo_map
    }

    pub fn context_for_request(&self, _request: &str) -> String {
        self.repo_map.clone()
    }

    pub fn read_text(&self, path: &str) -> Option<&str> {
        self.files
            .iter()
            .find(|file| file.path == normalize_repo_path(path))
            .and_then(|file| file.text.as_deref())
    }

    pub fn reveal_file(&self, path: &str) -> Option<String> {
        let text = self.read_text(path)?;
        Some(truncate_chars(text, MAX_REVEAL_BYTES))
    }

    pub fn reveal_definition(&self, name: &str) -> Option<String> {
        let definition = self
            .index
            .definitions
            .iter()
            .find(|definition| definition.name == name)
            .or_else(|| {
                self.index
                    .definitions
                    .iter()
                    .find(|definition| definition.name.ends_with(name))
            })?;
        let text = self.read_text(&definition.path)?;
        let start = definition.byte_start.min(text.len());
        let end = definition.byte_end.min(text.len()).max(start);
        Some(format!(
            "{}:{} {:?} {}\n{}",
            definition.path,
            definition.line,
            definition.kind,
            definition.name,
            truncate_chars(&text[start..end], MAX_REVEAL_BYTES)
        ))
    }

    pub fn replace_text(&mut self, path: &str, new_text: String) -> bool {
        let normalized = normalize_repo_path(path);
        let Some(file) = self.files.iter_mut().find(|file| file.path == normalized) else {
            return false;
        };
        file.bytes = new_text.as_bytes().to_vec();
        file.text = Some(new_text);
        self.rebuild_index();
        true
    }

    pub fn write_back(&self, path: &str) -> io::Result<()> {
        let normalized = normalize_repo_path(path);
        let Some(file) = self.files.iter().find(|file| file.path == normalized) else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "repo workspace path not loaded"));
        };
        let disk_path = self.root.join(&file.path);
        if let Some(parent) = disk_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(disk_path, &file.bytes)
    }

    fn rebuild_index(&mut self) {
        self.index = RepoIndex::default();
        for (index, file) in self.files.iter().enumerate() {
            self.index.by_path.insert(file.path.clone(), index);
            if let Some(text) = file.text.as_deref() {
                index_file(&mut self.index, &file.path, text);
            }
        }
        self.repo_map = build_repo_map(self);
    }

    fn load_dir(&mut self, dir: &Path) -> io::Result<()> {
        if self.files.len() >= MAX_FILES || self.total_bytes() >= MAX_TOTAL_BYTES {
            return Ok(());
        }
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if should_ignore_name(name) {
                continue;
            }
            let metadata = entry.metadata()?;
            if metadata.is_dir() {
                self.load_dir(&path)?;
            } else if metadata.is_file() {
                self.load_file(&path, metadata.len() as usize)?;
            }
        }
        Ok(())
    }

    fn load_file(&mut self, path: &Path, size: usize) -> io::Result<()> {
        if self.files.len() >= MAX_FILES {
            self.skipped_files += 1;
            self.skipped_bytes += size;
            return Ok(());
        }
        if size > MAX_FILE_BYTES || self.total_bytes().saturating_add(size) > MAX_TOTAL_BYTES {
            self.skipped_files += 1;
            self.skipped_bytes += size;
            return Ok(());
        }
        if should_ignore_path(path) {
            return Ok(());
        }
        let bytes = fs::read(path)?;
        let rel = path.strip_prefix(&self.root).unwrap_or(path);
        let repo_path = normalize_repo_path(&rel.to_string_lossy());
        let text = String::from_utf8(bytes.clone()).ok();
        self.files.push(RepoFile {
            path: repo_path,
            bytes,
            text,
        });
        Ok(())
    }

    fn important_paths(&self) -> Vec<&str> {
        let mut paths: Vec<&str> = self.files.iter().map(|file| file.path.as_str()).collect();
        paths.sort_by_key(|path| important_path_rank(path));
        paths
    }
}

fn build_repo_map(workspace: &RepoWorkspace) -> String {
    let mut out = String::new();
    out.push_str("RepoExpertIndex:\n");
    out.push_str(&format!("Root: {}\n", workspace.root.display()));
    out.push_str(&format!("LoadedFiles: {}\n", workspace.file_count()));
    out.push_str(&format!("LoadedBytes: {}\n", workspace.total_bytes()));
    out.push_str(&format!("SkippedFiles: {}\n", workspace.skipped_files));
    out.push_str(&format!("SkippedBytes: {}\n", workspace.skipped_bytes));
    out.push_str("\nImportantFiles:\n");
    for path in workspace.important_paths().into_iter().take(120) {
        out.push_str("- ");
        out.push_str(path);
        out.push('\n');
    }

    out.push_str("\nDefinitions:\n");
    for definition in workspace.index.definitions.iter().take(MAX_REPO_MAP_DEFINITIONS) {
        out.push_str("- ");
        out.push_str(&definition.path);
        out.push(':');
        out.push_str(&definition.line.to_string());
        out.push(' ');
        out.push_str(kind_label(definition.kind));
        out.push(' ');
        out.push_str(&definition.name);
        out.push('\n');
    }

    out.push_str("\nImports:\n");
    for import in workspace.index.imports.iter().take(MAX_REPO_MAP_IMPORTS) {
        out.push_str("- ");
        out.push_str(&import.path);
        out.push(':');
        out.push_str(&import.line.to_string());
        out.push(' ');
        out.push_str(&import.target);
        out.push('\n');
    }

    out.push_str("\nRevealTools:\n");
    out.push_str("- repo_reveal_file(path): reveal a loaded file body from memory.\n");
    out.push_str("- repo_reveal_definition(name): reveal a known function/type/body from memory.\n");
    out.push_str("- repo_edit(path, replacement): edit in-memory file contents.\n");
    out.push_str("- repo_writeback(path): write one edited file back to disk only after review.\n");
    out.push_str("- host_shell: non-repo OS command only when explicitly requested by user.\n");
    out
}

fn index_file(index: &mut RepoIndex, path: &str, text: &str) {
    let mut byte_offset = 0;
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if let Some(definition) = parse_definition(path, text, trimmed, line_index + 1, byte_offset) {
            index.definitions.push(definition);
        }
        if let Some(import) = parse_import(path, trimmed, line_index + 1) {
            index.imports.push(import);
        }
        byte_offset += line.len() + 1;
    }
}

fn parse_definition(path: &str, text: &str, line: &str, line_no: usize, byte_offset: usize) -> Option<Definition> {
    let stripped = line.strip_prefix("pub ").unwrap_or(line);
    let (kind, rest) = if let Some(rest) = stripped.strip_prefix("fn ") {
        (DefinitionKind::Function, rest)
    } else if let Some(rest) = stripped.strip_prefix("async fn ") {
        (DefinitionKind::Function, rest)
    } else if let Some(rest) = stripped.strip_prefix("struct ") {
        (DefinitionKind::Struct, rest)
    } else if let Some(rest) = stripped.strip_prefix("enum ") {
        (DefinitionKind::Enum, rest)
    } else if let Some(rest) = stripped.strip_prefix("trait ") {
        (DefinitionKind::Trait, rest)
    } else if let Some(rest) = stripped.strip_prefix("impl ") {
        (DefinitionKind::Impl, rest)
    } else if let Some(rest) = stripped.strip_prefix("const ") {
        (DefinitionKind::Const, rest)
    } else if let Some(rest) = stripped.strip_prefix("type ") {
        (DefinitionKind::Type, rest)
    } else if let Some(rest) = stripped.strip_prefix("pub const ") {
        (DefinitionKind::Const, rest)
    } else if let Some(rest) = stripped.strip_prefix("pub type ") {
        (DefinitionKind::Type, rest)
    } else if let Some(rest) = stripped.strip_prefix("pub struct ") {
        (DefinitionKind::Struct, rest)
    } else if let Some(rest) = stripped.strip_prefix("pub enum ") {
        (DefinitionKind::Enum, rest)
    } else {
        return None;
    };

    let name = definition_name(kind, rest)?;
    let byte_end = definition_end(text, byte_offset, kind);
    Some(Definition {
        path: path.to_string(),
        name,
        kind,
        line: line_no,
        byte_start: byte_offset,
        byte_end,
    })
}

fn definition_name(kind: DefinitionKind, rest: &str) -> Option<String> {
    let name = match kind {
        DefinitionKind::Impl => rest
            .split(|ch: char| ch == '{' || ch.is_whitespace())
            .find(|value| !value.is_empty())?,
        _ => rest
            .split(|ch: char| ch == '(' || ch == '<' || ch == ':' || ch == '=' || ch == '{' || ch.is_whitespace())
            .find(|value| !value.is_empty())?,
    };
    Some(name.trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_').to_string())
        .filter(|value| !value.is_empty())
}

fn definition_end(text: &str, byte_start: usize, kind: DefinitionKind) -> usize {
    let max = (byte_start + MAX_REVEAL_BYTES).min(text.len());
    if matches!(kind, DefinitionKind::Const | DefinitionKind::Type) {
        return text[byte_start..max]
            .find('\n')
            .map(|offset| byte_start + offset)
            .unwrap_or(max);
    }

    let slice = &text[byte_start..max];
    let mut depth = 0usize;
    let mut seen_open = false;
    for (offset, ch) in slice.char_indices() {
        if ch == '{' {
            depth += 1;
            seen_open = true;
        } else if ch == '}' && seen_open {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return byte_start + offset + ch.len_utf8();
            }
        }
    }
    max
}

fn parse_import(path: &str, line: &str, line_no: usize) -> Option<Import> {
    let target = line
        .strip_prefix("use ")
        .or_else(|| line.strip_prefix("mod "))
        .or_else(|| line.strip_prefix("const ").and_then(|value| value.split("@import(").nth(1)))?;
    Some(Import {
        path: path.to_string(),
        target: target.trim().trim_end_matches(';').trim_matches('"').to_string(),
        line: line_no,
    })
}

fn important_path_rank(path: &str) -> usize {
    if path == "Cargo.toml" || path == "Makefile" || path.ends_with("/Cargo.toml") {
        return 0;
    }
    if path.ends_with("main.rs") || path.ends_with("lib.rs") || path.ends_with("mod.rs") {
        return 1;
    }
    if path.contains("codex") || path.contains("agent") || path.contains("host") {
        return 2;
    }
    if path.ends_with(".rs") || path.ends_with(".zig") {
        return 3;
    }
    if path.ends_with(".md") || path.ends_with(".txt") {
        return 4;
    }
    5
}

fn kind_label(kind: DefinitionKind) -> &'static str {
    match kind {
        DefinitionKind::Function => "fn",
        DefinitionKind::Struct => "struct",
        DefinitionKind::Enum => "enum",
        DefinitionKind::Trait => "trait",
        DefinitionKind::Impl => "impl",
        DefinitionKind::Const => "const",
        DefinitionKind::Type => "type",
    }
}

fn normalize_repo_path(path: &str) -> String {
    path.replace('\\', "/")
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_string()
}

fn should_ignore_name(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".build"
            | "target"
            | "node_modules"
            | "vendor"
            | "dist"
            | "build"
            | ".cache"
            | ".zig-cache"
            | "zig-out"
    )
}

fn should_ignore_path(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
        return false;
    };
    matches!(
        ext,
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "wasm" | "a" | "o" | "so" | "dylib" | "dll" | "rlib" | "zip" | "tar" | "gz" | "xz" | "7z" | "bin"
    )
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for (index, ch) in value.chars().enumerate() {
        if index >= max_chars {
            out.push_str("...");
            break;
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_paths_are_repo_relative() {
        assert_eq!(normalize_repo_path("./a\\b.rs"), "a/b.rs");
    }

    #[test]
    fn parses_rust_function_definition() {
        let text = "pub fn hello() {\n    println!(\"hi\");\n}\n";
        let def = parse_definition("src/lib.rs", text, "pub fn hello() {", 1, 0).unwrap();
        assert_eq!(def.name, "hello");
        assert_eq!(def.kind, DefinitionKind::Function);
    }

    #[test]
    fn parses_zig_import_as_import() {
        let import = parse_import("src/app.zig", "const ui = @import(\"ui.zig\");", 1).unwrap();
        assert!(import.target.contains("ui.zig"));
    }
}
