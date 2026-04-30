//! Host-only Git repository explorer.
//!
//! Repositories are hidden by default. A repository becomes visible only when
//! the repository itself contains `.edgerun/git.yaml` with `visible: true`.

#![cfg_attr(target_os = "none", no_std)]

extern crate alloc;
#[cfg(not(target_os = "none"))]
extern crate std;

#[cfg(target_os = "none")]
compile_error!("edgerun-git is host-only because it shells out to git");

use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use edgerun_http::{Handler, Request, Response, StatusCode};
use edgerun_web_ui::PageShell;
use std::fs;
use std::io;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

const MAX_BLOB_BYTES: usize = 512 * 1024;
const DIR_VISIBILITY_MARKER: &str = ".gitvisible";
const FILE_VISIBILITY_SUFFIX: &str = ".gitvisible";

#[derive(Clone, Debug)]
pub struct GitConfig {
    pub root: PathBuf,
    pub bind_addr: String,
    pub title: String,
    pub description: String,
    pub base_url: String,
}

impl GitConfig {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            bind_addr: "127.0.0.1:8089".to_string(),
            title: "Edgerun Git".to_string(),
            description: "Code released from the Edgerun project.".to_string(),
            base_url: String::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct GitHandler {
    config: GitConfig,
}

#[derive(Clone, Debug)]
pub struct Repo {
    pub name: String,
    pub path: PathBuf,
    pub title: String,
    pub description: String,
    pub default_ref: String,
    pub public_paths: Vec<String>,
}

#[derive(Clone, Debug, Default)]
struct RepoPageConfig {
    visible: bool,
    title: Option<String>,
    description: Option<String>,
    default_ref: Option<String>,
    public_paths: Vec<String>,
}

#[derive(Clone, Debug)]
struct TreeEntry {
    mode: String,
    kind: String,
    hash: String,
    name: String,
}

#[derive(Clone, Debug)]
struct CrateInfo {
    name: String,
    rel_path: String,
    description: String,
    features: Vec<String>,
    api_items: Vec<ApiItem>,
    call_edges: Vec<CallEdge>,
    workspace_deps: Vec<String>,
    dependents: Vec<String>,
    test_count: usize,
    test_result: Option<String>,
    rfcs: Vec<RfcLink>,
}

#[derive(Clone, Debug)]
struct ApiItem {
    kind: String,
    name: String,
    path: String,
    line: usize,
}

#[derive(Clone, Debug)]
struct CallEdge {
    caller: String,
    callee: String,
    caller_path: String,
    caller_line: usize,
    callee_path: String,
    callee_line: usize,
    count: usize,
}

#[derive(Clone, Debug)]
struct FunctionDef {
    name: String,
    path: String,
    line: usize,
    body_start: usize,
    body_end: usize,
}

#[derive(Clone, Debug, Default)]
struct CrateMetadata {
    api_items: Vec<ApiItem>,
    call_edges: Vec<CallEdge>,
}

#[derive(Clone, Debug)]
struct GeneratedCrateFile {
    name: String,
    path: PathBuf,
    text: String,
}

#[derive(Clone, Debug)]
struct RustSource {
    path: String,
    text: String,
}

#[derive(Clone, Debug)]
struct RustToken {
    kind: RustTokenKind,
    line: usize,
    index: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RustTokenKind {
    Ident(String),
    Punct(char),
}

#[derive(Clone, Debug)]
struct RfcLink {
    path: String,
    title: String,
    completeness: String,
}

impl GitHandler {
    pub fn new(config: GitConfig) -> Self {
        Self { config }
    }

    pub fn handle_sync(&self, request: Request) -> Response {
        let target = request.uri().request_target();
        let path = target.split('?').next().unwrap_or("/");
        match self.route(path) {
            Ok(response) => response,
            Err(error) => server_error(error),
        }
    }

    fn route(&self, path: &str) -> io::Result<Response> {
        match path {
            "/" | "/index.html" => return self.index_response(),
            "/favicon.svg" => return Ok(favicon_response()),
            "/robots.txt" => return Ok(self.robots_response()),
            "/style.css" => return Ok(css_response()),
            "/app.js" => return Ok(js_response()),
            _ => {}
        }

        let segments = route_segments(path)?;
        if segments.is_empty() {
            return self.index_response();
        }
        let repo_name = &segments[0];
        let Some(repo) = self.visible_repo(repo_name)? else {
            return Ok(not_found_response(&self.config.title));
        };
        if segments.len() == 1 {
            return self.repo_response(&repo, &repo.default_ref, "");
        }
        match segments[1].as_str() {
            "crates" => {
                if segments.len() == 2 {
                    self.crates_response(&repo)
                } else if segments.len() == 3 {
                    self.crate_response(&repo, &segments[2])
                } else {
                    Ok(not_found_response(&self.config.title))
                }
            }
            "components" => {
                if segments.len() == 4 && segments[2] == "crates" {
                    self.crate_component_response(&repo, &segments[3])
                } else {
                    Ok(not_found_response(&self.config.title))
                }
            }
            "src" | "tree" => {
                let rev = segments
                    .get(2)
                    .map(String::as_str)
                    .unwrap_or(&repo.default_ref);
                let rel_path = segments
                    .get(3..)
                    .map(|parts| parts.join("/"))
                    .unwrap_or_default();
                self.repo_response(&repo, rev, &rel_path)
            }
            "commit" => {
                let Some(rev) = segments.get(2) else {
                    return Ok(not_found_response(&self.config.title));
                };
                self.commit_response(&repo, rev)
            }
            "raw" => {
                let Some(rev) = segments.get(2) else {
                    return Ok(not_found_response(&self.config.title));
                };
                let rel_path = segments
                    .get(3..)
                    .map(|parts| parts.join("/"))
                    .unwrap_or_default();
                self.raw_response(&repo, rev, &rel_path)
            }
            _ => Ok(not_found_response(&self.config.title)),
        }
    }

    fn index_response(&self) -> io::Result<Response> {
        let repos = self.visible_repos()?;
        Ok(html_response(render_index(&self.config, &repos)))
    }

    fn repo_response(&self, repo: &Repo, rev: &str, rel_path: &str) -> io::Result<Response> {
        if !safe_rev(rev) || !safe_repo_path(rel_path) {
            return Ok(not_found_response(&self.config.title));
        }
        let kind = git_output(
            &repo.path,
            &["cat-file", "-t", &format!("{rev}:{rel_path}")],
        )
        .ok()
        .map(|value| value.trim().to_string());
        match kind.as_deref() {
            Some("blob") => {
                if !path_is_public(repo, rel_path) {
                    return Ok(not_found_response(&self.config.title));
                }
                let bytes = git_output_bytes(&repo.path, &["show", &format!("{rev}:{rel_path}")])?;
                Ok(html_response(render_blob(
                    &self.config,
                    repo,
                    rev,
                    rel_path,
                    &bytes,
                )))
            }
            Some("tree") | None if rel_path.is_empty() => {
                let entries =
                    filter_tree_entries(repo, rel_path, git_tree(&repo.path, rev, rel_path)?);
                Ok(html_response(render_tree(
                    &self.config,
                    repo,
                    rev,
                    rel_path,
                    &entries,
                )))
            }
            Some("tree") => {
                if !path_is_public_or_ancestor(repo, rel_path) {
                    return Ok(not_found_response(&self.config.title));
                }
                let entries =
                    filter_tree_entries(repo, rel_path, git_tree(&repo.path, rev, rel_path)?);
                Ok(html_response(render_tree(
                    &self.config,
                    repo,
                    rev,
                    rel_path,
                    &entries,
                )))
            }
            _ => Ok(not_found_response(&self.config.title)),
        }
    }

    fn commit_response(&self, repo: &Repo, rev: &str) -> io::Result<Response> {
        if !safe_rev(rev) {
            return Ok(not_found_response(&self.config.title));
        }
        let commit = git_output(
            &repo.path,
            &[
                "show",
                "--no-patch",
                "--format=fuller",
                "--decorate=short",
                rev,
            ],
        )?;
        Ok(html_response(render_commit(
            &self.config,
            repo,
            rev,
            &commit,
        )))
    }

    fn raw_response(&self, repo: &Repo, rev: &str, rel_path: &str) -> io::Result<Response> {
        if !safe_rev(rev) || !safe_repo_path(rel_path) || rel_path.is_empty() {
            return Ok(not_found_response(&self.config.title));
        }
        if !path_is_public(repo, rel_path) {
            return Ok(not_found_response(&self.config.title));
        }
        let bytes = git_output_bytes(&repo.path, &["show", &format!("{rev}:{rel_path}")])?;
        Ok(Response::new(StatusCode::OK)
            .with_header("Content-Type", "text/plain; charset=utf-8")
            .with_header("Cache-Control", "public, max-age=300")
            .with_header("X-Content-Type-Options", "nosniff")
            .with_body(bytes))
    }

    fn crates_response(&self, repo: &Repo) -> io::Result<Response> {
        let crates = visible_crates(repo)?;
        Ok(html_response(render_crates_index(
            &self.config,
            repo,
            &crates,
        )))
    }

    fn crate_response(&self, repo: &Repo, crate_name: &str) -> io::Result<Response> {
        if !safe_repo_name(crate_name) {
            return Ok(not_found_response(&self.config.title));
        }
        let crates = visible_crates(repo)?;
        let Some(info) = crates.iter().find(|info| info.name == crate_name) else {
            return Ok(not_found_response(&self.config.title));
        };
        Ok(html_response(render_crate_page(
            &self.config,
            repo,
            info,
            &crates,
        )))
    }

    fn crate_component_response(&self, repo: &Repo, component_name: &str) -> io::Result<Response> {
        let Some(crate_name) = component_name.strip_suffix(".html") else {
            return Ok(not_found_response(&self.config.title));
        };
        if !safe_repo_name(crate_name) {
            return Ok(not_found_response(&self.config.title));
        }
        let crates = visible_crates(repo)?;
        let Some(info) = crates.iter().find(|info| info.name == crate_name) else {
            return Ok(not_found_response(&self.config.title));
        };
        Ok(component_response(render_crate_component_document(
            &self.config,
            repo,
            info,
        )))
    }

    fn robots_response(&self) -> Response {
        Response::new(StatusCode::OK)
            .with_header("Content-Type", "text/plain; charset=utf-8")
            .with_header("Cache-Control", "public, max-age=300")
            .with_header("X-Content-Type-Options", "nosniff")
            .with_body("User-agent: *\nAllow: /\n")
    }

