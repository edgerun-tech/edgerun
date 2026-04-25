//! HTTP server for the editor. Single POST /edit endpoint.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use edgerun_http::{
    Chain, Extensions, Handler, HttpServer, Method, Middleware, Next, Request, Response, StatusCode,
};
use edgerun_json::{from_str, json, to_string, Value};
use quote::ToTokens;

use crate::edit_ops;
use crate::git::{self, GitSafety};
use crate::project::Project;

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

pub async fn start(addr: &str) -> Result<edgerun_http::BoundHttpServer, String> {
    let handler = Chain::new(EditHandler)
        .with(middleware_fn(|req: Request, next: Next| {
            Box::pin(async move {
                // CORS for local dev
                let resp = next.run(req).await;
                // Add CORS headers
                let mut resp = resp;
                resp.headers_mut()
                    .insert("access-control-allow-origin", "*")
                    .ok();
                resp.headers_mut()
                    .insert("access-control-allow-methods", "GET, POST, OPTIONS")
                    .ok();
                resp.headers_mut()
                    .insert("access-control-allow-headers", "content-type")
                    .ok();
                resp
            })
        }))
        .build();

    HttpServer::new(handler)
        .keep_alive(None)
        .bind(addr)
        .await
        .map_err(|e| format!("bind {addr}: {e}"))
}

fn middleware_fn<F>(f: F) -> impl Middleware
where
    F: Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>
        + Send
        + Sync
        + 'static,
{
    edgerun_http::middleware_fn(f)
}

struct EditHandler;

