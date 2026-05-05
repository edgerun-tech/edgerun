use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::codealyzer::crate_model::*;
use crate::codealyzer::errors::Result;
use edgerun_json::TomlValue;

pub fn analyze_dependencies(
    cargo_toml_path: &Path,
    workspace_root: &Path,
    visible_files: &[PathBuf],
) -> Result<Vec<Dependency>> {
    let content = std::fs::read_to_string(cargo_toml_path)
        .map_err(|e| crate::codealyzer::errors::AnalyzerError::IoError(e.to_string()))?;

    let toml = edgerun_json::from_toml_str(&content)
        .map_err(|e| crate::codealyzer::errors::AnalyzerError::ParseError {
            file: cargo_toml_path.to_path_buf(),
            message: e.to_string(),
        })?;

    let workspace_members = load_workspace_members(workspace_root);
    let mut deps = Vec::new();

    if let Some(deps_obj) = toml.get("dependencies").and_then(|d| d.as_table()) {
        append_dependencies(&mut deps, deps_obj, DependencyKind::Normal, &workspace_members, visible_files);
    }

    if let Some(deps_obj) = toml.get("dev-dependencies").and_then(|d| d.as_table()) {
        append_dependencies(
            &mut deps,
            deps_obj,
            DependencyKind::Dev,
            &workspace_members,
            visible_files,
        );
    }

    if let Some(deps_obj) = toml.get("build-dependencies").and_then(|d| d.as_table()) {
        append_dependencies(
            &mut deps,
            deps_obj,
            DependencyKind::Build,
            &workspace_members,
            visible_files,
        );
    }

    Ok(deps)
}

#[derive(Default)]
struct DependencySpec {
    direct: Option<TomlValue>,
    properties: Vec<(String, TomlValue)>,
}

fn append_dependencies(
    deps: &mut Vec<Dependency>,
    deps_obj: &Vec<(String, TomlValue)>,
    kind: DependencyKind,
    workspace_members: &HashSet<String>,
    visible_files: &[PathBuf],
) {
    let mut grouped: HashMap<String, DependencySpec> = HashMap::new();

    for (name, value) in deps_obj {
        if let Some((canonical_name, field)) = name.split_once('.') {
            grouped
                .entry(canonical_name.to_string())
                .or_default()
                .properties
                .push((field.to_string(), value.clone()));
        } else {
            grouped
                .entry(name.to_string())
                .or_default()
                .direct
                .replace(value.clone());
        }
    }

    for (name, spec) in grouped {
        let dependency_value = build_dependency_value(spec);
        deps.push(parse_dependency(
            &name,
            &dependency_value,
            kind,
            workspace_members,
            visible_files,
        ));
    }
}

fn build_dependency_value(spec: DependencySpec) -> TomlValue {
    if spec.properties.is_empty() {
        return spec.direct.unwrap_or(TomlValue::String(String::new()));
    }

    let mut table = Vec::new();
    if let Some(direct) = spec.direct {
        match direct {
            TomlValue::Table(value) => {
                table.extend(value);
            }
            TomlValue::String(value) => {
                table.push(("version".to_string(), TomlValue::String(value)));
            }
            other => {
                table.push(("version".to_string(), other));
            }
        }
    }
    table.extend(spec.properties);
    TomlValue::Table(table)
}

fn parse_dependency(
    name: &str,
    value: &TomlValue,
    kind: DependencyKind,
    workspace_members: &HashSet<String>,
    visible_files: &[PathBuf],
) -> Dependency {
    let mut version_req = None;
    let mut optional = false;
    let mut features = Vec::new();
    let mut dependency_source = DependencySource::Unknown;
    let mut source_ref = None;
    let mut workspace_declared = false;

    let mut package_name = name.to_string();

    if let Some(inline_table) = value.as_str().and_then(parse_dependency_inline_table) {
        let parsed = parse_dependency_table(name, &inline_table);
        package_name = parsed.package_name;
        version_req = parsed.version_req;
        optional = parsed.optional;
        features = parsed.features;
        dependency_source = parsed.dependency_source;
        source_ref = parsed.source_ref;
        workspace_declared = parsed.workspace_declared;
    } else if let TomlValue::String(v) = value {
        version_req = Some(strip_toml_quoted(v));
        dependency_source = DependencySource::CratesIo;
    } else if let TomlValue::Table(obj) = value {
        let parsed = parse_dependency_table(name, obj);
        package_name = parsed.package_name;
        version_req = parsed.version_req;
        optional = parsed.optional;
        features = parsed.features;
        dependency_source = parsed.dependency_source;
        source_ref = parsed.source_ref;
        workspace_declared = parsed.workspace_declared;
    }

    let is_workspace = workspace_declared
        || is_workspace_member(name, workspace_members)
        || is_workspace_member(&package_name, workspace_members);

    if dependency_source == DependencySource::Unknown {
        if is_workspace {
            dependency_source = DependencySource::Workspace;
        }
    }

    let mut final_version_req = version_req;
    let is_local_path = matches!(dependency_source, DependencySource::Path);
    if is_local_path && final_version_req.is_none() {
        final_version_req = Some("path".to_string());
    }

    Dependency {
        name: name.to_string(),
        version_req: final_version_req,
        kind,
        optional,
        features,
        reason: detect_reason(name, &kind, is_workspace, &dependency_source),
        is_workspace,
        is_visible: true,
        source: dependency_source,
        source_ref,
        weight: estimate_dependency_weight(&package_name, visible_files),
    }
}