    fn visible_repo(&self, name: &str) -> io::Result<Option<Repo>> {
        if !safe_repo_name(name) {
            return Ok(None);
        }
        let mut repo_path = self.config.root.join(name);
        if !is_git_repo(&repo_path) {
            repo_path = self.config.root.join(format!("{name}.git"));
        }
        if !is_git_repo(&repo_path) {
            return Ok(None);
        }
        let Some(repo) = load_repo(name.to_string(), repo_path)? else {
            return Ok(None);
        };
        Ok(Some(repo))
    }

    fn visible_repos(&self) -> io::Result<Vec<Repo>> {
        let mut repos = Vec::new();
        for entry in fs::read_dir(&self.config.root)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let dir_name = entry.file_name().to_string_lossy().to_string();
            let name = dir_name
                .strip_suffix(".git")
                .unwrap_or(&dir_name)
                .to_string();
            if !safe_repo_name(&name) {
                continue;
            }
            if let Some(repo) = load_repo(name, entry.path())? {
                repos.push(repo);
            }
        }
        repos.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
        Ok(repos)
    }
}

impl Handler for GitHandler {
    fn handle(&self, request: Request) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
        Box::pin(async move { self.handle_sync(request) })
    }
}

pub fn start_git(config: GitConfig) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send>> {
    Box::pin(async move {
        let bind_addr = config.bind_addr.clone();
        let handler = GitHandler::new(config);
        let server = edgerun_http::HttpServer::new(handler)
            .bind(bind_addr.clone())
            .await
            .map_err(to_io_error)?;
        edgerun_log::info!("edgerun-git: listening on {}", bind_addr);
        server.serve().await.map_err(to_io_error)
    })
}

/// Generate host-only crate surface metadata for public crates in a checkout.
///
/// The generated files are catalog material intended to be committed by a
/// developer-side git hook. Serving can then stay cheap and deterministic.
pub fn generate_crate_metadata(repo_root: &Path, out_dir: &Path) -> io::Result<usize> {
    let files = build_crate_metadata_files(repo_root, out_dir)?;
    fs::create_dir_all(out_dir)?;
    remove_stale_metadata_files(out_dir, &files)?;
    for generated in &files {
        let mut file = fs::File::create(&generated.path)?;
        file.write_all(generated.text.as_bytes())?;
    }
    Ok(files.len())
}

pub fn check_crate_metadata(repo_root: &Path, out_dir: &Path) -> io::Result<usize> {
    let files = build_crate_metadata_files(repo_root, out_dir)?;
    let mut stale = Vec::new();
    for generated in &files {
        match fs::read_to_string(&generated.path) {
            Ok(existing) if existing == generated.text => {}
            Ok(_) => stale.push(generated.path.display().to_string()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                stale.push(generated.path.display().to_string())
            }
            Err(error) => return Err(error),
        }
    }
    if out_dir.exists() {
        for entry in fs::read_dir(out_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("txt") {
                continue;
            }
            if !files.iter().any(|generated| generated.path == path) {
                stale.push(path.display().to_string());
            }
        }
    }
    if stale.is_empty() {
        Ok(files.len())
    } else {
        stale.sort();
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("stale generated crate metadata: {}", stale.join(", ")),
        ))
    }
}

fn build_crate_metadata_files(
    repo_root: &Path,
    out_dir: &Path,
) -> io::Result<Vec<GeneratedCrateFile>> {
    let crates_dir = repo_root.join("crates");
    let mut files = Vec::new();
    for entry in fs::read_dir(crates_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let crate_dir = entry.path();
        if !crate_dir.join(DIR_VISIBILITY_MARKER).exists() {
            continue;
        }
        let manifest_path = crate_dir.join("Cargo.toml");
        let manifest = match fs::read_to_string(&manifest_path) {
            Ok(manifest) => manifest,
            Err(_) => continue,
        };
        let Some(name) = manifest_string_value(&manifest, "name") else {
            continue;
        };
        if !safe_repo_name(&name) {
            continue;
        }
        let rel_path = crate_dir
            .strip_prefix(repo_root)
            .ok()
            .and_then(|path| path.to_str())
            .unwrap_or("")
            .replace('\\', "/");
        if rel_path.is_empty() {
            continue;
        }
        let sources = read_rust_sources_from_fs(repo_root, &rel_path)?;
        let metadata = analyze_rust_sources(sources);
        files.push(GeneratedCrateFile {
            path: out_dir.join(format!("{name}.txt")),
            text: serialize_crate_metadata(&name, &metadata),
            name,
        });
    }
    files.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(files)
}

fn remove_stale_metadata_files(out_dir: &Path, files: &[GeneratedCrateFile]) -> io::Result<()> {
    if !out_dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(out_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("txt") {
            continue;
        }
        if !files.iter().any(|generated| generated.path == path) {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

fn load_repo(name: String, path: PathBuf) -> io::Result<Option<Repo>> {
    if !is_git_repo(&path) {
        return Ok(None);
    }
    let page = load_repo_page_config(&path)?;
    if !page.visible {
        return Ok(None);
    }
    let default_ref = page
        .default_ref
        .unwrap_or_else(|| git_default_ref(&path).unwrap_or_else(|| "HEAD".to_string()));
    let mut public_paths = page.public_paths;
    for path in load_gitvisible_paths(&path, &default_ref)? {
        add_public_path(&mut public_paths, &path);
    }
    Ok(Some(Repo {
        title: page.title.unwrap_or_else(|| name.clone()),
        description: page.description.unwrap_or_default(),
        name,
        path,
        default_ref,
        public_paths,
    }))
}

fn load_repo_page_config(repo: &Path) -> io::Result<RepoPageConfig> {
    let path = repo.join(".edgerun/git.yaml");
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => match git_output(repo, &["show", "HEAD:.edgerun/git.yaml"]) {
            Ok(text) => text,
            Err(_) => return Ok(RepoPageConfig::default()),
        },
    };
    let mut config = RepoPageConfig::default();
    let mut list_key: Option<String> = None;
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if let Some(value) = line.strip_prefix("- ") {
            if matches!(list_key.as_deref(), Some("paths" | "public_paths")) {
                add_public_path(&mut config.public_paths, trim_quotes(value.trim()));
            }
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let raw_value = value.trim();
        let value = trim_quotes(raw_value).to_string();
        list_key = None;
        match key {
            "visible" => config.visible = matches!(value.as_str(), "true" | "yes" | "1"),
            "title" => config.title = nonempty(value),
            "description" => config.description = nonempty(value),
            "default_ref" | "default_refname" | "branch" => config.default_ref = nonempty(value),
            "paths" | "public_paths" => {
                if raw_value.is_empty() {
                    list_key = Some(key.to_string());
                } else {
                    for path in parse_inline_list(raw_value) {
                        add_public_path(&mut config.public_paths, &path);
                    }
                }
            }
            _ => {}
        }
    }
    Ok(config)
}

fn git_default_ref(repo: &Path) -> Option<String> {
    git_output(repo, &["symbolic-ref", "--short", "HEAD"])
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn git_tree(repo: &Path, rev: &str, rel_path: &str) -> io::Result<Vec<TreeEntry>> {
    let target = if rel_path.is_empty() {
        format!("{rev}:")
    } else {
        format!("{rev}:{rel_path}")
    };
    let output = git_output(repo, &["ls-tree", &target])?;
    let mut entries = Vec::new();
    for line in output.lines() {
        let Some((left, name)) = line.split_once('\t') else {
            continue;
        };
        let mut parts = left.split_whitespace();
        let Some(mode) = parts.next() else { continue };
        let Some(kind) = parts.next() else { continue };
        let Some(hash) = parts.next() else { continue };
        entries.push(TreeEntry {
            mode: mode.to_string(),
            kind: kind.to_string(),
            hash: hash.to_string(),
            name: name.to_string(),
        });
    }
    entries.sort_by(|a, b| {
        tree_sort_key(&a.kind)
            .cmp(&tree_sort_key(&b.kind))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries)
}

fn visible_crates(repo: &Repo) -> io::Result<Vec<CrateInfo>> {
    let mut crates = Vec::new();
    for entry in git_tree(&repo.path, &repo.default_ref, "crates")? {
        if entry.kind != "tree" || !safe_repo_name(&entry.name) {
            continue;
        }
        let rel_path = format!("crates/{}", entry.name);
        if !path_is_public(repo, &rel_path) {
            continue;
        }
        let manifest_path = format!("{rel_path}/Cargo.toml");
        let Ok(manifest) = git_output(
            &repo.path,
            &["show", &format!("{}:{manifest_path}", repo.default_ref)],
        ) else {
            continue;
        };
        crates.push(crate_info_from_manifest(repo, &rel_path, &manifest)?);
    }

    let names = crates
        .iter()
        .map(|info| info.name.clone())
        .collect::<Vec<_>>();
    for index in 0..crates.len() {
        let name = crates[index].name.clone();
        crates[index].dependents = crates
            .iter()
            .filter(|other| other.workspace_deps.iter().any(|dep| dep == &name))
            .map(|other| other.name.clone())
            .collect();
        crates[index]
            .workspace_deps
            .retain(|dep| names.iter().any(|name| name == dep));
    }

    crates.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(crates)
}

fn crate_info_from_manifest(repo: &Repo, rel_path: &str, manifest: &str) -> io::Result<CrateInfo> {
    let name = manifest_string_value(manifest, "name")
        .unwrap_or_else(|| rel_path.rsplit('/').next().unwrap_or("unknown").to_string());
    let description = manifest_string_value(manifest, "description").unwrap_or_default();
    let features = manifest_table_keys(manifest, "features");
    let generated = load_generated_crate_metadata(repo, &name)?;
    let api_items = if generated.api_items.is_empty() {
        extract_public_api(repo, rel_path)?
    } else {
        generated.api_items
    };
    let call_edges = if generated.call_edges.is_empty() {
        extract_call_graph(repo, rel_path)?
    } else {
        generated.call_edges
    };
    let workspace_deps = workspace_dependency_names(manifest);
    let test_count = count_crate_tests(repo, rel_path)?;
    let test_result = load_crate_test_result(repo, &name).ok();
    let rfcs = related_rfcs(repo, &name, rel_path)?;
    Ok(CrateInfo {
        name,
        rel_path: rel_path.to_string(),
        description,
        features,
        api_items,
        call_edges,
        workspace_deps,
        dependents: Vec::new(),
        test_count,
        test_result,
        rfcs,
    })
}

fn manifest_string_value(manifest: &str, key: &str) -> Option<String> {
    manifest.lines().find_map(|line| {
        let line = line.split('#').next().unwrap_or("").trim();
        let (left, right) = line.split_once('=')?;
        if left.trim() == key {
            Some(trim_quotes(right.trim()).to_string())
        } else {
            None
        }
    })
}

fn manifest_table_keys(manifest: &str, table: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut in_table = false;
    for line in manifest.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_table = line == format!("[{table}]");
            continue;
        }
        if !in_table || line.is_empty() {
            continue;
        }
        if let Some((key, _)) = line.split_once('=') {
            keys.push(key.trim().to_string());
        }
    }
    keys.sort();
    keys
}

fn workspace_dependency_names(manifest: &str) -> Vec<String> {
    let mut deps = Vec::new();
    let mut in_deps = false;
    for line in manifest.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_deps = matches!(
                line,
                "[dependencies]" | "[dev-dependencies]" | "[build-dependencies]"
            );
            continue;
        }
        if !in_deps || line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim().starts_with("edgerun-") && value.contains("path") {
            deps.push(key.trim().to_string());
        }
    }
    deps.sort();
    deps.dedup();
    deps
}