impl Handler for EditHandler {
    fn handle(
        &self,
        request: Request,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send + '_>> {
        Box::pin(async move {
            if SHUTDOWN.load(Ordering::Relaxed) {
                return Response::internal_error();
            }

            match request.method() {
                Method::GET if request.uri().path() == "/health" => {
                    return Response::text(StatusCode::OK, "ok");
                }
                Method::POST if request.uri().path() == "/health" => {
                    SHUTDOWN.store(true, Ordering::Relaxed);
                    return Response::text(StatusCode::OK, "shutting down");
                }
                Method::OPTIONS => {
                    return Response::new(StatusCode::NO_CONTENT);
                }
                Method::POST if request.uri().path() == "/edit" => {}
                _ => {
                    return Response::text(
                        StatusCode::NOT_FOUND,
                        "not found — POST /edit or GET /health",
                    );
                }
            }

            let body = match request.body() {
                Some(b) => b,
                None => {
                    return Response::json(
                        StatusCode::BAD_REQUEST,
                        r#"{"ok":false,"error":"empty body"}"#,
                    )
                }
            };

            let body_str = match std::str::from_utf8(body) {
                Ok(s) => s,
                Err(e) => {
                    return Response::json(
                        StatusCode::BAD_REQUEST,
                        &format!(r#"{{"ok":false,"error":"invalid utf8: {e}"}}"#),
                    )
                }
            };

            let req: Value = match from_str(body_str) {
                Ok(v) => v,
                Err(e) => {
                    return Response::json(
                        StatusCode::BAD_REQUEST,
                        &format!(r#"{{"ok":false,"error":"invalid json: {e}"}}"#),
                    )
                }
            };

            let resp = handle_edit(&req).await;
            let body = to_string(&resp).unwrap_or_else(|_| {
                r#"{"ok":false,"error":"failed to serialize response"}"#.to_string()
            });
            Response::json(StatusCode::OK, &body)
        })
    }
}

/// Resolve the cargo binary path: tries `cargo` on PATH, then falls back
/// to the standard rustup install location.
fn resolve_cargo() -> PathBuf {
    // Try PATH first (works when cargo is installed and on PATH)
    if which_in_path("cargo") {
        return PathBuf::from("cargo");
    }
    // Fallback: standard rustup location
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let candidate = PathBuf::from(format!("{home}/.cargo/bin/cargo"));
    if candidate.exists() {
        return candidate;
    }
    // Last resort: just use "cargo" and let the OS try PATH
    PathBuf::from("cargo")
}

fn which_in_path(name: &str) -> bool {
    if let Ok(paths) = std::env::var("PATH") {
        for dir in paths.split(':') {
            let candidate = PathBuf::from(dir).join(name);
            if candidate.exists() {
                return true;
            }
        }
    }
    false
}

async fn handle_edit(req: &Value) -> Value {
    // Resolve project
    let project_path = req["project"].as_str().unwrap_or(".");
    let project_path = PathBuf::from(project_path);

    let project = if project_path.ends_with("Cargo.toml") && project_path.exists() {
        match Project::from_manifest(&project_path) {
            Ok(p) => p,
            Err(e) => return json!({ "ok": false, "error": e }),
        }
    } else if project_path.exists() {
        Project::single_file(&project_path)
    } else {
        let candidate = project_path.join("Cargo.toml");
        if candidate.exists() {
            match Project::from_manifest(&candidate) {
                Ok(p) => p,
                Err(e) => return json!({ "ok": false, "error": e }),
            }
        } else {
            return json!({
                "ok": false,
                "error": format!("project not found: {}", project_path.display()),
            });
        }
    };

    let edits = req["edits"].as_array().cloned().unwrap_or_default();
    if edits.is_empty() {
        return json!({
            "ok": false,
            "error": "no edits provided",
        });
    }

    // Phase 1: Parse all project files once into a cache.
    // Each file is parsed exactly once, then all transforms are applied
    // to the in-memory AST, and only at the end are modified files written.
    let mut file_cache: HashMap<PathBuf, syn::File> = HashMap::new();
    for file in &project.source_files {
        match edit_ops::parse_file(file) {
            Ok(parsed) => {
                file_cache.insert(file.clone(), parsed);
            }
            Err(e) => {
                return json!({
                    "ok": false,
                    "error": format!("parsing {}: {e}", file.display()),
                });
            }
        }
    }

    // Set up git safety if project root is a git repo
    let root = project.root_dir();
    let mut git_safety = GitSafety::new(root);

    // Phase 2: Apply edits to cached ASTs.
    // Track which files are modified so we only write those.
    let mut modified_paths: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
    let mut new_files: Vec<String> = Vec::new();

    for (i, edit) in edits.iter().enumerate() {
        let op = edit["op"].as_str().unwrap_or("");
        let result = match op {
            "rename_type" => {
                let old = edit["old"].as_str().unwrap_or("");
                let new_name = edit["new"].as_str().unwrap_or("");
                apply_rename_type_cached(
                    &mut file_cache,
                    &project,
                    old,
                    new_name,
                    &mut modified_paths,
                )
            }
            "add_fn" => {
                let file = resolve_file(&project, edit);
                let name = edit["name"].as_str().unwrap_or("");
                let args = edit["args"].as_str().unwrap_or("");
                let ret = edit["ret"].as_str().unwrap_or("");
                let body = edit["body"].as_str().unwrap_or("");
                apply_add_fn_cached(
                    &mut file_cache,
                    &file,
                    name,
                    args,
                    ret,
                    body,
                    &mut modified_paths,
                )
            }
            "replace_fn_body" => {
                let file = resolve_file(&project, edit);
                let name = edit["name"].as_str().unwrap_or("");
                let body = edit["body"].as_str().unwrap_or("");
                apply_replace_fn_body_cached(
                    &mut file_cache,
                    &file,
                    name,
                    body,
                    &mut modified_paths,
                )
            }
            "remove_fn" => {
                let file = resolve_file(&project, edit);
                let name = edit["name"].as_str().unwrap_or("");
                apply_remove_fn_cached(&mut file_cache, &file, name, &mut modified_paths)
            }
            "add_use" => {
                let file = resolve_file(&project, edit);
                let use_path = edit["use"].as_str().unwrap_or("");
                apply_add_use_cached(&mut file_cache, &file, use_path, &mut modified_paths)
            }
            "add_derive" => {
                let file = resolve_file(&project, edit);
                let name = edit["name"].as_str().unwrap_or("");
                let derive = edit["derive"].as_str().unwrap_or("");
                apply_add_derive_cached(&mut file_cache, &file, name, derive, &mut modified_paths)
            }
            "new_file" => {
                let path = edit["path"].as_str().unwrap_or("");
                let content = edit["content"].as_str().unwrap_or("");
                apply_new_file(&project, path, content, &mut new_files)
            }
            "remove_file" => {
                let path = edit["path"].as_str().unwrap_or("");
                let force = edit["force"].as_bool().unwrap_or(false);
                apply_remove_file_cached(
                    &mut file_cache,
                    &project,
                    path,
                    force,
                    &mut modified_paths,
                )
            }
            _ => Err(format!("unknown op: {op}")),
        };

        if let Err(e) = result {
            // Rollback all file modifications
            if let Some(git) = &git_safety {
                let (_, _) = git.rollback();
            }
            // Delete any new files created
            for f in &new_files {
                std::fs::remove_file(f).ok();
            }
            return json!({
                "ok": false,
                "error": format!("edit[{i}] ({op}): {e}"),
                "rolled_back": true,
            });
        }
    }

    // Phase 3: Write all modified files once.
    for path in &modified_paths {
        if let Some(ast) = file_cache.get(path) {
            if let Err(e) = edit_ops::write_file(path, ast) {
                // Rollback on write failure
                if let Some(git) = &git_safety {
                    let (_, _) = git.rollback();
                }
                return json!({
                    "ok": false,
                    "error": format!("writing {}: {e}", path.display()),
                    "rolled_back": true,
                });
            }
        }
    }

    // Track all modified files for git
    if let Some(ref mut git) = git_safety {
        for f in &modified_paths {
            git.track(f);
        }
        for f in &new_files {
            git.track(PathBuf::from(f).as_path());
        }
    }

    // Run cargo check
    let cargo_check = run_cargo_check(&project);
    if let Err(e) = cargo_check {
        if let Some(ref git) = git_safety {
            let (_, msg) = git.rollback();
            return json!({
                "ok": false,
                "error": "cargo check failed",
                "stderr": e,
                "rolled_back": true,
                "rollback_msg": msg,
            });
        }
        return json!({
            "ok": false,
            "error": "cargo check failed",
            "stderr": e,
            "rolled_back": false,
        });
    }

    // Stage changes
    let staged = if let Some(ref git) = git_safety {
        let (ok, _) = git.stage_all();
        ok
    } else {
        false
    };

    json!({
        "ok": true,
        "files_modified": modified_paths.len(),
        "new_files": new_files.len(),
        "staged": staged,
    })
}

fn resolve_file(project: &Project, edit: &Value) -> PathBuf {
    if let Some(f) = edit["file"].as_str() {
        let p = PathBuf::from(f);
        if p.is_absolute() {
            return p;
        }
        return project.root_dir().join(&p);
    }
    // Default: try src/lib.rs, then src/main.rs
    let lib = project.root_dir().join("src/lib.rs");
    if lib.exists() {
        return lib;
    }
    project.root_dir().join("src/main.rs")
}

// ── Cached apply functions (operate on in-memory ASTs) ───────────────────────

fn apply_rename_type_cached(
    cache: &mut HashMap<PathBuf, syn::File>,
    project: &Project,
    old: &str,
    new: &str,
    modified: &mut std::collections::HashSet<PathBuf>,
) -> Result<(), String> {
    for file in &project.source_files {
        if let Some(parsed) = cache.get_mut(file) {
            edit_ops::rename_type_in_file(parsed, old, new);
            modified.insert(file.clone());
        }
    }
    Ok(())
}

fn apply_add_fn_cached(
    cache: &mut HashMap<PathBuf, syn::File>,
    file: &Path,
    name: &str,
    args: &str,
    ret: &str,
    body: &str,
    modified: &mut std::collections::HashSet<PathBuf>,
) -> Result<(), String> {
    let parsed = ensure_in_cache(cache, file)?;
    edit_ops::add_fn(parsed, name, args, ret, body)?;
    modified.insert(file.to_path_buf());
    Ok(())
}

fn apply_replace_fn_body_cached(
    cache: &mut HashMap<PathBuf, syn::File>,
    file: &Path,
    name: &str,
    body: &str,
    modified: &mut std::collections::HashSet<PathBuf>,
) -> Result<(), String> {
    let parsed = ensure_in_cache(cache, file)?;
    if !edit_ops::replace_fn_body(parsed, name, body)? {
        return Err(format!("fn {name} not found"));
    }
    modified.insert(file.to_path_buf());
    Ok(())
}

fn apply_remove_fn_cached(
    cache: &mut HashMap<PathBuf, syn::File>,
    file: &Path,
    name: &str,
    modified: &mut std::collections::HashSet<PathBuf>,
) -> Result<(), String> {
    let parsed = ensure_in_cache(cache, file)?;
    if !edit_ops::remove_fn(parsed, name) {
        return Err(format!("fn {name} not found"));
    }
    modified.insert(file.to_path_buf());
    Ok(())
}

fn apply_add_use_cached(
    cache: &mut HashMap<PathBuf, syn::File>,
    file: &Path,
    use_path: &str,
    modified: &mut std::collections::HashSet<PathBuf>,
) -> Result<(), String> {
    let parsed = ensure_in_cache(cache, file)?;
    edit_ops::add_use(parsed, use_path)?;
    modified.insert(file.to_path_buf());
    Ok(())
}

fn apply_add_derive_cached(
    cache: &mut HashMap<PathBuf, syn::File>,
    file: &Path,
    name: &str,
    derive: &str,
    modified: &mut std::collections::HashSet<PathBuf>,
) -> Result<(), String> {
    let parsed = ensure_in_cache(cache, file)?;
    if !edit_ops::add_derive(parsed, name, derive)? {
        return Err(format!("struct/enum {name} not found"));
    }
    modified.insert(file.to_path_buf());
    Ok(())
}

fn apply_remove_file_cached(
    cache: &mut HashMap<PathBuf, syn::File>,
    project: &Project,
    path: &str,
    force: bool,
    modified: &mut std::collections::HashSet<PathBuf>,
) -> Result<(), String> {
    let abs = if PathBuf::from(path).is_absolute() {
        PathBuf::from(path)
    } else {
        project.root_dir().join(path)
    };
    if !abs.exists() {
        return Err(format!("file not found: {}", abs.display()));
    }
    if !force {
        let refs = edit_ops::incoming_refs(&project.source_files, &abs);
        if refs > 0 {
            return Err(format!(
                "file has {refs} incoming reference(s); set force=true to delete anyway"
            ));
        }
    }
    edit_ops::remove_file(&abs)?;
    cache.remove(&abs);
    modified.insert(abs.clone());
    Ok(())
}

/// Ensure a file is in the cache (for files not in the initial project scan,
/// e.g. when a resolve_file points to a file outside source_files).
fn ensure_in_cache<'a>(
    cache: &'a mut HashMap<PathBuf, syn::File>,
    file: &Path,
) -> Result<&'a mut syn::File, String> {
    use std::collections::hash_map::Entry;
    match cache.entry(file.to_path_buf()) {
        Entry::Occupied(entry) => Ok(entry.into_mut()),
        Entry::Vacant(entry) => {
            let parsed = edit_ops::parse_file(file)?;
            Ok(entry.insert(parsed))
        }
    }
}

fn apply_new_file(
    project: &Project,
    path: &str,
    content: &str,
    new_files: &mut Vec<String>,
) -> Result<(), String> {
    let abs = if PathBuf::from(path).is_absolute() {
        PathBuf::from(path)
    } else {
        project.root_dir().join(path)
    };
    edit_ops::new_file(&abs, content)?;
    new_files.push(abs.to_string_lossy().to_string());
    Ok(())
}

fn run_cargo_check(project: &Project) -> Result<(), String> {
    use std::process::Command;
    let cargo = resolve_cargo();
    let output = Command::new(&cargo)
        .arg("check")
        .arg("--manifest-path")
        .arg(&project.manifest)
        .env("RUSTFLAGS", "-A dead_code -A unused_variables -A unused_imports -A unused_assignments -A unused_mut -A non_upper_case_globals -A non_snake_case -A non_camel_case_types")
        .output()
        .map_err(|e| format!("running {} check: {e}", cargo.display()))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}