struct DependencyConfig {
    package_name: String,
    version_req: Option<String>,
    optional: bool,
    features: Vec<String>,
    dependency_source: DependencySource,
    source_ref: Option<String>,
    workspace_declared: bool,
}

impl Default for DependencyConfig {
    fn default() -> Self {
        Self {
            package_name: String::new(),
            version_req: None,
            optional: false,
            features: Vec::new(),
            dependency_source: DependencySource::Unknown,
            source_ref: None,
            workspace_declared: false,
        }
    }
}

fn parse_dependency_table(name: &str, table: &Vec<(String, TomlValue)>) -> DependencyConfig {
    let mut config = DependencyConfig {
        package_name: name.to_string(),
        dependency_source: DependencySource::Unknown,
        ..Default::default()
    };

    if let Some(v) = table
        .iter()
        .find(|(k, _)| k == &"package")
        .and_then(|(_, v)| v.as_str())
    {
        config.package_name = strip_toml_quoted(v);
    }

    if let Some(v) = table
        .iter()
        .find(|(k, _)| k == &"version")
        .and_then(|(_, v)| v.as_str())
    {
        config.version_req = Some(strip_toml_quoted(v));
        config.dependency_source = DependencySource::CratesIo;
    }

    config.optional = table
        .iter()
        .find(|(k, _)| k == &"optional")
        .and_then(|(_, v)| v.as_bool())
        .unwrap_or(false);
    config.features = table
        .iter()
        .find(|(k, _)| k == &"features")
        .and_then(|(_, v)| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(strip_toml_quoted)
                .collect()
        })
        .unwrap_or_default();

    if let Some(v) = table.iter().find(|(k, _)| k == &"path").and_then(|(_, v)| v.as_str()) {
        config.dependency_source = DependencySource::Path;
        config.source_ref = Some(strip_toml_quoted(v));
    }

    if let Some(v) = table.iter().find(|(k, _)| k == &"git").and_then(|(_, v)| v.as_str()) {
        config.dependency_source = DependencySource::Git;
        config.source_ref = Some(strip_toml_quoted(v));
    }

    config.workspace_declared = table
        .iter()
        .find(|(k, _)| k == &"workspace")
        .and_then(|(_, v)| v.as_bool())
        .unwrap_or(false);
    config
}

fn parse_dependency_inline_table(value: &str) -> Option<Vec<(String, TomlValue)>> {
    let value = value.trim();
    if !(value.starts_with('{') && value.ends_with('}')) {
        return None;
    }

    let mut entries = Vec::new();
    let content = value[1..value.len() - 1].trim();
    if content.is_empty() {
        return Some(entries);
    }

    for raw_entry in split_toml_level(content, ',') {
        let entry = raw_entry.trim();
        if entry.is_empty() {
            continue;
        }

        let Some(eq_pos) = entry.find('=') else {
            continue;
        };
        let key = entry[..eq_pos].trim();
        if key.is_empty() {
            continue;
        }
        let value = entry[eq_pos + 1..].trim();
        entries.push((key.to_string(), parse_dependency_value(value)));
    }

    Some(entries)
}

fn parse_dependency_value(value: &str) -> TomlValue {
    let value = value.trim();
    if value.is_empty() {
        return TomlValue::String(String::new());
    }

    if (value.starts_with('"') && value.ends_with('"'))
        || (value.starts_with('\'') && value.ends_with('\''))
    {
        return TomlValue::String(strip_toml_quoted(value));
    }

    if value == "true" {
        return TomlValue::Boolean(true);
    }
    if value == "false" {
        return TomlValue::Boolean(false);
    }

    if value.starts_with('[') && value.ends_with(']') {
        let inner = &value[1..value.len() - 1];
        let items = if inner.trim().is_empty() {
            Vec::new()
        } else {
            split_toml_level(inner, ',')
                .into_iter()
                .map(|item| parse_dependency_value(item.trim()))
                .collect()
        };
        return TomlValue::Array(items);
    }

    if let Ok(value) = value.parse::<i64>() {
        return TomlValue::Integer(value);
    }
    if let Ok(value) = value.parse::<f64>() {
        return TomlValue::Float(value);
    }

    TomlValue::String(value.to_string())
}