fn extract_public_api(repo: &Repo, rel_path: &str) -> io::Result<Vec<ApiItem>> {
    Ok(analyze_rust_sources(read_rust_sources_from_git(repo, rel_path)?).api_items)
}

fn extract_call_graph(repo: &Repo, rel_path: &str) -> io::Result<Vec<CallEdge>> {
    Ok(analyze_rust_sources(read_rust_sources_from_git(repo, rel_path)?).call_edges)
}

fn read_rust_sources_from_git(repo: &Repo, rel_path: &str) -> io::Result<Vec<RustSource>> {
    let files = git_output(
        &repo.path,
        &["ls-tree", "-r", "--name-only", &repo.default_ref, rel_path],
    )?;
    let mut out = Vec::new();
    for file in files.lines().filter(|file| file.ends_with(".rs")) {
        let Ok(source) = git_output(
            &repo.path,
            &["show", &format!("{}:{file}", repo.default_ref)],
        ) else {
            continue;
        };
        out.push(RustSource {
            path: file.to_string(),
            text: source,
        });
    }
    Ok(out)
}

fn read_rust_sources_from_fs(repo_root: &Path, rel_path: &str) -> io::Result<Vec<RustSource>> {
    let mut out = Vec::new();
    collect_rust_sources(repo_root, &repo_root.join(rel_path), &mut out)?;
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

fn collect_rust_sources(repo_root: &Path, dir: &Path, out: &mut Vec<RustSource>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect_rust_sources(repo_root, &path, out)?;
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        let Ok(rel_path) = path.strip_prefix(repo_root) else {
            continue;
        };
        let Some(rel_path) = rel_path.to_str() else {
            continue;
        };
        out.push(RustSource {
            path: rel_path.replace('\\', "/"),
            text: fs::read_to_string(path)?,
        });
    }
    Ok(())
}

fn analyze_rust_sources(sources: Vec<RustSource>) -> CrateMetadata {
    let mut metadata = CrateMetadata::default();
    let mut functions = Vec::new();
    let mut token_sets = Vec::new();
    for source in sources {
        let stripped = strip_cfg_test_modules(&source.text);
        let tokens = rust_tokens(&stripped);
        metadata
            .api_items
            .extend(parse_api_items(&source.path, &tokens));
        functions.extend(parse_functions(&source.path, &tokens));
        token_sets.push((source.path, tokens));
    }
    let mut unique_function_names = Vec::new();
    for function in &functions {
        if !unique_function_names
            .iter()
            .any(|name| name == &function.name)
            && functions
                .iter()
                .filter(|candidate| candidate.name == function.name)
                .count()
                == 1
        {
            unique_function_names.push(function.name.clone());
        }
    }
    for (path, tokens) in &token_sets {
        let file_functions = functions
            .iter()
            .filter(|function| function.path == *path)
            .collect::<Vec<_>>();
        for caller in file_functions {
            for token in tokens
                .iter()
                .filter(|token| token.index > caller.body_start && token.index < caller.body_end)
            {
                let RustTokenKind::Ident(name) = &token.kind else {
                    continue;
                };
                if name == &caller.name || !token_is_followed_by_punct(tokens, token.index, '(') {
                    continue;
                }
                if !unique_function_names.iter().any(|unique| unique == name) {
                    continue;
                }
                let Some(callee) = functions.iter().find(|function| function.name == *name) else {
                    continue;
                };
                add_call_edge(&mut metadata.call_edges, caller, callee);
            }
        }
    }
    metadata.api_items.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.name.cmp(&b.name))
    });
    metadata.call_edges.sort_by(|a, b| {
        a.caller
            .cmp(&b.caller)
            .then_with(|| a.callee.cmp(&b.callee))
            .then_with(|| a.caller_path.cmp(&b.caller_path))
            .then_with(|| a.caller_line.cmp(&b.caller_line))
    });
    metadata
}

