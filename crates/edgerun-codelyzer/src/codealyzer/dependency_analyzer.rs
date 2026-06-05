use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::codealyzer::cargo_toml_projection::{
    DependencyProjection, DependencyValue, parse_cargo_toml_projection,
};
use crate::codealyzer::crate_model::*;
use crate::codealyzer::errors::Result;

pub fn analyze_dependencies(
    cargo_toml_path: &Path,
    workspace_root: &Path,
    visible_files: &[PathBuf],
) -> Result<Vec<Dependency>> {
    let content = std::fs::read_to_string(cargo_toml_path)
        .map_err(|e| crate::codealyzer::errors::AnalyzerError::IoError(e.to_string()))?;

    let projection = parse_cargo_toml_projection(&content);
    let workspace_members = load_workspace_members(workspace_root);
    let mut deps = Vec::new();

    append_dependencies(
        &mut deps,
        &projection.dependencies,
        DependencyKind::Normal,
        &workspace_members,
        visible_files,
    );
    append_dependencies(
        &mut deps,
        &projection.dev_dependencies,
        DependencyKind::Dev,
        &workspace_members,
        visible_files,
    );
    append_dependencies(
        &mut deps,
        &projection.build_dependencies,
        DependencyKind::Build,
        &workspace_members,
        visible_files,
    );

    Ok(deps)
}

fn append_dependencies(
    deps: &mut Vec<Dependency>,
    deps_obj: &[DependencyProjection],
    kind: DependencyKind,
    workspace_members: &HashSet<String>,
    visible_files: &[PathBuf],
) {
    for row in deps_obj {
        let dependency_value = build_dependency_value(row);
        deps.push(parse_dependency(
            &row.name,
            &dependency_value,
            kind,
            workspace_members,
            visible_files,
        ));
    }
}

fn build_dependency_value(row: &DependencyProjection) -> DependencyValue {
    if row.properties.is_empty() {
        return row.value.clone();
    }

    let mut table = Vec::new();
    match &row.value {
        DependencyValue::InlineTable(value) => {
            table.extend(value.clone());
        }
        DependencyValue::String(value) => {
            table.push((
                "version".to_string(),
                DependencyValue::String(value.clone()),
            ));
        }
        DependencyValue::Bare(value) if value.is_empty() => {}
        other => {
            table.push(("version".to_string(), other.clone()));
        }
    }
    table.extend(row.properties.clone());
    DependencyValue::InlineTable(table)
}

fn parse_dependency(
    name: &str,
    value: &DependencyValue,
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
    } else if let DependencyValue::String(v) | DependencyValue::Bare(v) = value {
        if !v.is_empty() {
            version_req = Some(strip_toml_quoted(v));
            dependency_source = DependencySource::CratesIo;
        }
    } else if let DependencyValue::InlineTable(obj) = value {
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

fn parse_dependency_table(name: &str, table: &[(String, DependencyValue)]) -> DependencyConfig {
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
        .and_then(|(_, v)| v.as_string_array())
        .map(|arr| arr.iter().map(|v| strip_toml_quoted(v)).collect())
        .unwrap_or_default();

    if let Some(v) = table
        .iter()
        .find(|(k, _)| k == &"path")
        .and_then(|(_, v)| v.as_str())
    {
        config.dependency_source = DependencySource::Path;
        config.source_ref = Some(strip_toml_quoted(v));
    }

    if let Some(v) = table
        .iter()
        .find(|(k, _)| k == &"git")
        .and_then(|(_, v)| v.as_str())
    {
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

fn parse_dependency_inline_table(value: &str) -> Option<Vec<(String, DependencyValue)>> {
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

fn parse_dependency_value(value: &str) -> DependencyValue {
    let value = value.trim();
    if value.is_empty() {
        return DependencyValue::Bare(String::new());
    }

    if (value.starts_with('"') && value.ends_with('"'))
        || (value.starts_with('\'') && value.ends_with('\''))
    {
        return DependencyValue::String(strip_toml_quoted(value));
    }

    if value == "true" {
        return DependencyValue::Bool(true);
    }
    if value == "false" {
        return DependencyValue::Bool(false);
    }

    if value.starts_with('[') && value.ends_with(']') {
        let inner = &value[1..value.len() - 1];
        let items = if inner.trim().is_empty() {
            Vec::new()
        } else {
            split_toml_level(inner, ',')
                .into_iter()
                .filter_map(|item| {
                    let parsed = parse_dependency_value(item.trim());
                    parsed.as_str().map(strip_toml_quoted)
                })
                .collect()
        };
        return DependencyValue::StringArray(items);
    }

    if value.starts_with('{') && value.ends_with('}') {
        let mut entries = Vec::new();
        for raw_entry in split_toml_level(&value[1..value.len() - 1], ',') {
            let entry = raw_entry.trim();
            let Some(eq_pos) = entry.find('=') else {
                continue;
            };
            entries.push((
                entry[..eq_pos].trim().to_string(),
                parse_dependency_value(entry[eq_pos + 1..].trim()),
            ));
        }
        return DependencyValue::InlineTable(entries);
    }

    DependencyValue::Bare(value.to_string())
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
                            n if n.starts_with("edgerun-") => {
                                "edgerun external or local path dependency"
                            }
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

    let projection = parse_cargo_toml_projection(&content);

    for name in projection.workspace_members {
        if name.starts_with("crates/") {
            members.insert(name.trim_start_matches("crates/").to_string());
        }
    }
    members.extend(projection.workspace_dependencies);

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
