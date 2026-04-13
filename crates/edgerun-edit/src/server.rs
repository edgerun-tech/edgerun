//! HTTP server for the editor. Single POST /edit endpoint.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use edgerun_http::{
    Chain, Extensions, Handler, HttpServer, Method, Middleware, Next, Request, Response, StatusCode,
};
use edgerun_json::{json, from_str, to_string, Value};
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
                    return Response::text(StatusCode::NOT_FOUND, "not found — POST /edit or GET /health");
                }
            }

            let body = match request.body() {
                Some(b) => b,
                None => return Response::json(StatusCode::BAD_REQUEST, r#"{"ok":false,"error":"empty body"}"#),
            };

            let body_str = match std::str::from_utf8(body) {
                Ok(s) => s,
                Err(e) => return Response::json(StatusCode::BAD_REQUEST, &format!(r#"{{"ok":false,"error":"invalid utf8: {e}"}}"#)),
            };

            let req: Value = match from_str(body_str) {
                Ok(v) => v,
                Err(e) => return Response::json(StatusCode::BAD_REQUEST, &format!(r#"{{"ok":false,"error":"invalid json: {e}"}}"#)),
            };

            let resp = handle_edit(&req).await;
            let body = to_string(&resp).unwrap_or_else(|_| r#"{"ok":false,"error":"failed to serialize response"}"#.to_string());
            Response::json(StatusCode::OK, &body)
        })
    }
}

async fn handle_edit(req: &Value) -> Value {
    // Resolve project
    let project_path = req["project"].as_str().unwrap_or(".");
    let project_path = PathBuf::from(project_path);

    // If it's a Cargo.toml, load as project; if it's a .rs file, single-file mode
    let project = if project_path.ends_with("Cargo.toml") && project_path.exists() {
        Project::from_manifest(&project_path)
    } else if project_path.exists() {
        Project::single_file(&project_path)
    } else {
        // Try auto-detect: look for Cargo.toml in project_path dir
        let candidate = project_path.join("Cargo.toml");
        if candidate.exists() {
            Project::from_manifest(&candidate)
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

    // Set up git safety if project root is a git repo
    let root = project.root_dir();
    let mut git_safety = GitSafety::new(root);

    // Apply each edit
    let mut modified_files: Vec<String> = Vec::new();
    let mut new_files: Vec<String> = Vec::new();

    for (i, edit) in edits.iter().enumerate() {
        let op = edit["op"].as_str().unwrap_or("");
        let result = match op {
            "rename_type" => {
                let old = edit["old"].as_str().unwrap_or("");
                let new_name = edit["new"].as_str().unwrap_or("");
                apply_rename_type(&project, old, new_name, &mut modified_files)
            }
            "add_fn" => {
                let file = resolve_file(&project, edit);
                let name = edit["name"].as_str().unwrap_or("");
                let args = edit["args"].as_str().unwrap_or("");
                let ret = edit["ret"].as_str().unwrap_or("");
                let body = edit["body"].as_str().unwrap_or("");
                apply_add_fn(&file, name, args, ret, body, &mut modified_files)
            }
            "replace_fn_body" => {
                let file = resolve_file(&project, edit);
                let name = edit["name"].as_str().unwrap_or("");
                let body = edit["body"].as_str().unwrap_or("");
                apply_replace_fn_body(&file, name, body, &mut modified_files)
            }
            "remove_fn" => {
                let file = resolve_file(&project, edit);
                let name = edit["name"].as_str().unwrap_or("");
                apply_remove_fn(&file, name, &mut modified_files)
            }
            "add_use" => {
                let file = resolve_file(&project, edit);
                let use_path = edit["use"].as_str().unwrap_or("");
                apply_add_use(&file, use_path, &mut modified_files)
            }
            "add_derive" => {
                let file = resolve_file(&project, edit);
                let name = edit["name"].as_str().unwrap_or("");
                let derive = edit["derive"].as_str().unwrap_or("");
                apply_add_derive(&file, name, derive, &mut modified_files)
            }
            "new_file" => {
                let path = edit["path"].as_str().unwrap_or("");
                let content = edit["content"].as_str().unwrap_or("");
                apply_new_file(&project, path, content, &mut new_files)
            }
            "remove_file" => {
                let path = edit["path"].as_str().unwrap_or("");
                let force = edit["force"].as_bool().unwrap_or(false);
                apply_remove_file(&project, path, force, &mut modified_files)
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

    // Track all modified files for git
    if let Some(ref mut git) = git_safety {
        for f in &modified_files {
            git.track(PathBuf::from(f).as_path());
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
        let (ok, msg) = git.stage_all();
        ok
    } else {
        false
    };

    json!({
        "ok": true,
        "files_modified": modified_files.len(),
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

fn apply_rename_type(
    project: &Project,
    old: &str,
    new: &str,
    modified: &mut Vec<String>,
) -> Result<(), String> {
    for file in &project.source_files {
        let mut parsed = edit_ops::parse_file(file)?;
        edit_ops::rename_type_in_file(&mut parsed, old, new);
        edit_ops::write_file(file, &parsed)?;
        modified.push(file.to_string_lossy().to_string());
    }
    Ok(())
}

fn apply_add_fn(
    file: &PathBuf,
    name: &str,
    args: &str,
    ret: &str,
    body: &str,
    modified: &mut Vec<String>,
) -> Result<(), String> {
    let mut parsed = edit_ops::parse_file(file)?;
    edit_ops::add_fn(&mut parsed, name, args, ret, body)?;
    edit_ops::write_file(file, &parsed)?;
    modified.push(file.to_string_lossy().to_string());
    Ok(())
}

fn apply_replace_fn_body(
    file: &PathBuf,
    name: &str,
    body: &str,
    modified: &mut Vec<String>,
) -> Result<(), String> {
    let mut parsed = edit_ops::parse_file(file)?;
    if !edit_ops::replace_fn_body(&mut parsed, name, body)? {
        return Err(format!("fn {name} not found"));
    }
    edit_ops::write_file(file, &parsed)?;
    modified.push(file.to_string_lossy().to_string());
    Ok(())
}

fn apply_remove_fn(
    file: &PathBuf,
    name: &str,
    modified: &mut Vec<String>,
) -> Result<(), String> {
    let mut parsed = edit_ops::parse_file(file)?;
    if !edit_ops::remove_fn(&mut parsed, name) {
        return Err(format!("fn {name} not found"));
    }
    edit_ops::write_file(file, &parsed)?;
    modified.push(file.to_string_lossy().to_string());
    Ok(())
}

fn apply_add_use(
    file: &PathBuf,
    use_path: &str,
    modified: &mut Vec<String>,
) -> Result<(), String> {
    let mut parsed = edit_ops::parse_file(file)?;
    edit_ops::add_use(&mut parsed, use_path)?;
    edit_ops::write_file(file, &parsed)?;
    modified.push(file.to_string_lossy().to_string());
    Ok(())
}

fn apply_add_derive(
    file: &PathBuf,
    name: &str,
    derive: &str,
    modified: &mut Vec<String>,
) -> Result<(), String> {
    let mut parsed = edit_ops::parse_file(file)?;
    if !edit_ops::add_derive(&mut parsed, name, derive)? {
        return Err(format!("struct/enum {name} not found"));
    }
    edit_ops::write_file(file, &parsed)?;
    modified.push(file.to_string_lossy().to_string());
    Ok(())
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

fn apply_remove_file(
    project: &Project,
    path: &str,
    force: bool,
    modified: &mut Vec<String>,
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
    modified.push(abs.to_string_lossy().to_string());
    Ok(())
}

fn run_cargo_check(project: &Project) -> Result<(), String> {
    use std::process::Command;
    let output = Command::new("cargo")
        .arg("check")
        .arg("--manifest-path")
        .arg(&project.manifest)
        .env("RUSTFLAGS", "-A dead_code -A unused_variables -A unused_imports -A unused_assignments -A unused_mut -A non_upper_case_globals -A non_snake_case -A non_camel_case_types")
        .output()
        .map_err(|e| format!("running cargo check: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}