fn strip_cfg_test_modules(source: &str) -> String {
    let mut out = String::new();
    let mut cfg_test_pending = false;
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("#[cfg(test)]") {
            cfg_test_pending = true;
            continue;
        }
        if cfg_test_pending && trimmed.starts_with("mod tests") {
            break;
        }
        if cfg_test_pending {
            out.push_str("#[cfg(test)]\n");
            cfg_test_pending = false;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn add_call_edge(edges: &mut Vec<CallEdge>, caller: &FunctionDef, callee: &FunctionDef) {
    if let Some(edge) = edges.iter_mut().find(|edge| {
        edge.caller == caller.name
            && edge.callee == callee.name
            && edge.caller_path == caller.path
            && edge.callee_path == callee.path
    }) {
        edge.count += 1;
        return;
    }
    edges.push(CallEdge {
        caller: caller.name.clone(),
        callee: callee.name.clone(),
        caller_path: caller.path.clone(),
        caller_line: caller.line,
        callee_path: callee.path.clone(),
        callee_line: callee.line,
        count: 1,
    });
}

fn parse_api_items(path: &str, tokens: &[RustToken]) -> Vec<ApiItem> {
    let mut items = Vec::new();
    let mut index = 0usize;
    while index < tokens.len() {
        if !token_is_ident(tokens, index, "pub") {
            index += 1;
            continue;
        }
        let mut cursor = skip_visibility(tokens, index + 1);
        while token_is_modifier(tokens, cursor) {
            cursor += 1;
        }
        let Some(kind) = token_ident(tokens, cursor) else {
            index += 1;
            continue;
        };
        if !matches!(
            kind,
            "fn" | "struct" | "enum" | "trait" | "type" | "const" | "static" | "mod"
        ) {
            index += 1;
            continue;
        }
        let Some(name) = token_ident(tokens, cursor + 1) else {
            index += 1;
            continue;
        };
        items.push(ApiItem {
            kind: kind.to_string(),
            name: name.to_string(),
            path: path.to_string(),
            line: tokens[index].line,
        });
        index = cursor + 2;
    }
    items
}

fn parse_functions(path: &str, tokens: &[RustToken]) -> Vec<FunctionDef> {
    let mut functions = Vec::new();
    let mut index = 0usize;
    while index < tokens.len() {
        if !token_is_ident(tokens, index, "fn") {
            index += 1;
            continue;
        }
        let Some(name) = token_ident(tokens, index + 1) else {
            index += 1;
            continue;
        };
        let Some(open_brace) = find_punct(tokens, index + 2, '{') else {
            index += 1;
            continue;
        };
        let Some(close_brace) = matching_brace(tokens, open_brace) else {
            index += 1;
            continue;
        };
        functions.push(FunctionDef {
            name: name.to_string(),
            path: path.to_string(),
            line: tokens[index].line,
            body_start: open_brace,
            body_end: close_brace,
        });
        index = close_brace + 1;
    }
    functions
}

fn rust_tokens(source: &str) -> Vec<RustToken> {
    let mut tokens = Vec::new();
    let mut chars = source.char_indices().peekable();
    let mut line = 1usize;
    while let Some((_, ch)) = chars.next() {
        if ch == '\n' {
            line += 1;
            continue;
        }
        if ch.is_whitespace() {
            continue;
        }
        if ch == '/' && chars.peek().is_some_and(|(_, next)| *next == '/') {
            for (_, next) in chars.by_ref() {
                if next == '\n' {
                    line += 1;
                    break;
                }
            }
            continue;
        }
        if ch == '/' && chars.peek().is_some_and(|(_, next)| *next == '*') {
            let _ = chars.next();
            let mut prev = '\0';
            for (_, next) in chars.by_ref() {
                if next == '\n' {
                    line += 1;
                }
                if prev == '*' && next == '/' {
                    break;
                }
                prev = next;
            }
            continue;
        }
        if ch == '\'' {
            let mut lookahead = chars.clone();
            let next = lookahead.next().map(|(_, next)| next);
            let after_next = lookahead.next().map(|(_, next)| next);
            if next.is_some_and(is_ident_start) && after_next != Some('\'') {
                continue;
            }
        }
        if ch == '"' || ch == '\'' {
            skip_quoted(ch, &mut chars, &mut line);
            continue;
        }
        if ch == 'r'
            && chars
                .peek()
                .is_some_and(|(_, next)| *next == '"' || *next == '#')
        {
            skip_raw_string(&mut chars, &mut line);
            continue;
        }
        if is_ident_start(ch) {
            let mut ident = String::new();
            ident.push(ch);
            while let Some((_, next)) = chars.peek().copied() {
                if !is_ident_continue(next) {
                    break;
                }
                ident.push(next);
                let _ = chars.next();
            }
            let index = tokens.len();
            tokens.push(RustToken {
                kind: RustTokenKind::Ident(ident),
                line,
                index,
            });
            continue;
        }
        if "{}()[];:,.<>!&|=+-*/#".contains(ch) {
            let index = tokens.len();
            tokens.push(RustToken {
                kind: RustTokenKind::Punct(ch),
                line,
                index,
            });
        }
    }
    tokens
}

fn skip_quoted(
    quote: char,
    chars: &mut core::iter::Peekable<core::str::CharIndices<'_>>,
    line: &mut usize,
) {
    let mut escaped = false;
    for (_, ch) in chars.by_ref() {
        if ch == '\n' {
            *line += 1;
        }
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == quote {
            break;
        }
    }
}

fn skip_raw_string(chars: &mut core::iter::Peekable<core::str::CharIndices<'_>>, line: &mut usize) {
    let mut hashes = 0usize;
    while let Some((_, '#')) = chars.peek().copied() {
        hashes += 1;
        let _ = chars.next();
    }
    if !matches!(chars.peek(), Some((_, '"'))) {
        return;
    }
    let _ = chars.next();
    let mut saw_quote = false;
    let mut closing_hashes = 0usize;
    for (_, ch) in chars.by_ref() {
        if ch == '\n' {
            *line += 1;
        }
        if ch == '"' {
            if hashes == 0 {
                break;
            }
            saw_quote = true;
            closing_hashes = 0;
            continue;
        }
        if saw_quote && ch == '#' {
            closing_hashes += 1;
            if closing_hashes == hashes {
                break;
            }
        } else {
            saw_quote = false;
            closing_hashes = 0;
        }
    }
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn skip_visibility(tokens: &[RustToken], mut index: usize) -> usize {
    if !token_is_punct(tokens, index, '(') {
        return index;
    }
    let mut depth = 0isize;
    while index < tokens.len() {
        if token_is_punct(tokens, index, '(') {
            depth += 1;
        } else if token_is_punct(tokens, index, ')') {
            depth -= 1;
            if depth == 0 {
                return index + 1;
            }
        }
        index += 1;
    }
    index
}

fn token_is_modifier(tokens: &[RustToken], index: usize) -> bool {
    matches!(
        token_ident(tokens, index),
        Some("async" | "unsafe" | "const" | "extern")
    )
}

fn token_is_ident(tokens: &[RustToken], index: usize, expected: &str) -> bool {
    token_ident(tokens, index) == Some(expected)
}

fn token_ident(tokens: &[RustToken], index: usize) -> Option<&str> {
    let token = tokens.get(index)?;
    let RustTokenKind::Ident(value) = &token.kind else {
        return None;
    };
    Some(value)
}

fn token_is_punct(tokens: &[RustToken], index: usize, expected: char) -> bool {
    matches!(
        tokens.get(index).map(|token| &token.kind),
        Some(RustTokenKind::Punct(value)) if *value == expected
    )
}

fn token_is_followed_by_punct(tokens: &[RustToken], index: usize, expected: char) -> bool {
    token_is_punct(tokens, index + 1, expected)
}

fn find_punct(tokens: &[RustToken], mut index: usize, expected: char) -> Option<usize> {
    while index < tokens.len() {
        if token_is_punct(tokens, index, ';') {
            return None;
        }
        if token_is_punct(tokens, index, expected) {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn matching_brace(tokens: &[RustToken], open_index: usize) -> Option<usize> {
    let mut depth = 0isize;
    for index in open_index..tokens.len() {
        if token_is_punct(tokens, index, '{') {
            depth += 1;
        } else if token_is_punct(tokens, index, '}') {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn serialize_crate_metadata(crate_name: &str, metadata: &CrateMetadata) -> String {
    let mut out = String::new();
    out.push_str("# generated by edgerun-git generate; do not edit\n");
    out.push_str("# kind: generated type/catalog material; target: host-only\n");
    out.push_str(&format!("crate\t{crate_name}\n"));
    for item in &metadata.api_items {
        out.push_str(&format!(
            "api\t{}\t{}\t{}\t{}\n",
            item.kind, item.name, item.path, item.line
        ));
    }
    for edge in &metadata.call_edges {
        out.push_str(&format!(
            "call\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            edge.caller,
            edge.callee,
            edge.caller_path,
            edge.caller_line,
            edge.callee_path,
            edge.callee_line,
            edge.count
        ));
    }
    out
}

fn load_generated_crate_metadata(repo: &Repo, crate_name: &str) -> io::Result<CrateMetadata> {
    let path = format!(".edgerun/git/crates/{crate_name}.txt");
    let text = match git_output(
        &repo.path,
        &["show", &format!("{}:{path}", repo.default_ref)],
    ) {
        Ok(text) => text,
        Err(_) => return Ok(CrateMetadata::default()),
    };
    Ok(parse_crate_metadata(&text))
}

fn parse_crate_metadata(text: &str) -> CrateMetadata {
    let mut metadata = CrateMetadata::default();
    for line in text.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts = line.split('\t').collect::<Vec<_>>();
        match parts.as_slice() {
            ["api", kind, name, path, line] => {
                let Ok(line) = line.parse::<usize>() else {
                    continue;
                };
                metadata.api_items.push(ApiItem {
                    kind: (*kind).to_string(),
                    name: (*name).to_string(),
                    path: (*path).to_string(),
                    line,
                });
            }
            ["call", caller, callee, caller_path, caller_line, callee_path, callee_line, count] => {
                let (Ok(caller_line), Ok(callee_line), Ok(count)) = (
                    caller_line.parse::<usize>(),
                    callee_line.parse::<usize>(),
                    count.parse::<usize>(),
                ) else {
                    continue;
                };
                metadata.call_edges.push(CallEdge {
                    caller: (*caller).to_string(),
                    callee: (*callee).to_string(),
                    caller_path: (*caller_path).to_string(),
                    caller_line,
                    callee_path: (*callee_path).to_string(),
                    callee_line,
                    count,
                });
            }
            _ => {}
        }
    }
    metadata
}

fn count_crate_tests(repo: &Repo, rel_path: &str) -> io::Result<usize> {
    let files = git_output(
        &repo.path,
        &["ls-tree", "-r", "--name-only", &repo.default_ref, rel_path],
    )?;
    let mut count = 0;
    for file in files.lines().filter(|file| file.ends_with(".rs")) {
        let Ok(source) = git_output(
            &repo.path,
            &["show", &format!("{}:{file}", repo.default_ref)],
        ) else {
            continue;
        };
        count += source.matches("#[test]").count();
        count += source.matches("#[tokio::test]").count();
    }
    Ok(count)
}

fn load_crate_test_result(repo: &Repo, name: &str) -> io::Result<String> {
    for path in [
        format!(".edgerun/test-results/{name}.txt"),
        format!("target/edgerun-test-results/{name}.txt"),
    ] {
        if let Ok(result) = git_output(
            &repo.path,
            &["show", &format!("{}:{path}", repo.default_ref)],
        ) {
            return Ok(result.trim().to_string());
        }
    }
    Err(io::Error::new(io::ErrorKind::NotFound, "no test result"))
}

fn related_rfcs(repo: &Repo, crate_name: &str, rel_path: &str) -> io::Result<Vec<RfcLink>> {
    let files = match git_output(
        &repo.path,
        &[
            "ls-tree",
            "-r",
            "--name-only",
            &repo.default_ref,
            "docs/rfc",
        ],
    ) {
        Ok(files) => files,
        Err(_) => return Ok(Vec::new()),
    };
    let mut rfcs = Vec::new();
    for file in files.lines().filter(|file| file.ends_with(".md")) {
        let Ok(text) = git_output(
            &repo.path,
            &["show", &format!("{}:{file}", repo.default_ref)],
        ) else {
            continue;
        };
        if !text.contains(crate_name) && !text.contains(rel_path) {
            continue;
        }
        rfcs.push(RfcLink {
            path: file.to_string(),
            title: first_markdown_heading(&text).unwrap_or_else(|| file.to_string()),
            completeness: checklist_completeness(&text),
        });
    }
    rfcs.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(rfcs)
}

fn first_markdown_heading(text: &str) -> Option<String> {
    text.lines()
        .find_map(|line| line.trim().strip_prefix("# ").map(str::trim))
        .map(ToString::to_string)
}

fn checklist_completeness(text: &str) -> String {
    let done = text.matches("[x]").count() + text.matches("[X]").count();
    let open = text.matches("[ ]").count();
    if done + open == 0 {
        "referenced".to_string()
    } else {
        format!("{done}/{} checklist items", done + open)
    }
}

fn load_gitvisible_paths(repo: &Path, rev: &str) -> io::Result<Vec<String>> {
    let output = match git_output(repo, &["ls-tree", "-r", "--name-only", rev]) {
        Ok(output) => output,
        Err(_) => return Ok(Vec::new()),
    };
    let mut paths = Vec::new();
    for line in output.lines() {
        if let Some(path) = marker_to_public_path(line) {
            add_public_path(&mut paths, &path);
        }
    }
    Ok(paths)
}

fn marker_to_public_path(path: &str) -> Option<String> {
    if path == DIR_VISIBILITY_MARKER {
        return None;
    }
    if let Some(parent) = path.strip_suffix(&format!("/{DIR_VISIBILITY_MARKER}")) {
        return Some(parent.to_string());
    }
    path.strip_suffix(FILE_VISIBILITY_SUFFIX)
        .map(|value| value.trim_end_matches('.').to_string())
        .filter(|value| !value.is_empty())
}

fn filter_tree_entries(repo: &Repo, rel_path: &str, entries: Vec<TreeEntry>) -> Vec<TreeEntry> {
    entries
        .into_iter()
        .filter(|entry| !is_visibility_marker(&entry.name))
        .filter(|entry| {
            let path = join_repo_path(rel_path, &entry.name);
            path_is_public(repo, &path)
                || repo
                    .public_paths
                    .iter()
                    .any(|allowed| allowed.starts_with(&format!("{path}/")))
        })
        .collect()
}

fn path_is_public(repo: &Repo, rel_path: &str) -> bool {
    if rel_path.is_empty() {
        return true;
    }
    if repo.public_paths.is_empty() {
        return false;
    }
    repo.public_paths
        .iter()
        .any(|allowed| rel_path == allowed || rel_path.starts_with(&format!("{allowed}/")))
}

fn is_visibility_marker(name: &str) -> bool {
    name == DIR_VISIBILITY_MARKER || name.ends_with(FILE_VISIBILITY_SUFFIX)
}

fn path_is_public_or_ancestor(repo: &Repo, rel_path: &str) -> bool {
    path_is_public(repo, rel_path)
        || repo
            .public_paths
            .iter()
            .any(|allowed| allowed.starts_with(&format!("{rel_path}/")))
}

fn git_output(repo: &Path, args: &[&str]) -> io::Result<String> {
    let bytes = git_output_bytes(repo, args)?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

fn git_output_bytes(repo: &Path, args: &[&str]) -> io::Result<Vec<u8>> {
    let output = Command::new("git")
        .arg("-c")
        .arg(format!("safe.directory={}", repo.display()))
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ))
    }
}

fn is_git_repo(path: &Path) -> bool {
    path.join(".git").exists() || (path.join("objects").exists() && path.join("HEAD").exists())
}

fn render_index(config: &GitConfig, repos: &[Repo]) -> String {
    let mut items = String::new();
    for repo in repos {
        items.push_str(&format!(
            "<article class=\"repo-card\"><a href=\"/{}/\"><h2>{}</h2><p>{}</p><span>{}</span></a></article>",
            escape_attr(&repo.name),
            escape_html(&repo.title),
            escape_html(&repo.description),
            escape_html(&repo.default_ref)
        ));
    }
    if items.is_empty() {
        items.push_str("<p class=\"empty\">No repositories are public yet.</p>");
    }
    page_shell(
        config,
        &config.title,
        &config.description,
        &format!(
            "<main id=\"content\"><section class=\"hero\"><p class=\"eyebrow\">Released code</p><h1>{}</h1><p>{}</p></section><section class=\"repos\" aria-label=\"Repositories\">{}</section></main>",
            escape_html(&config.title),
            escape_html(&config.description),
            items
        ),
    )
}

fn render_crates_index(config: &GitConfig, repo: &Repo, crates: &[CrateInfo]) -> String {
    let items = if crates.is_empty() {
        "<p class=\"empty\">No crates are public yet.</p>".to_string()
    } else {
        crates
            .iter()
            .map(|info| {
                format!(
                    "<article class=\"repo-card crate-card\" data-crate-search=\"{} {} {}\"><a href=\"/{}/crates/{}\"><h2>{}</h2><p>{}</p><span>{} features · {} API items · {} calls · {} tests</span></a></article>",
                    escape_attr(&info.name),
                    escape_attr(&info.description),
                    escape_attr(&info.workspace_deps.join(" ")),
                    escape_attr(&repo.name),
                    escape_attr(&info.name),
                    escape_html(&info.name),
                    escape_html(&info.description),
                    info.features.len(),
                    info.api_items.len(),
                    info.call_edges.len(),
                    info.test_count
                )
            })
            .collect::<Vec<_>>()
            .join("")
    };
    page_shell(
        config,
        &format!("{} crates | {}", repo.title, config.title),
        &repo.description,
        &format!(
            "<main id=\"content\"><nav class=\"crumbs\"><a href=\"/\">Repositories</a><span>/</span><a href=\"/{}/\">{}</a></nav><section class=\"hero\"><p class=\"eyebrow\">Crate explorer</p><h1>{} crates</h1><p>Workspace crates that have been released through this repository's visibility policy.</p><form class=\"crate-search\" role=\"search\"><label for=\"crate-search\">Search crates</label><input id=\"crate-search\" type=\"search\" placeholder=\"Search crates\" autocomplete=\"off\"><span id=\"crate-search-count\">{} crates</span></form></section><section class=\"repos\" aria-label=\"Crates\">{}</section></main>",
            escape_attr(&repo.name),
            escape_html(&repo.title),
            escape_html(&repo.title),
            crates.len(),
            items
        ),
    )
}

fn render_crate_page(
    config: &GitConfig,
    repo: &Repo,
    info: &CrateInfo,
    crates: &[CrateInfo],
) -> String {
    let features = render_pills(&info.features, "No declared features.");
    let deps = render_crate_links(repo, &info.workspace_deps);
    let dependents = render_crate_links(repo, &info.dependents);
    let api = render_api_items(repo, &repo.default_ref, info);
    let call_graph = render_call_graph(repo, &repo.default_ref, info);
    let rfcs = render_rfc_links(repo, &repo.default_ref, &info.rfcs);
    let tree = format!(
        "<ol class=\"dependency-tree\">{}</ol>",
        render_dependency_tree(repo, info, crates, 0, &mut Vec::new())
    );
    let embed_url = absolute_site_url(config, &crate_component_path(repo, &info.name));
    let embed_title = format!("{} crate status", info.name);
    let embed_code = format!(
        "<iframe src=\"{}\" title=\"{}\" loading=\"lazy\"></iframe>",
        embed_url, embed_title
    );
    let vulnerability_href = vulnerability_report_href(&info.name);
    let result = info
        .test_result
        .as_deref()
        .map(|value| escape_html(value))
        .unwrap_or_else(|| "No recorded test run yet.".to_string());
    page_shell(
        config,
        &format!("{} | {}", info.name, config.title),
        &info.description,
        &format!(
            "<main id=\"content\" class=\"repo crate-page\"><nav class=\"crumbs\"><a href=\"/\">Repositories</a><span>/</span><a href=\"/{}/\">{}</a><span>/</span><a href=\"/{}/crates\">crates</a></nav><header class=\"repo-head\"><div><p class=\"eyebrow\">Workspace crate</p><h1>{}</h1><p>{}</p></div><nav class=\"repo-actions\" aria-label=\"Crate actions\"><a class=\"commit-link\" href=\"/{}/src/{}/{}\">Source</a><a class=\"commit-link\" href=\"{}\">Report vulnerability</a></nav></header><nav class=\"section-nav\" aria-label=\"Crate sections\"><a href=\"#api\">API</a><a href=\"#calls\">Calls</a><a href=\"#related\">Related</a><a href=\"#tests\">Tests</a><a href=\"#rfcs\">RFCs</a><a href=\"#embed\">Embed</a></nav><section class=\"crate-grid\"><article class=\"crate-panel\"><h2>Features</h2>{}</article><article class=\"crate-panel\" id=\"related\"><h2>Related crates</h2><h3>Depends on</h3>{}<h3>Used by</h3>{}</article><article class=\"crate-panel\" id=\"tests\"><h2>Tests</h2><p><strong>{}</strong> test declarations found.</p><pre class=\"commit\"><code>{}</code></pre></article><article class=\"crate-panel\" id=\"rfcs\"><h2>Related RFCs</h2>{}</article><article class=\"crate-panel wide\"><h2>Dependency tree</h2>{}</article><article class=\"crate-panel wide\" id=\"api\"><h2>API surface</h2>{}</article><article class=\"crate-panel wide\" id=\"calls\"><h2>Call graph</h2>{}</article><article class=\"crate-panel wide\" id=\"embed\"><h2>Embeddable status</h2><p class=\"muted\">Use this iframe in build-log posts when the post should point at the live crate surface.</p><pre class=\"commit\"><code>{}</code></pre></article></section></main>",
            escape_attr(&repo.name),
            escape_html(&repo.title),
            escape_attr(&repo.name),
            escape_html(&info.name),
            escape_html(&info.description),
            escape_attr(&repo.name),
            escape_attr(&repo.default_ref),
            escape_attr(&info.rel_path),
            escape_attr(&vulnerability_href),
            features,
            deps,
            dependents,
            info.test_count,
            result,
            rfcs,
            tree,
            api,
            call_graph,
            escape_html(&embed_code)
        ),
    )
}

fn render_crate_component_document(config: &GitConfig, repo: &Repo, info: &CrateInfo) -> String {
    let path = crate_component_path(repo, &info.name);
    let canonical = absolute_site_url(config, &path);
    let style = git_style();
    let body = render_crate_component_card(repo, info);
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><meta name=\"color-scheme\" content=\"light dark\"><meta name=\"theme-color\" content=\"#146c63\"><meta name=\"referrer\" content=\"strict-origin-when-cross-origin\"><title>{}</title><meta name=\"description\" content=\"{}\"><link rel=\"canonical\" href=\"{}\"><style>{}</style></head><body class=\"component-body\">{}</body></html>",
        escape_html(&format!("{} crate status", info.name)),
        escape_attr(&info.description),
        escape_attr(&canonical),
        style,
        body
    )
}

fn render_crate_component_card(repo: &Repo, info: &CrateInfo) -> String {
    let test_result = crate_test_summary(info);
    format!(
        "<article class=\"crate-component\" aria-label=\"{} crate status\"><div><p class=\"eyebrow\">Workspace crate</p><h1>{}</h1><p>{}</p></div><dl class=\"component-stats\"><div><dt>Features</dt><dd>{}</dd></div><div><dt>API</dt><dd>{}</dd></div><div><dt>Calls</dt><dd>{}</dd></div><div><dt>Tests</dt><dd>{}</dd></div><div><dt>Depends on</dt><dd>{}</dd></div><div><dt>Used by</dt><dd>{}</dd></div></dl><p class=\"component-result\">{}</p><p class=\"component-actions\"><a target=\"_top\" href=\"/{}/crates/{}\">Open crate page</a><a target=\"_top\" href=\"/{}/src/{}/{}\">Source</a></p></article>",
        escape_attr(&info.name),
        escape_html(&info.name),
        escape_html(&info.description),
        info.features.len(),
        info.api_items.len(),
        info.call_edges.len(),
        info.test_count,
        info.workspace_deps.len(),
        info.dependents.len(),
        escape_html(&test_result),
        escape_attr(&repo.name),
        escape_attr(&info.name),
        escape_attr(&repo.name),
        escape_attr(&repo.default_ref),
        escape_attr(&info.rel_path)
    )
}

fn crate_test_summary(info: &CrateInfo) -> String {
    info.test_result
        .as_deref()
        .and_then(|result| result.lines().find(|line| !line.trim().is_empty()))
        .map(|line| line.trim().to_string())
        .unwrap_or_else(|| "No recorded test run yet.".to_string())
}

fn crate_component_path(repo: &Repo, crate_name: &str) -> String {
    format!("/{}/components/crates/{}.html", repo.name, crate_name)
}

fn absolute_site_url(config: &GitConfig, path: &str) -> String {
    if config.base_url.is_empty() {
        path.to_string()
    } else {
        format!("{}{}", config.base_url.trim_end_matches('/'), path)
    }
}

fn vulnerability_report_href(crate_name: &str) -> String {
    format!(
        "mailto:ken@edgerun.tech?subject={}&body={}",
        query_escape(&format!("Security report for {crate_name}")),
        query_escape("Please describe the affected crate, impact, reproduction steps, and whether this should be handled privately before public disclosure.")
    )
}

fn render_pills(values: &[String], empty: &str) -> String {
    if values.is_empty() {
        return format!("<p class=\"empty\">{}</p>", escape_html(empty));
    }
    format!(
        "<div class=\"pills\">{}</div>",
        values
            .iter()
            .map(|value| format!("<span>{}</span>", escape_html(value)))
            .collect::<Vec<_>>()
            .join("")
    )
}

fn render_crate_links(repo: &Repo, names: &[String]) -> String {
    if names.is_empty() {
        return "<p class=\"empty\">None yet.</p>".to_string();
    }
    format!(
        "<ul class=\"link-list\">{}</ul>",
        names
            .iter()
            .map(|name| format!(
                "<li><a href=\"/{}/crates/{}\">{}</a></li>",
                escape_attr(&repo.name),
                escape_attr(name),
                escape_html(name)
            ))
            .collect::<Vec<_>>()
            .join("")
    )
}

fn render_api_items(repo: &Repo, rev: &str, info: &CrateInfo) -> String {
    if info.api_items.is_empty() {
        return "<p class=\"empty\">No public Rust items found in released source yet.</p>"
            .to_string();
    }
    format!(
        "<ul class=\"api-list\">{}</ul>",
        info.api_items
            .iter()
            .take(80)
            .map(|item| format!(
                "<li><span>{}</span><a href=\"/{}/src/{}/{}#L{}\">{}</a><small>{}:{}</small></li>",
                escape_html(&item.kind),
                escape_attr(&repo.name),
                escape_attr(rev),
                escape_attr(&item.path),
                item.line,
                escape_html(&item.name),
                escape_html(&item.path),
                item.line
            ))
            .collect::<Vec<_>>()
            .join("")
    )
}

fn render_call_graph(repo: &Repo, rev: &str, info: &CrateInfo) -> String {
    if info.call_edges.is_empty() {
        return "<p class=\"empty\">No intra-crate calls found in released source yet.</p>"
            .to_string();
    }
    format!(
        "<ol class=\"call-list\">{}</ol>",
        info.call_edges
            .iter()
            .take(80)
            .map(|edge| format!(
                "<li><a href=\"/{}/src/{}/{}#L{}\">{}</a><span>calls</span><a href=\"/{}/src/{}/{}#L{}\">{}</a><small>{} time{}</small></li>",
                escape_attr(&repo.name),
                escape_attr(rev),
                escape_attr(&edge.caller_path),
                edge.caller_line,
                escape_html(&edge.caller),
                escape_attr(&repo.name),
                escape_attr(rev),
                escape_attr(&edge.callee_path),
                edge.callee_line,
                escape_html(&edge.callee),
                edge.count,
                if edge.count == 1 { "" } else { "s" }
            ))
            .collect::<Vec<_>>()
            .join("")
    )
}

fn render_rfc_links(repo: &Repo, rev: &str, rfcs: &[RfcLink]) -> String {
    if rfcs.is_empty() {
        return "<p class=\"empty\">No RFC mentions found yet.</p>".to_string();
    }
    format!(
        "<ul class=\"link-list\">{}</ul>",
        rfcs.iter()
            .map(|rfc| format!(
                "<li><a href=\"/{}/src/{}/{}\">{}</a><span>{}</span></li>",
                escape_attr(&repo.name),
                escape_attr(rev),
                escape_attr(&rfc.path),
                escape_html(&rfc.title),
                escape_html(&rfc.completeness)
            ))
            .collect::<Vec<_>>()
            .join("")
    )
}

fn render_dependency_tree(
    repo: &Repo,
    info: &CrateInfo,
    crates: &[CrateInfo],
    depth: usize,
    seen: &mut Vec<String>,
) -> String {
    if depth > 5 || seen.iter().any(|name| name == &info.name) {
        return format!(
            "<li>{} <span>cycle or depth limit</span></li>",
            escape_html(&info.name)
        );
    }
    seen.push(info.name.clone());
    let children = info
        .workspace_deps
        .iter()
        .filter_map(|dep| crates.iter().find(|candidate| candidate.name == *dep))
        .map(|dep| render_dependency_tree(repo, dep, crates, depth + 1, seen))
        .collect::<Vec<_>>()
        .join("");
    let _ = seen.pop();
    let child_html = if children.is_empty() {
        String::new()
    } else {
        format!("<ol>{children}</ol>")
    };
    format!(
        "<li><a href=\"/{}/crates/{}\">{}</a>{}</li>",
        escape_attr(&repo.name),
        escape_attr(&info.name),
        escape_html(&info.name),
        child_html
    )
}

fn render_tree(
    config: &GitConfig,
    repo: &Repo,
    rev: &str,
    rel_path: &str,
    entries: &[TreeEntry],
) -> String {
    let mut rows = String::new();
    if !rel_path.is_empty() {
        let parent = parent_path(rel_path);
        rows.push_str(&format!(
            "<li><a href=\"/{}/src/{}/{}\">../</a><span>tree</span></li>",
            escape_attr(&repo.name),
            escape_attr(rev),
            escape_attr(&parent)
        ));
    }
    for entry in entries {
        let path = join_repo_path(rel_path, &entry.name);
        rows.push_str(&format!(
            "<li><a href=\"/{}/src/{}/{}\">{}{}</a><span>{}</span></li>",
            escape_attr(&repo.name),
            escape_attr(rev),
            escape_attr(&path),
            if entry.kind == "tree" { "" } else { "" },
            escape_html(&entry.name),
            escape_html(&entry.kind)
        ));
    }
    page_shell(
        config,
        &format!("{} | {}", repo.title, config.title),
        &repo.description,
        &format!(
            "<main id=\"content\" class=\"repo\"><nav class=\"crumbs\"><a href=\"/\">Repositories</a><span>/</span><a href=\"/{}/\">{}</a>{}</nav><header class=\"repo-head\"><div><p class=\"eyebrow\">{}</p><h1>{}</h1><p>{}</p></div><a class=\"commit-link\" href=\"/{}/commit/{}\">{}</a></header><ol class=\"tree-list\">{}</ol></main>",
            escape_attr(&repo.name),
            escape_html(&repo.title),
            render_path_crumbs(repo, rev, rel_path),
            escape_html(rev),
            escape_html(path_title(rel_path)),
            escape_html(&repo.description),
            escape_attr(&repo.name),
            escape_attr(rev),
            escape_html(rev),
            rows
        ),
    )
}

fn render_blob(config: &GitConfig, repo: &Repo, rev: &str, rel_path: &str, bytes: &[u8]) -> String {
    let body = if bytes.len() > MAX_BLOB_BYTES {
        format!(
            "<p class=\"empty\">File is too large to render inline. <a href=\"/{}/raw/{}/{}\">Open raw</a>.</p>",
            escape_attr(&repo.name),
            escape_attr(rev),
            escape_attr(rel_path)
        )
    } else {
        render_code(bytes)
    };
    page_shell(
        config,
        &format!("{} | {}", rel_path, config.title),
        &repo.description,
        &format!(
            "<main id=\"content\" class=\"repo\"><nav class=\"crumbs\"><a href=\"/\">Repositories</a><span>/</span><a href=\"/{}/\">{}</a>{}</nav><header class=\"repo-head\"><div><p class=\"eyebrow\">{}</p><h1>{}</h1><p>{}</p></div><a class=\"commit-link\" href=\"/{}/raw/{}/{}\">Raw</a></header>{}</main>",
            escape_attr(&repo.name),
            escape_html(&repo.title),
            render_path_crumbs(repo, rev, rel_path),
            escape_html(rev),
            escape_html(rel_path),
            escape_html(&repo.description),
            escape_attr(&repo.name),
            escape_attr(rev),
            escape_attr(rel_path),
            body
        ),
    )
}

fn render_commit(config: &GitConfig, repo: &Repo, rev: &str, commit: &str) -> String {
    page_shell(
        config,
        &format!("{} commit | {}", repo.title, config.title),
        &repo.description,
        &format!(
            "<main id=\"content\" class=\"repo\"><nav class=\"crumbs\"><a href=\"/\">Repositories</a><span>/</span><a href=\"/{}/\">{}</a></nav><header class=\"repo-head\"><div><p class=\"eyebrow\">Commit</p><h1>{}</h1></div><a class=\"commit-link\" href=\"/{}/src/{}\">Browse tree</a></header><pre class=\"commit\"><code>{}</code></pre></main>",
            escape_attr(&repo.name),
            escape_html(&repo.title),
            escape_html(rev),
            escape_attr(&repo.name),
            escape_attr(rev),
            escape_html(commit)
        ),
    )
}

fn render_code(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let mut out = String::from("<table class=\"code\"><tbody>");
    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        out.push_str(&format!(
            "<tr id=\"L{number}\"><th><a href=\"#L{number}\">{number}</a></th><td><code>{}</code></td></tr>",
            escape_html(line)
        ));
    }
    if text.is_empty() {
        out.push_str("<tr><th></th><td><code></code></td></tr>");
    }
    out.push_str("</tbody></table>");
    out
}

fn render_path_crumbs(repo: &Repo, rev: &str, rel_path: &str) -> String {
    if rel_path.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    let mut current = String::new();
    for part in rel_path.split('/') {
        current = join_repo_path(&current, part);
        out.push_str(&format!(
            "<span>/</span><a href=\"/{}/src/{}/{}\">{}</a>",
            escape_attr(&repo.name),
            escape_attr(rev),
            escape_attr(&current),
            escape_html(part)
        ));
    }
    out
}

fn page_shell(config: &GitConfig, title: &str, description: &str, body: &str) -> String {
    let style = git_style();
    edgerun_web_ui::render_page(&PageShell {
        lang: "en",
        title,
        description,
        theme_color: "#146c63",
        generator: "edgerun-git",
        extra_head: "<link rel=\"icon\" href=\"/favicon.svg\" type=\"image/svg+xml\">",
        style: &style,
        brand_href: "/",
        brand_label: "Edgerun Git home",
        brand_text: &config.title,
        header_extra: "<nav aria-label=\"Primary\"><a href=\"https://blog.edgerun.tech/\">Blog</a></nav><nav aria-label=\"Theme\"><er-theme-toggle></er-theme-toggle></nav>",
        footer: "",
        body,
        script_src: Some("/app.js"),
    })
}

fn html_response(body: String) -> Response {
    Response::html(StatusCode::OK, &body)
        .with_header("Cache-Control", "no-store")
        .with_header("X-Content-Type-Options", "nosniff")
}

fn component_response(body: String) -> Response {
    Response::html(StatusCode::OK, &body)
        .with_header("Cache-Control", "public, max-age=60")
        .with_header("X-Content-Type-Options", "nosniff")
}

fn css_response() -> Response {
    Response::new(StatusCode::OK)
        .with_header("Content-Type", "text/css; charset=utf-8")
        .with_header("Cache-Control", "public, max-age=31536000, immutable")
        .with_header("X-Content-Type-Options", "nosniff")
        .with_body(git_style())
}

fn js_response() -> Response {
    Response::new(StatusCode::OK)
        .with_header("Content-Type", "application/javascript; charset=utf-8")
        .with_header("Cache-Control", "public, max-age=31536000, immutable")
        .with_header("X-Content-Type-Options", "nosniff")
        .with_body(git_js())
}

fn favicon_response() -> Response {
    Response::new(StatusCode::OK)
        .with_header("Content-Type", "image/svg+xml")
        .with_header("Cache-Control", "public, max-age=31536000, immutable")
        .with_header("X-Content-Type-Options", "nosniff")
        .with_body(edgerun_web_ui::FAVICON_COMMAND_SVG)
}

fn not_found_response(site_title: &str) -> Response {
    Response::html(
        StatusCode::new(404).unwrap_or(StatusCode::OK),
        &page_shell(
            &GitConfig {
                root: PathBuf::new(),
                bind_addr: String::new(),
                title: site_title.to_string(),
                description: String::new(),
                base_url: String::new(),
            },
            "Not found",
            "The requested page does not exist.",
            "<main id=\"content\" class=\"empty\"><h1>Not found</h1><p>The requested repository, commit, or path is not public.</p></main>",
        ),
    )
    .with_header("Cache-Control", "no-store")
    .with_header("X-Content-Type-Options", "nosniff")
}

fn server_error(error: io::Error) -> Response {
    Response::text(
        StatusCode::new(500).unwrap_or(StatusCode::OK),
        &format!("edgerun-git error: {error}"),
    )
    .with_header("Cache-Control", "no-store")
    .with_header("X-Content-Type-Options", "nosniff")
}

fn route_segments(path: &str) -> io::Result<Vec<String>> {
    let mut out = Vec::new();
    for raw in path.trim_matches('/').split('/') {
        if raw.is_empty() {
            continue;
        }
        let decoded = percent_decode(raw)?;
        if decoded == "." || decoded == ".." || decoded.contains('\\') {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "unsafe path"));
        }
        out.push(decoded);
    }
    Ok(out)
}

fn percent_decode(input: &str) -> io::Result<String> {
    let mut out = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "bad escape"));
            }
            let hex = &input[i + 1..i + 3];
            let value = u8::from_str_radix(hex, 16)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "bad escape"))?;
            out.push(value);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "non-utf8 path"))
}

