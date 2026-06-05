use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Default)]
pub struct CargoTomlProjection {
    pub package_name: Option<String>,
    pub package_version: Option<String>,
    pub package_description: Option<String>,
    pub package_license: Option<String>,
    pub package_edition: Option<String>,
    pub package_rust_version: Option<String>,
    pub feature_keys: Vec<String>,
    pub workspace_members: Vec<String>,
    pub workspace_default_members: Vec<String>,
    pub workspace_dependencies: Vec<String>,
    pub dependencies: Vec<DependencyProjection>,
    pub dev_dependencies: Vec<DependencyProjection>,
    pub build_dependencies: Vec<DependencyProjection>,
    pub lib_present: bool,
    pub bin_targets: Vec<TargetProjection>,
    pub test_targets: Vec<TargetProjection>,
    pub bench_targets: Vec<TargetProjection>,
}

#[derive(Debug, Clone, Default)]
pub struct TargetProjection {
    pub name: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DependencyProjection {
    pub name: String,
    pub value: DependencyValue,
    pub properties: Vec<(String, DependencyValue)>,
}

#[derive(Debug, Clone)]
pub enum DependencyValue {
    String(String),
    Bool(bool),
    StringArray(Vec<String>),
    InlineTable(Vec<(String, DependencyValue)>),
    Bare(String),
}

impl DependencyValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) | Self::Bare(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_string_array(&self) -> Option<&[String]> {
        match self {
            Self::StringArray(values) => Some(values),
            _ => None,
        }
    }

    pub fn as_inline_table(&self) -> Option<&[(String, DependencyValue)]> {
        match self {
            Self::InlineTable(values) => Some(values),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Section {
    Root,
    Package,
    Features,
    Workspace,
    WorkspaceDependencies,
    Dependencies,
    DevDependencies,
    BuildDependencies,
    Lib,
    Bin,
    Test,
    Bench,
    Other,
}

pub fn parse_cargo_toml_projection(input: &str) -> CargoTomlProjection {
    let mut projection = CargoTomlProjection::default();
    let mut section = Section::Root;
    let mut dependency_rows: BTreeMap<(Section, String), DependencyProjection> = BTreeMap::new();
    let mut current_bin: Option<TargetProjection> = None;
    let mut current_test: Option<TargetProjection> = None;
    let mut current_bench: Option<TargetProjection> = None;

    for line in logical_toml_lines(input) {
        if line.is_empty() {
            continue;
        }

        if line.starts_with("[[") && line.ends_with("]]") {
            flush_target(&mut projection.bin_targets, &mut current_bin);
            flush_target(&mut projection.test_targets, &mut current_test);
            flush_target(&mut projection.bench_targets, &mut current_bench);

            let name = line[2..line.len() - 2].trim();
            section = match name {
                "bin" => {
                    current_bin = Some(TargetProjection::default());
                    Section::Bin
                }
                "test" => {
                    current_test = Some(TargetProjection::default());
                    Section::Test
                }
                "bench" => {
                    current_bench = Some(TargetProjection::default());
                    Section::Bench
                }
                _ => Section::Other,
            };
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            flush_target(&mut projection.bin_targets, &mut current_bin);
            flush_target(&mut projection.test_targets, &mut current_test);
            flush_target(&mut projection.bench_targets, &mut current_bench);

            let name = line[1..line.len() - 1].trim();
            section = match name {
                "package" => Section::Package,
                "features" => Section::Features,
                "workspace" => Section::Workspace,
                "workspace.dependencies" => Section::WorkspaceDependencies,
                "dependencies" => Section::Dependencies,
                "dev-dependencies" => Section::DevDependencies,
                "build-dependencies" => Section::BuildDependencies,
                "lib" => {
                    projection.lib_present = true;
                    Section::Lib
                }
                _ => Section::Other,
            };
            continue;
        }

        let Some((raw_key, raw_value)) = split_key_value(&line) else {
            continue;
        };
        let key = normalize_key(raw_key);
        let value = raw_value.trim();

        match section {
            Section::Package => match key.as_str() {
                "name" => projection.package_name = parse_toml_string(value),
                "version" => projection.package_version = parse_toml_string(value),
                "description" => projection.package_description = parse_toml_string(value),
                "license" => projection.package_license = parse_toml_string(value),
                "edition" => projection.package_edition = parse_toml_string(value),
                "rust-version" => projection.package_rust_version = parse_toml_string(value),
                _ => {}
            },
            Section::Features => projection.feature_keys.push(key),
            Section::Workspace => match key.as_str() {
                "members" => projection.workspace_members = parse_string_array(value),
                "default-members" => {
                    projection.workspace_default_members = parse_string_array(value)
                }
                _ => {}
            },
            Section::WorkspaceDependencies => projection.workspace_dependencies.push(key),
            Section::Dependencies | Section::DevDependencies | Section::BuildDependencies => {
                push_dependency_cell(
                    &mut dependency_rows,
                    &section,
                    key,
                    parse_dependency_value(value),
                );
            }
            Section::Lib => {
                projection.lib_present = true;
            }
            Section::Bin => update_target(
                current_bin.get_or_insert_with(TargetProjection::default),
                &key,
                value,
            ),
            Section::Test => update_target(
                current_test.get_or_insert_with(TargetProjection::default),
                &key,
                value,
            ),
            Section::Bench => update_target(
                current_bench.get_or_insert_with(TargetProjection::default),
                &key,
                value,
            ),
            Section::Root | Section::Other => {}
        }
    }

    flush_target(&mut projection.bin_targets, &mut current_bin);
    flush_target(&mut projection.test_targets, &mut current_test);
    flush_target(&mut projection.bench_targets, &mut current_bench);

    for ((section, _), row) in dependency_rows {
        match section {
            Section::Dependencies => projection.dependencies.push(row),
            Section::DevDependencies => projection.dev_dependencies.push(row),
            Section::BuildDependencies => projection.build_dependencies.push(row),
            _ => {}
        }
    }

    dedupe_sort(&mut projection.feature_keys);
    dedupe_sort(&mut projection.workspace_members);
    dedupe_sort(&mut projection.workspace_default_members);
    dedupe_sort(&mut projection.workspace_dependencies);
    projection
}

pub fn parse_standard_statuses(input: &str) -> Vec<(String, String)> {
    let mut rows = Vec::new();
    let mut current_table: Option<String> = None;

    for line in logical_toml_lines(input) {
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') && !line.starts_with("[[") {
            current_table = Some(line[1..line.len() - 1].trim().to_string());
            continue;
        }
        let Some((raw_key, raw_value)) = split_key_value(&line) else {
            continue;
        };
        if normalize_key(raw_key) != "status" {
            continue;
        }
        let Some(table) = current_table.clone() else {
            continue;
        };
        rows.push((
            table,
            parse_toml_string(raw_value.trim()).unwrap_or_else(|| "unknown".to_string()),
        ));
    }

    rows
}

fn push_dependency_cell(
    rows: &mut BTreeMap<(Section, String), DependencyProjection>,
    section: &Section,
    key: String,
    value: DependencyValue,
) {
    let (name, property) = key
        .split_once('.')
        .map(|(name, property)| (name.to_string(), Some(property.to_string())))
        .unwrap_or_else(|| (key, None));
    let row = rows
        .entry((section.clone(), name.clone()))
        .or_insert_with(|| DependencyProjection {
            name,
            value: DependencyValue::Bare(String::new()),
            properties: Vec::new(),
        });
    if let Some(property) = property {
        row.properties.push((property, value));
    } else {
        row.value = value;
    }
}

fn update_target(target: &mut TargetProjection, key: &str, value: &str) {
    match key {
        "name" => target.name = parse_toml_string(value),
        "path" => target.path = parse_toml_string(value),
        _ => {}
    }
}

fn flush_target(targets: &mut Vec<TargetProjection>, current: &mut Option<TargetProjection>) {
    if let Some(target) = current.take() {
        targets.push(target);
    }
}

fn parse_dependency_value(value: &str) -> DependencyValue {
    let value = value.trim();
    if let Some(value) = parse_toml_string(value) {
        return DependencyValue::String(value);
    }
    if value == "true" {
        return DependencyValue::Bool(true);
    }
    if value == "false" {
        return DependencyValue::Bool(false);
    }
    if value.starts_with('[') && value.ends_with(']') {
        return DependencyValue::StringArray(parse_string_array(value));
    }
    if value.starts_with('{') && value.ends_with('}') {
        let inner = &value[1..value.len() - 1];
        let mut table = Vec::new();
        for entry in split_toml_level(inner, ',') {
            let Some((key, value)) = split_key_value(entry.trim()) else {
                continue;
            };
            table.push((normalize_key(key), parse_dependency_value(value.trim())));
        }
        return DependencyValue::InlineTable(table);
    }
    DependencyValue::Bare(value.to_string())
}

fn parse_string_array(value: &str) -> Vec<String> {
    let value = value.trim();
    if !(value.starts_with('[') && value.ends_with(']')) {
        return Vec::new();
    }
    split_toml_level(&value[1..value.len() - 1], ',')
        .into_iter()
        .filter_map(|item| parse_toml_string(item.trim()))
        .collect()
}

fn parse_toml_string(value: &str) -> Option<String> {
    let value = value.trim();
    if value.len() < 2 {
        return None;
    }
    if value.starts_with('"') && value.ends_with('"') {
        return Some(unescape_basic_string(&value[1..value.len() - 1]));
    }
    if value.starts_with('\'') && value.ends_with('\'') {
        return Some(value[1..value.len() - 1].to_string());
    }
    None
}

fn unescape_basic_string(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

fn split_key_value(line: &str) -> Option<(&str, &str)> {
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    for (index, ch) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if in_double {
            if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_double = false;
            }
            continue;
        }
        if in_single {
            if ch == '\'' {
                in_single = false;
            }
            continue;
        }
        match ch {
            '"' => in_double = true,
            '\'' => in_single = true,
            '=' => return Some((&line[..index], &line[index + 1..])),
            _ => {}
        }
    }
    None
}

fn split_toml_level(value: &str, delimiter: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut in_single = false;
    let mut in_double = false;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut escaped = false;
    for (index, ch) in value.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if in_double {
            if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_double = false;
            }
            continue;
        }
        if in_single {
            if ch == '\'' {
                in_single = false;
            }
            continue;
        }
        match ch {
            '"' => in_double = true,
            '\'' => in_single = true,
            '[' => bracket_depth += 1,
            ']' if bracket_depth > 0 => bracket_depth -= 1,
            '{' => brace_depth += 1,
            '}' if brace_depth > 0 => brace_depth -= 1,
            c if c == delimiter && bracket_depth == 0 && brace_depth == 0 => {
                parts.push(value[start..index].trim());
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(value[start..].trim());
    parts
}

fn strip_toml_comment(line: &str) -> String {
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    for (index, ch) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if in_double {
            if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_double = false;
            }
            continue;
        }
        if in_single {
            if ch == '\'' {
                in_single = false;
            }
            continue;
        }
        match ch {
            '"' => in_double = true,
            '\'' => in_single = true,
            '#' => return line[..index].to_string(),
            _ => {}
        }
    }
    line.to_string()
}

