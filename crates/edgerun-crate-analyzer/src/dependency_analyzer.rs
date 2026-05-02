use std::path::Path;
use crate::crate_model::*;
use crate::errors::Result;

pub fn analyze_dependencies(cargo_toml_path: &Path, workspace_root: &Path) -> Result<Vec<Dependency>> {
    let content = std::fs::read_to_string(cargo_toml_path)
        .map_err(|e| crate::errors::AnalyzerError::IoError(e.to_string()))?;

    let toml = edgerun_json::from_toml_str(&content)
        .map_err(|e| crate::errors::AnalyzerError::ParseError {
            file: cargo_toml_path.to_path_buf(),
            message: e.to_string(),
        })?;

    let mut deps = Vec::new();

    if let Some(deps_obj) = toml.get("dependencies").and_then(|d| d.as_object()) {
        for (name, value) in deps_obj {
            deps.push(parse_dependency(name, value, DependencyKind::Normal, workspace_root));
        }
    }

    if let Some(deps_obj) = toml.get("dev-dependencies").and_then(|d| d.as_object()) {
        for (name, value) in deps_obj {
            deps.push(parse_dependency(name, value, DependencyKind::Dev, workspace_root));
        }
    }

    if let Some(deps_obj) = toml.get("build-dependencies").and_then(|d| d.as_object()) {
        for (name, value) in deps_obj {
            deps.push(parse_dependency(name, value, DependencyKind::Build, workspace_root));
        }
    }

    Ok(deps)
}

fn parse_dependency(name: &str, value: &edgerun_json::Value, kind: DependencyKind, workspace_root: &Path) -> Dependency {
    let (version_req, optional, features, is_workspace) = match value {
        edgerun_json::Value::String(v) => (Some(v.as_str().to_string()), false, Vec::new(), false),
        edgerun_json::Value::Object(obj) => {
            let ver = obj.get("version").and_then(|v| v.as_str()).map(|s| s.to_string());
            let opt = obj.get("optional").and_then(|v| v.as_bool()).unwrap_or(false);
            let feats = obj.get("features")
                .and_then(|f| f.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect())
                .unwrap_or_default();
            let ws = obj.get("workspace").and_then(|v| v.as_bool()).unwrap_or(false);
            (ver, opt, feats, ws)
        }
        _ => (None, false, Vec::new(), false),
    };

    let is_workspace_crate = workspace_root.join("crates").join(name).exists();

    Dependency {
        name: name.to_string(),
        version_req,
        kind,
        optional,
        features,
        reason: detect_reason(name, &kind),
        is_workspace: is_workspace_crate,
        is_visible: true,
    }
}

fn detect_reason(name: &str, kind: &DependencyKind) -> Option<String> {
    match kind {
        DependencyKind::Dev => Some("development/testing".into()),
        DependencyKind::Build => Some("build script".into()),
        _ => {
            let reason = match name {
                "tokio" | "async-std" => "async runtime",
                "serde" => "serialization",
                "rand" => "randomness",
                "log" | "tracing" => "logging",
                n if n.starts_with("edgerun-") => "edgerun workspace crate",
                _ => "runtime dependency",
            };
            Some(reason.into())
        }
    }
}