fn safe_repo_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        && !name.starts_with('.')
}

fn safe_rev(rev: &str) -> bool {
    rev == "HEAD"
        || (!rev.is_empty()
            && rev.len() <= 80
            && rev
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '/' | '.')))
            && !rev.contains("..")
            && !rev.starts_with('-')
}

fn safe_repo_path(path: &str) -> bool {
    if path.starts_with('/') || path.contains('\\') || path.contains('\0') {
        return false;
    }
    Path::new(path)
        .components()
        .all(|part| matches!(part, Component::Normal(_)))
        || path.is_empty()
}

fn trim_quotes(value: &str) -> &str {
    value.trim_matches('"').trim_matches('\'')
}

fn parse_inline_list(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    let inner = trimmed
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(trimmed);
    inner
        .split(',')
        .map(|part| trim_quotes(part.trim()).to_string())
        .filter(|part| !part.is_empty())
        .collect()
}

fn add_public_path(paths: &mut Vec<String>, path: &str) {
    let path = path.trim().trim_matches('/');
    if !path.is_empty() && safe_repo_path(path) && !paths.iter().any(|existing| existing == path) {
        paths.push(path.to_string());
    }
}

fn nonempty(value: String) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn tree_sort_key(kind: &str) -> u8 {
    if kind == "tree" {
        0
    } else {
        1
    }
}