fn split_toml_level(value: &str, delimiter: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut in_single = false;
    let mut in_double = false;
    let mut bracket_depth = 0usize;
    let mut escaped = false;

    for (index, current) in value.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        if in_double {
            if current == '\\' {
                escaped = true;
            } else if current == '"' {
                in_double = false;
            }
            continue;
        }

        if in_single {
            if current == '\'' {
                in_single = false;
            }
            continue;
        }

        match current {
            '"' => in_double = true,
            '\'' => in_single = true,
            '[' => bracket_depth += 1,
            ']' if bracket_depth > 0 => bracket_depth -= 1,
            c if c == delimiter && bracket_depth == 0 => {
                parts.push(value[start..index].to_string());
                start = index + 1;
            }
            _ => {}
        }
    }

    if start <= value.len() {
        parts.push(value[start..].to_string());
    }

    parts
}

fn detect_reason(
    name: &str,
    kind: &DependencyKind,
    is_workspace: bool,
    source: &DependencySource,
) -> Option<String> {
    match kind {
        DependencyKind::Dev => Some("development/testing".into()),
        DependencyKind::Build => Some("build script".into()),
        _ => {
            let reason = match source {
                DependencySource::Workspace => "edgerun workspace crate",
                DependencySource::Path => "local path dependency",
                DependencySource::Git => "git dependency",
                DependencySource::CratesIo => "crates.io dependency",
                DependencySource::Unknown => {
                    if is_workspace {
                        "edgerun workspace crate"
                    } else {
                        match name {
                            n if n.starts_with("edgerun-") => "edgerun external or local path dependency",
                            _ => "runtime dependency",
                        }
                    }
                }
            };
            Some(reason.into())
        }
    }
}

fn estimate_dependency_weight(name: &str, files: &[PathBuf]) -> usize {
    let mut weight = 0usize;
    let aliases: Vec<String> = vec![name.to_string(), name.replace('-', "_")];

    for file in files {
        if file.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }

        if let Ok(content) = std::fs::read_to_string(file) {
            for alias in &aliases {
                weight += count_token_occurrences(&content, alias);
            }
        }
    }

    weight
}

fn count_token_occurrences(content: &str, token: &str) -> usize {
    let mut count = 0usize;
    if token.is_empty() {
        return 0;
    }

    count += content.matches(token).count();

    let delimiter = format!("{}::", token);
    count += content.matches(&delimiter).count();

    let import_pattern = format!("extern crate {}", token);
    count += content.matches(&import_pattern).count();

    if token.ends_with("_") {
        count += content.matches(&token[..token.len() - 1]).count();
    }

    count
}

fn is_workspace_member(name: &str, members: &HashSet<String>) -> bool {
    members.contains(name)
}

fn load_workspace_members(workspace_root: &Path) -> HashSet<String> {
    let mut members = HashSet::new();

    let workspace_manifest = workspace_root.join("Cargo.toml");
    let Ok(content) = std::fs::read_to_string(&workspace_manifest) else {
        return members;
    };

    let Ok(toml) = edgerun_json::from_toml_str(&content) else {
        return members;
    };

    let Some(workspace) = toml.get("workspace").and_then(|v| v.as_table()) else {
        return members;
    };

    let workspace_members = workspace.iter().find(|(name, _)| name == "members");
    if let Some((_, v)) = workspace_members {
        if let Some(crates) = v.as_array() {
            for item in crates {
    if let Some(name) = item.as_str() {
                    let cleaned = strip_toml_quoted(name);
                    if cleaned.starts_with("crates/") {
                    members.insert(cleaned.trim_start_matches("crates/").to_string());
                    }
                }
            }
        }
    }

    let workspace_dependencies = workspace.iter().find(|(name, _)| name == "dependencies");
    if let Some((_, v)) = workspace_dependencies {
        if let Some(deps) = v.as_table() {
            members.extend(deps.iter().map(|(name, _)| name.clone()));
        }
    }

    members
}

fn strip_toml_quoted(value: &str) -> String {
    if value.len() >= 2 {
        if (value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\''))
        {
            return value[1..value.len() - 1].to_string();
        }
    }
    value.to_string()
}