fn logical_toml_lines(input: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for raw_line in input.lines() {
        let line = strip_toml_comment(raw_line).trim().to_string();
        if line.is_empty() {
            continue;
        }

        if current.is_empty() {
            current = line;
        } else {
            current.push(' ');
            current.push_str(&line);
        }

        if toml_delimiters_balanced(&current) {
            lines.push(std::mem::take(&mut current));
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines
}

fn toml_delimiters_balanced(value: &str) -> bool {
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;

    for ch in value.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if in_double {
            if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_double = false;
            }
            continue;
        }
        if in_single {
            if ch == '\'' {
                in_single = false;
            }
            continue;
        }
        match ch {
            '"' => in_double = true,
            '\'' => in_single = true,
            '[' => bracket_depth += 1,
            ']' if bracket_depth > 0 => bracket_depth -= 1,
            '{' => brace_depth += 1,
            '}' if brace_depth > 0 => brace_depth -= 1,
            _ => {}
        }
    }

    bracket_depth == 0 && brace_depth == 0 && !in_single && !in_double
}

fn normalize_key(key: &str) -> String {
    parse_toml_string(key.trim()).unwrap_or_else(|| key.trim().to_string())
}

fn dedupe_sort(values: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    values.retain(|value| !value.is_empty() && seen.insert(value.clone()));
}