fn join_repo_path(base: &str, child: &str) -> String {
    if base.is_empty() {
        child.to_string()
    } else {
        format!("{base}/{child}")
    }
}

fn parent_path(path: &str) -> String {
    path.rsplit_once('/')
        .map(|(parent, _)| parent.to_string())
        .unwrap_or_default()
}

fn path_title(path: &str) -> &str {
    if path.is_empty() {
        "Files"
    } else {
        path
    }
}

fn escape_html(input: &str) -> String {
    edgerun_web_ui::escape_html(input)
}

fn escape_attr(input: &str) -> String {
    edgerun_web_ui::escape_attr(input)
}

fn query_escape(input: &str) -> String {
    let mut out = String::new();
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn to_io_error(error: edgerun_http::io::Error) -> io::Error {
    io::Error::other(error.to_string())
}

fn git_style() -> String {
    format!("{}\n{}", edgerun_web_ui::BASE_STYLE, GIT_STYLE)
}

fn git_js() -> String {
    format!("{}\n{}", edgerun_web_ui::THEME_TOGGLE_JS, GIT_JS)
}

const GIT_JS: &str = r#"
const crateSearch=document.getElementById('crate-search');
const crateCards=[...document.querySelectorAll('[data-crate-search]')];
const crateCount=document.getElementById('crate-search-count');
function applyCrateSearch(value){const query=value.trim().toLowerCase();let shown=0;for(const card of crateCards){const match=!query||card.dataset.crateSearch.toLowerCase().includes(query);card.hidden=!match;if(match)shown++}if(crateCount){crateCount.textContent=shown+' crates'}}
if(crateSearch){crateSearch.addEventListener('input',()=>applyCrateSearch(crateSearch.value))}
"#;

const GIT_STYLE: &str = r#"
.repos{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:16px;max-width:1180px;margin:0 auto;padding:34px 18px 80px}.repo-card{background:var(--panel);border:1px solid var(--line);border-radius:8px}.repo-card a{display:block;min-height:180px;padding:22px;text-decoration:none}.repo-card h2{margin:0 0 10px;font-size:26px;line-height:1.15}.repo-card p{color:var(--muted)}.repo-card span,.commit-link{color:var(--accent);font-weight:800}.crate-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:16px}.crate-panel{background:var(--panel);border:1px solid var(--line);border-radius:8px;padding:18px}.crate-panel h2{margin:0 0 12px;font-size:22px}.crate-panel h3{margin:16px 0 8px;font-size:15px;color:var(--muted);text-transform:uppercase;letter-spacing:.08em}.crate-panel.wide{grid-column:1/-1}.pills{display:flex;flex-wrap:wrap;gap:8px}.pills span{border:1px solid var(--line);border-radius:999px;padding:4px 9px;color:var(--muted)}.link-list{display:grid;gap:8px;margin:0;padding-left:18px}.link-list a{color:var(--accent);font-weight:750;text-decoration:none}.link-list span{display:block;color:var(--muted)}.crate-panel ol{margin:8px 0 0 22px}.dependency-tree{padding-left:20px}.crate-panel li{margin:5px 0}.crate-panel li a{color:var(--accent);font-weight:750;text-decoration:none}.repo{max-width:1180px;margin:0 auto;padding:34px 18px 80px}.crumbs{display:flex;gap:8px;flex-wrap:wrap;color:var(--muted);margin-bottom:18px}.crumbs a{color:var(--accent);text-decoration:none}.repo-head{display:flex;justify-content:space-between;gap:18px;align-items:flex-start;margin-bottom:22px}.repo-head h1{margin:0;font-size:clamp(32px,5vw,54px);line-height:1;letter-spacing:0}.repo-head p{color:var(--muted)}.commit-link{border:1px solid var(--line);border-radius:8px;padding:9px 12px;text-decoration:none;background:var(--panel);white-space:nowrap}.tree-list{list-style:none;margin:0;padding:0;border:1px solid var(--line);border-radius:8px;overflow:hidden;background:var(--panel)}.tree-list li{display:grid;grid-template-columns:1fr 90px;gap:12px;padding:10px 14px;border-top:1px solid var(--line)}.tree-list li:first-child{border-top:0}.tree-list a{text-decoration:none;font-weight:700}.tree-list span{color:var(--muted)}.code{width:100%;border-collapse:collapse;background:var(--panel);border:1px solid var(--line);border-radius:8px;overflow:hidden;display:block}.code tbody{display:table;width:100%}.code tr:target{background:color-mix(in srgb,var(--accent) 14%,transparent)}.code th{width:1%;min-width:54px;padding:0 12px;text-align:right;color:var(--muted);border-right:1px solid var(--line);user-select:none}.code th a{text-decoration:none;color:inherit}.code td{padding:0 12px;white-space:pre;overflow:auto}.code code,.commit code{font-family:ui-monospace,SFMono-Regular,Consolas,monospace;font-size:14px}.commit{background:var(--panel);border:1px solid var(--line);border-radius:8px;padding:16px;overflow:auto}.empty{max-width:720px;margin:80px auto;padding:0 18px;color:var(--muted)}@media(max-width:760px){.repos,.crate-grid{grid-template-columns:1fr}.repo-head{display:block}.commit-link{display:inline-block;margin-top:8px}}
.crate-search{display:grid;grid-template-columns:minmax(220px,420px) max-content;gap:10px;align-items:center;max-width:620px;margin-top:22px}.crate-search label{position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap}.crate-search input{min-width:0;height:44px;border:1px solid var(--line);border-radius:8px;background:var(--panel);color:var(--text);padding:10px 13px;font:inherit}.crate-search span{color:var(--muted);font-weight:750}.repo-actions{display:flex;gap:10px;flex-wrap:wrap;justify-content:flex-end}.section-nav{display:flex;gap:8px;flex-wrap:wrap;margin:-8px 0 22px}.section-nav a{border:1px solid var(--line);border-radius:999px;padding:5px 10px;color:var(--muted);font-weight:750;text-decoration:none}.section-nav a:hover{border-color:var(--accent);color:var(--accent)}.api-list,.call-list{display:grid;gap:8px;list-style:none;margin:0;padding:0}.api-list li{display:grid;grid-template-columns:70px minmax(0,1fr) minmax(0,1.4fr);gap:10px;align-items:baseline;border-bottom:1px solid var(--line);padding:7px 0}.api-list span,.call-list span{color:var(--muted);font-size:12px;font-weight:800;text-transform:uppercase}.api-list a,.call-list a{color:var(--accent);font-weight:800;text-decoration:none}.api-list small,.call-list small{color:var(--muted);overflow-wrap:anywhere}.call-list li{display:grid;grid-template-columns:minmax(0,1fr) 54px minmax(0,1fr) 80px;gap:10px;align-items:baseline;border-bottom:1px solid var(--line);padding:7px 0}@media(max-width:760px){.crate-search{grid-template-columns:1fr}.repo-actions{justify-content:flex-start}.api-list li,.call-list li{grid-template-columns:1fr}.api-list span,.call-list span{font-size:11px}}
.component-body{margin:0;padding:0;background:transparent}.crate-component{min-height:100vh;border:1px solid var(--line);border-radius:8px;background:var(--panel);padding:18px}.crate-component h1{margin:0;font-size:28px;line-height:1.05;letter-spacing:0}.crate-component p{margin:8px 0;color:var(--muted)}.component-stats{display:grid;grid-template-columns:repeat(auto-fit,minmax(86px,1fr));gap:8px;margin:16px 0}.component-stats div{border:1px solid var(--line);border-radius:8px;padding:8px;background:color-mix(in srgb,var(--panel) 82%,var(--code))}.component-stats dt{font-size:11px;font-weight:800;text-transform:uppercase;letter-spacing:.08em;color:var(--muted)}.component-stats dd{margin:2px 0 0;font-weight:800}.component-result{border-left:3px solid var(--accent);padding-left:10px}.component-actions{display:flex;gap:12px;flex-wrap:wrap}.component-actions a{color:var(--accent);font-weight:800;text-decoration:none}
"#;
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_by_default() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("edgerun-git-test-{stamp}"));
        let repo = base.join("demo");
        fs::create_dir_all(repo.join(".git")).unwrap();
        assert!(load_repo("demo".to_string(), repo).unwrap().is_none());
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn recognizes_bare_repo_layout() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("edgerun-git-bare-test-{stamp}"));
        let repo = base.join("demo.git");
        fs::create_dir_all(repo.join("objects")).unwrap();
        fs::write(repo.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        assert!(is_git_repo(&repo));
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn parses_in_repo_visibility_config() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("edgerun-git-config-test-{stamp}"));
        fs::create_dir_all(base.join(".edgerun")).unwrap();
        fs::write(
            base.join(".edgerun/git.yaml"),
            "visible: true\ntitle: Demo\ndescription: Released feature slice\ndefault_ref: abc123\npaths:\n  - README.md\n  - crates/edgerun-email\n",
        )
        .unwrap();
        let config = load_repo_page_config(&base).unwrap();
        assert!(config.visible);
        assert_eq!(config.title.as_deref(), Some("Demo"));
        assert_eq!(
            config.description.as_deref(),
            Some("Released feature slice")
        );
        assert_eq!(config.default_ref.as_deref(), Some("abc123"));
        assert_eq!(config.public_paths, ["README.md", "crates/edgerun-email"]);
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn filters_tree_entries_to_public_paths() {
        let repo = Repo {
            name: "demo".to_string(),
            path: PathBuf::new(),
            title: "Demo".to_string(),
            description: String::new(),
            default_ref: "HEAD".to_string(),
            public_paths: vec!["README.md".to_string(), "crates/edgerun-email".to_string()],
        };
        let entries = vec![
            TreeEntry {
                mode: "100644".to_string(),
                kind: "blob".to_string(),
                hash: "a".to_string(),
                name: "README.md".to_string(),
            },
            TreeEntry {
                mode: "040000".to_string(),
                kind: "tree".to_string(),
                hash: "b".to_string(),
                name: "crates".to_string(),
            },
            TreeEntry {
                mode: "040000".to_string(),
                kind: "tree".to_string(),
                hash: "c".to_string(),
                name: "private".to_string(),
            },
        ];
        let names = filter_tree_entries(&repo, "", entries)
            .into_iter()
            .map(|entry| entry.name)
            .collect::<Vec<_>>();
        assert_eq!(names, ["README.md", "crates"]);
        assert!(path_is_public(&repo, "crates/edgerun-email/src/lib.rs"));
        assert!(!path_is_public(&repo, "crates/edgerun-server/src/lib.rs"));
    }

    #[test]
    fn maps_gitvisible_markers_to_public_paths() {
        assert_eq!(
            marker_to_public_path("crates/edgerun-git/.gitvisible").as_deref(),
            Some("crates/edgerun-git")
        );
        assert_eq!(
            marker_to_public_path("README.md.gitvisible").as_deref(),
            Some("README.md")
        );
        assert!(marker_to_public_path(".gitvisible").is_none());
        assert!(is_visibility_marker(".gitvisible"));
        assert!(is_visibility_marker("README.md.gitvisible"));
    }

    #[test]
    fn empty_public_paths_hide_tree_entries() {
        let repo = Repo {
            name: "demo".to_string(),
            path: PathBuf::new(),
            title: "Demo".to_string(),
            description: String::new(),
            default_ref: "HEAD".to_string(),
            public_paths: Vec::new(),
        };
        let entries = vec![TreeEntry {
            mode: "100644".to_string(),
            kind: "blob".to_string(),
            hash: "a".to_string(),
            name: "README.md".to_string(),
        }];
        assert!(filter_tree_entries(&repo, "", entries).is_empty());
        assert!(!path_is_public(&repo, "README.md"));
        assert!(path_is_public(&repo, ""));
    }

    #[test]
    fn loads_gitvisible_markers_from_git_tree() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("edgerun-git-marker-test-{stamp}"));
        let repo_path = base.join("demo");
        fs::create_dir_all(repo_path.join(".edgerun")).unwrap();
        fs::create_dir_all(repo_path.join("src")).unwrap();
        fs::write(
            repo_path.join(".edgerun/git.yaml"),
            "visible: true\ntitle: Demo\n",
        )
        .unwrap();
        fs::write(repo_path.join("README.md"), "public").unwrap();
        fs::write(repo_path.join("README.md.gitvisible"), "").unwrap();
        fs::write(repo_path.join("src/lib.rs"), "pub fn demo() {}\n").unwrap();
        fs::write(repo_path.join("src/.gitvisible"), "").unwrap();
        git_ok(&repo_path, &["init"]);
        git_ok(&repo_path, &["config", "user.email", "test@example.com"]);
        git_ok(&repo_path, &["config", "user.name", "Test"]);
        git_ok(&repo_path, &["add", "."]);
        git_ok(&repo_path, &["commit", "-m", "init"]);

        let repo = load_repo("demo".to_string(), repo_path).unwrap().unwrap();
        assert_eq!(repo.public_paths, ["README.md", "src"]);
        assert!(path_is_public(&repo, "src/lib.rs"));
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn visible_crates_require_public_crate_directory() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("edgerun-git-crate-test-{stamp}"));
        let repo_path = base.join("demo");
        fs::create_dir_all(repo_path.join(".edgerun")).unwrap();
        fs::create_dir_all(repo_path.join("crates/edgerun-demo/src")).unwrap();
        fs::create_dir_all(repo_path.join("crates/edgerun-hidden/src")).unwrap();
        fs::write(repo_path.join(".edgerun/git.yaml"), "visible: true\n").unwrap();
        fs::write(
            repo_path.join("crates/edgerun-demo/Cargo.toml"),
            "[package]\nname = \"edgerun-demo\"\ndescription = \"Demo crate\"\n[features]\ndefault = []\nstd = []\n[dependencies]\nedgerun-http = { path = \"../edgerun-http\" }\n",
        )
        .unwrap();
        fs::write(
            repo_path.join("crates/edgerun-demo/src/lib.rs"),
            "pub struct Demo;\npub async fn run_demo() { helper(); }\nfn helper() {}\n#[test]\nfn demo() {}\n",
        )
        .unwrap();
        fs::write(repo_path.join("crates/edgerun-demo/.gitvisible"), "").unwrap();
        fs::write(
            repo_path.join("crates/edgerun-hidden/Cargo.toml"),
            "[package]\nname = \"edgerun-hidden\"\n",
        )
        .unwrap();
        git_ok(&repo_path, &["init"]);
        git_ok(&repo_path, &["config", "user.email", "test@example.com"]);
        git_ok(&repo_path, &["config", "user.name", "Test"]);
        git_ok(&repo_path, &["add", "."]);
        git_ok(&repo_path, &["commit", "-m", "init"]);

        let repo = load_repo("demo".to_string(), repo_path).unwrap().unwrap();
        let crates = visible_crates(&repo).unwrap();
        assert_eq!(crates.len(), 1);
        assert_eq!(crates[0].name, "edgerun-demo");
        assert_eq!(crates[0].features, ["default", "std"]);
        assert_eq!(crates[0].api_items.len(), 2);
        assert_eq!(crates[0].call_edges.len(), 1);
        assert_eq!(crates[0].call_edges[0].caller, "run_demo");
        assert_eq!(crates[0].call_edges[0].callee, "helper");
        assert_eq!(crates[0].test_count, 1);
        let mut config = GitConfig::new(base.join("repos"));
        config.base_url = "https://git.example.test".to_string();
        let html = render_crate_page(&config, &repo, &crates[0], &crates);
        assert!(html.contains("Workspace crate"));
        assert!(html.contains("API surface"));
        assert!(html.contains("Call graph"));
        assert!(html.contains("run_demo"));
        assert!(html.contains("helper"));
        assert!(html.contains("Report vulnerability"));
        assert!(html.contains("Security%20report%20for%20edgerun-demo"));
        assert!(html.contains("edgerun-demo"));
        assert!(!html.contains("edgerun-hidden"));
        assert!(html.contains("https://git.example.test/demo/components/crates/edgerun-demo.html"));
        let component = render_crate_component_document(&config, &repo, &crates[0]);
        assert!(component.contains("edgerun-demo crate status"));
        assert!(component.contains("<dt>API</dt><dd>2</dd>"));
        assert!(component.contains("<dt>Calls</dt><dd>1</dd>"));
        assert!(component.contains("Open crate page"));
        assert!(component.contains("target=\"_top\""));
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn generated_crate_metadata_check_detects_stale_files() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("edgerun-git-generate-test-{stamp}"));
        let repo_path = base.join("demo");
        let out = repo_path.join(".edgerun/git/crates");
        fs::create_dir_all(repo_path.join("crates/edgerun-demo/src")).unwrap();
        fs::write(repo_path.join("crates/edgerun-demo/.gitvisible"), "").unwrap();
        fs::write(
            repo_path.join("crates/edgerun-demo/Cargo.toml"),
            "[package]\nname = \"edgerun-demo\"\n",
        )
        .unwrap();
        fs::write(
            repo_path.join("crates/edgerun-demo/src/lib.rs"),
            "pub fn run_demo() { helper(); }\nfn helper() {}\n",
        )
        .unwrap();

        assert_eq!(generate_crate_metadata(&repo_path, &out).unwrap(), 1);
        assert_eq!(check_crate_metadata(&repo_path, &out).unwrap(), 1);

        fs::write(
            repo_path.join("crates/edgerun-demo/src/lib.rs"),
            "pub fn run_demo() {}\n",
        )
        .unwrap();
        let error = check_crate_metadata(&repo_path, &out).unwrap_err();
        assert!(error.to_string().contains("stale generated crate metadata"));
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn parses_public_api_from_tokens() {
        let tokens = rust_tokens(
            "pub fn send_mail() {}\npub(crate) struct Mailbox;\npub static_root: Option<PathBuf>;\nfn private() {}\n",
        );
        let items = parse_api_items("src/lib.rs", &tokens)
            .into_iter()
            .map(|item| (item.kind, item.name))
            .collect::<Vec<_>>();
        assert_eq!(
            items,
            [
                ("fn".to_string(), "send_mail".to_string()),
                ("struct".to_string(), "Mailbox".to_string())
            ]
        );
    }

    #[test]
    fn parses_functions_and_calls_from_tokens() {
        let metadata = analyze_rust_sources(vec![RustSource {
            path: "src/lib.rs".to_string(),
            text: "pub async fn run_demo() { let _ = \"helper()\"; helper(); }\nfn helper(value: u8) -> u8 { value }\nfn other_helper() {}\n"
                .to_string(),
        }]);
        assert!(metadata
            .call_edges
            .iter()
            .any(|edge| edge.caller == "run_demo" && edge.callee == "helper"));
        assert!(!metadata
            .call_edges
            .iter()
            .any(|edge| edge.caller == "run_demo" && edge.callee == "other_helper"));
    }

    #[test]
    fn rejects_unsafe_routes() {
        assert!(!safe_repo_name("../x"));
        assert!(!safe_repo_path("../secret"));
        assert!(!safe_rev("../main"));
        assert!(safe_repo_path("crates/edgerun-git/src/lib.rs"));
        assert!(safe_rev("a3747419"));
    }

    #[test]
    fn renders_line_anchors() {
        let html = render_code(b"one\ntwo\n");
        assert!(html.contains("id=\"L1\""));
        assert!(html.contains("href=\"#L2\""));
        assert!(html.contains("<code>two</code>"));
    }

    fn git_ok(repo: &Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(args)
            .status()
            .unwrap();
        assert!(status.success(), "git command failed: {:?}", args);
    }
}
