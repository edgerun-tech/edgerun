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
    workspace_deps: Vec<String>,
    dependents: Vec<String>,
    test_count: usize,
    test_result: Option<String>,
    rfcs: Vec<RfcLink>,
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
    let workspace_deps = workspace_dependency_names(manifest);
    let test_count = count_crate_tests(repo, rel_path)?;
    let test_result = load_crate_test_result(repo, &name).ok();
    let rfcs = related_rfcs(repo, &name, rel_path)?;
    Ok(CrateInfo {
        name,
        rel_path: rel_path.to_string(),
        description,
        features,
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
                    "<article class=\"repo-card crate-card\"><a href=\"/{}/crates/{}\"><h2>{}</h2><p>{}</p><span>{} features · {} tests</span></a></article>",
                    escape_attr(&repo.name),
                    escape_attr(&info.name),
                    escape_html(&info.name),
                    escape_html(&info.description),
                    info.features.len(),
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
            "<main id=\"content\"><nav class=\"crumbs\"><a href=\"/\">Repositories</a><span>/</span><a href=\"/{}/\">{}</a></nav><section class=\"hero\"><p class=\"eyebrow\">Crate explorer</p><h1>{} crates</h1><p>Workspace crates that have been released through this repository's visibility policy.</p></section><section class=\"repos\" aria-label=\"Crates\">{}</section></main>",
            escape_attr(&repo.name),
            escape_html(&repo.title),
            escape_html(&repo.title),
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
    let rfcs = render_rfc_links(repo, &repo.default_ref, &info.rfcs);
    let tree = render_dependency_tree(repo, info, crates, 0, &mut Vec::new());
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
            "<main id=\"content\" class=\"repo crate-page\"><nav class=\"crumbs\"><a href=\"/\">Repositories</a><span>/</span><a href=\"/{}/\">{}</a><span>/</span><a href=\"/{}/crates\">crates</a></nav><header class=\"repo-head\"><div><p class=\"eyebrow\">Workspace crate</p><h1>{}</h1><p>{}</p></div><a class=\"commit-link\" href=\"/{}/src/{}/{}\">Source</a></header><section class=\"crate-grid\"><article class=\"crate-panel\"><h2>Features</h2>{}</article><article class=\"crate-panel\"><h2>Related crates</h2><h3>Depends on</h3>{}<h3>Used by</h3>{}</article><article class=\"crate-panel\"><h2>Tests</h2><p><strong>{}</strong> test declarations found.</p><pre class=\"commit\"><code>{}</code></pre></article><article class=\"crate-panel\"><h2>Related RFCs</h2>{}</article><article class=\"crate-panel wide\"><h2>Dependency tree</h2>{}</article></section></main>",
            escape_attr(&repo.name),
            escape_html(&repo.title),
            escape_attr(&repo.name),
            escape_html(&info.name),
            escape_html(&info.description),
            escape_attr(&repo.name),
            escape_attr(&repo.default_ref),
            escape_attr(&info.rel_path),
            features,
            deps,
            dependents,
            info.test_count,
            result,
            rfcs,
            tree
        ),
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
        .with_body(edgerun_web_ui::THEME_TOGGLE_JS)
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

fn to_io_error(error: edgerun_http::io::Error) -> io::Error {
    io::Error::other(error.to_string())
}

fn git_style() -> String {
    format!("{}\n{}", edgerun_web_ui::BASE_STYLE, GIT_STYLE)
}

const GIT_STYLE: &str = r#"
.repos{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:16px;max-width:1180px;margin:0 auto;padding:34px 18px 80px}.repo-card{background:var(--panel);border:1px solid var(--line);border-radius:8px}.repo-card a{display:block;min-height:180px;padding:22px;text-decoration:none}.repo-card h2{margin:0 0 10px;font-size:26px;line-height:1.15}.repo-card p{color:var(--muted)}.repo-card span,.commit-link{color:var(--accent);font-weight:800}.crate-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:16px}.crate-panel{background:var(--panel);border:1px solid var(--line);border-radius:8px;padding:18px}.crate-panel h2{margin:0 0 12px;font-size:22px}.crate-panel h3{margin:16px 0 8px;font-size:15px;color:var(--muted);text-transform:uppercase;letter-spacing:.08em}.crate-panel.wide{grid-column:1/-1}.pills{display:flex;flex-wrap:wrap;gap:8px}.pills span{border:1px solid var(--line);border-radius:999px;padding:4px 9px;color:var(--muted)}.link-list{display:grid;gap:8px;margin:0;padding-left:18px}.link-list a{color:var(--accent);font-weight:750;text-decoration:none}.link-list span{display:block;color:var(--muted)}.crate-panel ol{margin:8px 0 0 22px}.crate-panel li{margin:5px 0}.crate-panel li a{color:var(--accent);font-weight:750;text-decoration:none}.repo{max-width:1180px;margin:0 auto;padding:34px 18px 80px}.crumbs{display:flex;gap:8px;flex-wrap:wrap;color:var(--muted);margin-bottom:18px}.crumbs a{color:var(--accent);text-decoration:none}.repo-head{display:flex;justify-content:space-between;gap:18px;align-items:flex-start;margin-bottom:22px}.repo-head h1{margin:0;font-size:clamp(32px,5vw,54px);line-height:1;letter-spacing:0}.repo-head p{color:var(--muted)}.commit-link{border:1px solid var(--line);border-radius:8px;padding:9px 12px;text-decoration:none;background:var(--panel);white-space:nowrap}.tree-list{list-style:none;margin:0;padding:0;border:1px solid var(--line);border-radius:8px;overflow:hidden;background:var(--panel)}.tree-list li{display:grid;grid-template-columns:1fr 90px;gap:12px;padding:10px 14px;border-top:1px solid var(--line)}.tree-list li:first-child{border-top:0}.tree-list a{text-decoration:none;font-weight:700}.tree-list span{color:var(--muted)}.code{width:100%;border-collapse:collapse;background:var(--panel);border:1px solid var(--line);border-radius:8px;overflow:hidden;display:block}.code tbody{display:table;width:100%}.code tr:target{background:color-mix(in srgb,var(--accent) 14%,transparent)}.code th{width:1%;min-width:54px;padding:0 12px;text-align:right;color:var(--muted);border-right:1px solid var(--line);user-select:none}.code th a{text-decoration:none;color:inherit}.code td{padding:0 12px;white-space:pre;overflow:auto}.code code,.commit code{font-family:ui-monospace,SFMono-Regular,Consolas,monospace;font-size:14px}.commit{background:var(--panel);border:1px solid var(--line);border-radius:8px;padding:16px;overflow:auto}.empty{max-width:720px;margin:80px auto;padding:0 18px;color:var(--muted)}@media(max-width:760px){.repos,.crate-grid{grid-template-columns:1fr}.repo-head{display:block}.commit-link{display:inline-block;margin-top:8px}}
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
            "#[test]\nfn demo() {}\n",
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
        assert_eq!(crates[0].test_count, 1);
        let html = render_crate_page(
            &GitConfig::new(base.join("repos")),
            &repo,
            &crates[0],
            &crates,
        );
        assert!(html.contains("Workspace crate"));
        assert!(html.contains("edgerun-demo"));
        assert!(!html.contains("edgerun-hidden"));
        let _ = fs::remove_dir_all(base);
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
