use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use crate::parser::{self, ParserPool};
use crate::rust_edit::{RustItemKind, scan_items};
use crate::uir::CallKind;

#[derive(Debug, Clone)]
pub struct AnalyzeOptions {
    pub workspace_root: PathBuf,
    pub package: Option<String>,
    pub path: Option<PathBuf>,
    pub configs: Vec<String>,
    pub include_public: bool,
    pub include_tests_as_roots: bool,
    pub top: usize,
}

#[derive(Debug, Clone)]
pub struct AnalyzedConfig {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct DeadCodeReport {
    pub workspace_root: PathBuf,
    pub package: Option<String>,
    pub configs: Vec<AnalyzedConfig>,
    pub file_count: usize,
    pub function_count: usize,
    pub edge_count: usize,
    pub inactive_cfg_items: Vec<DeadItem>,
    pub dead_functions: Vec<DeadItem>,
    pub unused_crates: Vec<UnusedCrate>,
}

#[derive(Debug, Clone)]
pub struct DeadItem {
    pub kind: DeadItemKind,
    pub file: String,
    pub name: String,
    pub line: usize,
    pub lines: usize,
    pub start: usize,
    pub end: usize,
    pub cfg: String,
    pub active_config_count: usize,
    pub outgoing_edges: usize,
    pub is_public: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadItemKind {
    InactiveCfgItem,
    DeadFunction,
}

#[derive(Debug, Clone)]
pub struct UnusedCrate {
    pub name: String,
    pub manifest_path: PathBuf,
    pub package_dir: PathBuf,
}

#[derive(Debug, Default, Clone)]
pub struct DeleteOptions {
    pub apply: bool,
    pub kinds: Vec<DeleteKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteKind {
    InactiveCfgItem,
    DeadFunction,
    UnusedCrate,
}

#[derive(Debug, Default, Clone)]
pub struct DeleteSummary {
    pub removed_items: usize,
    pub removed_crates: usize,
    pub bytes_removed: usize,
}

#[derive(Debug, Clone)]
struct Config {
    name: String,
    features: BTreeSet<String>,
    symbols: BTreeSet<String>,
    values: BTreeMap<String, String>,
    default_features: bool,
    all_features: bool,
    test: bool,
    target_arch: String,
    target_os: String,
}

#[derive(Debug, Clone)]
struct SourceFile {
    path: String,
    source: String,
}

#[derive(Debug, Clone)]
struct FunctionInfo {
    id: String,
    name: String,
    file: String,
    start: usize,
    end: usize,
    line: usize,
    lines: usize,
    cfgs: Vec<String>,
    is_public: bool,
    is_test: bool,
}

#[derive(Debug, Clone)]
struct CfgItem {
    file: String,
    name: String,
    start: usize,
    end: usize,
    line: usize,
    lines: usize,
    cfgs: Vec<String>,
}

#[derive(Debug, Default, Clone)]
struct MetadataPackage {
    name: String,
    manifest_path: PathBuf,
    features: BTreeSet<String>,
    build_cfg_symbols: BTreeSet<String>,
    build_cfg_values: BTreeMap<String, String>,
    dependency_names: Vec<String>,
    is_workspace: bool,
    is_default_workspace: bool,
}

struct WorkspaceMemory {
    vfs: crate::vfs::VirtualFileSystem,
    packages: Vec<MetadataPackage>,
}

pub fn analyze_dead_code(opts: &AnalyzeOptions) -> Result<DeadCodeReport, String> {
    let workspace_root = opts
        .workspace_root
        .canonicalize()
        .map_err(|err| format!("invalid workspace root: {err}"))?;
    let workspace = load_workspace_memory(&workspace_root)?;
    let selected = select_package(opts, &workspace.packages)?;
    let configs = build_configs(opts, selected.as_ref())?;
    let files = collect_selected_sources(opts, selected.as_ref(), &workspace.vfs)?;
    let parsed = analyze_files(&files)?;

    let mut active_by_config: Vec<HashSet<String>> = Vec::new();
    let mut incoming_by_config: Vec<HashMap<String, usize>> = Vec::new();
    let mut outgoing_by_config: Vec<HashMap<String, usize>> = Vec::new();
    let mut reachable_union: HashSet<String> = HashSet::new();

    for config in &configs {
        let active = parsed
            .functions
            .iter()
            .filter(|function| cfgs_active(&function.cfgs, config))
            .map(|function| function.id.clone())
            .collect::<HashSet<_>>();
        let (incoming, outgoing, reachable) = reachability(&parsed, &active, config, opts);
        reachable_union.extend(reachable);
        active_by_config.push(active);
        incoming_by_config.push(incoming);
        outgoing_by_config.push(outgoing);
    }

    let mut inactive_cfg_items = parsed
        .cfg_items
        .iter()
        .filter(|item| !configs.iter().any(|config| cfgs_active(&item.cfgs, config)))
        .map(|item| DeadItem {
            kind: DeadItemKind::InactiveCfgItem,
            file: item.file.clone(),
            name: item.name.clone(),
            line: item.line,
            lines: item.lines,
            start: item.start,
            end: item.end,
            cfg: item.cfgs.join(" && "),
            active_config_count: 0,
            outgoing_edges: 0,
            is_public: false,
        })
        .collect::<Vec<_>>();
    inactive_cfg_items.sort_by(|a, b| b.lines.cmp(&a.lines).then(a.file.cmp(&b.file)));
    inactive_cfg_items.truncate(opts.top);

    let mut dead_functions = Vec::new();
    for function in &parsed.functions {
        let active_count = active_by_config
            .iter()
            .filter(|active| active.contains(&function.id))
            .count();
        if active_count == 0 || reachable_union.contains(&function.id) {
            continue;
        }
        if function.is_test && opts.include_tests_as_roots {
            continue;
        }
        if function.is_public && !opts.include_public {
            continue;
        }
        let incoming = incoming_by_config
            .iter()
            .map(|counts| counts.get(&function.id).copied().unwrap_or(0))
            .max()
            .unwrap_or(0);
        if incoming != 0 {
            continue;
        }
        let outgoing = outgoing_by_config
            .iter()
            .map(|counts| counts.get(&function.id).copied().unwrap_or(0))
            .max()
            .unwrap_or(0);
        dead_functions.push(DeadItem {
            kind: DeadItemKind::DeadFunction,
            file: function.file.clone(),
            name: function.name.clone(),
            line: function.line,
            lines: function.lines,
            start: function.start,
            end: function.end,
            cfg: function.cfgs.join(" && "),
            active_config_count: active_count,
            outgoing_edges: outgoing,
            is_public: function.is_public,
        });
    }
    dead_functions.sort_by(|a, b| b.lines.cmp(&a.lines).then(a.file.cmp(&b.file)));
    let mut seen_dead = HashSet::new();
    dead_functions.retain(|item| {
        seen_dead.insert((item.file.clone(), item.start, item.end, item.name.clone()))
    });
    dead_functions.truncate(opts.top);

    let unused_crates = unused_workspace_crates(&workspace.packages, selected.as_ref());

    Ok(DeadCodeReport {
        workspace_root,
        package: selected.as_ref().map(|pkg| pkg.name.clone()),
        configs: configs
            .iter()
            .map(|config| AnalyzedConfig {
                name: config.name.clone(),
            })
            .collect(),
        file_count: files.len(),
        function_count: parsed.functions.len(),
        edge_count: parsed.edges.len(),
        inactive_cfg_items,
        dead_functions,
        unused_crates,
    })
}

pub fn apply_deletions(
    report: &DeadCodeReport,
    opts: &DeleteOptions,
) -> Result<DeleteSummary, String> {
    if !opts.apply {
        return Ok(DeleteSummary::default());
    }

    let mut summary = DeleteSummary::default();
    let mut by_file: BTreeMap<String, Vec<&DeadItem>> = BTreeMap::new();
    for item in report
        .inactive_cfg_items
        .iter()
        .chain(report.dead_functions.iter())
    {
        let selected = match item.kind {
            DeadItemKind::InactiveCfgItem => opts.kinds.contains(&DeleteKind::InactiveCfgItem),
            DeadItemKind::DeadFunction => opts.kinds.contains(&DeleteKind::DeadFunction),
        };
        if selected {
            by_file.entry(item.file.clone()).or_default().push(item);
        }
    }

    for (file, mut items) in by_file {
        items.sort_by(|a, b| b.start.cmp(&a.start));
        let path = report.workspace_root.join(&file);
        let mut source = fs::read_to_string(&path)
            .map_err(|err| format!("read {} failed: {err}", path.display()))?;
        let mut removed = 0usize;
        for item in items {
            if item.end > source.len() || item.start >= item.end {
                return Err(format!("stale range for {}:{}", item.file, item.line));
            }
            removed += item.end - item.start;
            source.replace_range(item.start..item.end, "");
            summary.removed_items += 1;
        }
        fs::write(&path, source)
            .map_err(|err| format!("write {} failed: {err}", path.display()))?;
        summary.bytes_removed += removed;
    }

    if opts.kinds.contains(&DeleteKind::UnusedCrate) {
        let mut removed_member_paths = Vec::new();
        for krate in &report.unused_crates {
            let package_dir = report.workspace_root.join(&krate.package_dir);
            if package_dir.exists() {
                let bytes = dir_size(&package_dir)?;
                fs::remove_dir_all(&package_dir)
                    .map_err(|err| format!("remove {} failed: {err}", package_dir.display()))?;
                removed_member_paths.push(krate.package_dir.to_string_lossy().replace('\\', "/"));
                summary.bytes_removed += bytes;
                summary.removed_crates += 1;
            }
        }
        if !removed_member_paths.is_empty() {
            remove_workspace_member_lines(
                &report.workspace_root.join("Cargo.toml"),
                &removed_member_paths,
            )?;
        }
    }

    Ok(summary)
}

#[derive(Debug)]
struct ParsedSources {
    functions: Vec<FunctionInfo>,
    edges: Vec<(String, String, CallKind)>,
    cfg_items: Vec<CfgItem>,
}

fn analyze_files(files: &[SourceFile]) -> Result<ParsedSources, String> {
    let mut pool = ParserPool::new();
    let mut functions = Vec::new();
    let mut file_name_to_ids: HashMap<String, HashMap<String, Vec<String>>> = HashMap::new();
    let mut name_to_ids: HashMap<String, Vec<String>> = HashMap::new();
    let mut calls_by_file = Vec::new();
    let mut cfg_items = Vec::new();

    for file in files {
        let rust_items = if file.path.ends_with(".rs") {
            scan_items(&file.source)
        } else {
            Vec::new()
        };
        cfg_items.extend(collect_inactive_candidate_items(file, &rust_items));
        let parsed = pool
            .parse_file(&file.path, &file.source)
            .ok_or_else(|| format!("unsupported source file: {}", file.path))?
            .to_owned();
        let mut used_names: HashMap<String, usize> = HashMap::new();
        for raw in &parsed.functions {
            let suffix = used_names.entry(raw.name.clone()).or_insert(0);
            let occurrence = *suffix;
            *suffix += 1;
            let item = rust_items
                .iter()
                .filter(|item| item.kind == RustItemKind::Fn && item.name == raw.name)
                .find(|item| ranges_overlap(item.start, item.end, raw.start_byte, raw.end_byte));
            let start = item.map(|item| item.start).unwrap_or(raw.start_byte);
            let end = item.map(|item| item.end).unwrap_or(raw.end_byte);
            let signature_end = item
                .and_then(|item| item.signature.map(|(_, end)| end))
                .unwrap_or(raw.start_byte);
            let signature = &file.source[start..signature_end.min(file.source.len())];
            let cfgs = effective_cfgs(&file.source, &rust_items, start, end);
            let name = if occurrence == 0 {
                raw.name.clone()
            } else {
                format!("{}#{occurrence}", raw.name)
            };
            let id = format!("{}::{name}", file.path);
            let info = FunctionInfo {
                id: id.clone(),
                name: raw.name.clone(),
                file: file.path.clone(),
                start,
                end,
                line: line_number(&file.source, start),
                lines: line_count(&file.source[start..end.min(file.source.len())]),
                cfgs,
                is_public: signature_contains_pub(signature),
                is_test: has_test_attr(&file.source[start..signature_end.min(file.source.len())]),
            };
            file_name_to_ids
                .entry(file.path.clone())
                .or_default()
                .entry(raw.name.clone())
                .or_default()
                .push(id.clone());
            name_to_ids.entry(raw.name.clone()).or_default().push(id);
            functions.push(info);
        }
        calls_by_file.push((file.path.clone(), parsed.functions, parsed.calls));
    }

    let mut edges = Vec::new();
    for (file, raw_functions, raw_calls) in calls_by_file {
        for (caller_name, call) in parser::build_call_pairs_owned(&raw_functions, &raw_calls) {
            let Some(caller_id) = file_name_to_ids
                .get(&file)
                .and_then(|names| names.get(&caller_name))
                .and_then(|ids| ids.first())
            else {
                continue;
            };
            let Some(callee_id) =
                resolve_callee(&file, &call.callee_name, &file_name_to_ids, &name_to_ids)
            else {
                continue;
            };
            edges.push((caller_id.clone(), callee_id, call.kind));
        }
    }

    Ok(ParsedSources {
        functions,
        edges,
        cfg_items,
    })
}

fn reachability(
    parsed: &ParsedSources,
    active: &HashSet<String>,
    config: &Config,
    opts: &AnalyzeOptions,
) -> (
    HashMap<String, usize>,
    HashMap<String, usize>,
    HashSet<String>,
) {
    let function_by_id = parsed
        .functions
        .iter()
        .map(|function| (function.id.as_str(), function))
        .collect::<HashMap<_, _>>();
    let mut incoming = HashMap::new();
    let mut outgoing = HashMap::new();
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    for (caller, callee, _kind) in &parsed.edges {
        if active.contains(caller) && active.contains(callee) {
            *incoming.entry(callee.clone()).or_insert(0) += 1;
            *outgoing.entry(caller.clone()).or_insert(0) += 1;
            graph
                .entry(caller.clone())
                .or_default()
                .push(callee.clone());
        }
    }

    let mut roots = VecDeque::new();
    for function in &parsed.functions {
        if !active.contains(&function.id) {
            continue;
        }
        if function.name == "main"
            || function.is_public
            || (opts.include_tests_as_roots && config.test && function.is_test)
        {
            roots.push_back(function.id.clone());
        }
    }

    let mut reachable = HashSet::new();
    while let Some(id) = roots.pop_front() {
        if !reachable.insert(id.clone()) {
            continue;
        }
        let Some(function) = function_by_id.get(id.as_str()) else {
            continue;
        };
        for callee in graph.get(&function.id).into_iter().flatten() {
            roots.push_back(callee.clone());
        }
    }
    (incoming, outgoing, reachable)
}

fn collect_inactive_candidate_items(
    file: &SourceFile,
    items: &[crate::rust_edit::RustItem],
) -> Vec<CfgItem> {
    items
        .iter()
        .filter(|item| matches!(item.kind, RustItemKind::Mod | RustItemKind::Fn))
        .filter_map(|item| {
            let cfgs = item_direct_cfgs(&file.source, item);
            if cfgs.is_empty() {
                return None;
            }
            Some(CfgItem {
                file: file.path.clone(),
                name: format!("{:?} {}", item.kind, item.name),
                start: item.start,
                end: item.end,
                line: line_number(&file.source, item.start),
                lines: line_count(&file.source[item.start..item.end.min(file.source.len())]),
                cfgs,
            })
        })
        .collect()
}

fn effective_cfgs(
    source: &str,
    items: &[crate::rust_edit::RustItem],
    start: usize,
    end: usize,
) -> Vec<String> {
    let mut cfgs = Vec::new();
    for item in items {
        if item.start <= start && item.end >= end {
            cfgs.extend(item_direct_cfgs(source, item));
        }
    }
    cfgs.sort();
    cfgs.dedup();
    cfgs
}

fn item_direct_cfgs(source: &str, item: &crate::rust_edit::RustItem) -> Vec<String> {
    let keyword = match item.kind {
        RustItemKind::Fn => "fn",
        RustItemKind::Struct => "struct",
        RustItemKind::Enum => "enum",
        RustItemKind::Trait => "trait",
        RustItemKind::Impl => "impl",
        RustItemKind::Mod => "mod",
        RustItemKind::Use => "use",
        RustItemKind::Const => "const",
        RustItemKind::Static => "static",
        RustItemKind::Type => "type",
    };
    let search = &source[item.start..item.end.min(source.len())];
    let prefix_end = search
        .find(keyword)
        .map(|offset| item.start + offset)
        .unwrap_or(item.start);
    let prefix = &source[item.start..prefix_end];
    extract_cfg_attrs(prefix)
}

fn extract_cfg_attrs(prefix: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut offset = 0;
    while let Some(rel) = prefix[offset..].find("#[cfg(") {
        let start = offset + rel + "#[cfg(".len();
        let Some(end) = find_matching_paren(prefix, start.saturating_sub(1)) else {
            break;
        };
        out.push(prefix[start..end].trim().to_string());
        offset = end + 1;
    }
    out
}

fn cfgs_active(cfgs: &[String], config: &Config) -> bool {
    cfgs.iter().all(|expr| eval_cfg(expr, config))
}

fn eval_cfg(expr: &str, config: &Config) -> bool {
    let expr = expr.trim();
    if expr.is_empty() {
        return true;
    }
    if let Some(inner) = strip_call(expr, "all") {
        return split_cfg_args(inner)
            .iter()
            .all(|arg| eval_cfg(arg, config));
    }
    if let Some(inner) = strip_call(expr, "any") {
        return split_cfg_args(inner)
            .iter()
            .any(|arg| eval_cfg(arg, config));
    }
    if let Some(inner) = strip_call(expr, "not") {
        return !eval_cfg(inner, config);
    }
    if expr == "test" {
        return config.test;
    }
    if expr == "unix" {
        return config.target_os != "windows";
    }
    if expr == "windows" {
        return config.target_os == "windows";
    }
    if let Some(value) = cfg_equals(expr, "feature") {
        return config.all_features || config.features.contains(value);
    }
    if let Some(value) = cfg_equals(expr, "target_arch") {
        return config.target_arch == value;
    }
    if let Some(value) = cfg_equals(expr, "target_os") {
        return config.target_os == value;
    }
    if let Some((key, value)) = expr.split_once('=') {
        return config
            .values
            .get(key.trim())
            .is_some_and(|configured| configured == value.trim().trim_matches('"'));
    }
    config.symbols.contains(expr)
}

fn strip_call<'a>(expr: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{name}(");
    expr.strip_prefix(&prefix)?.strip_suffix(')')
}

fn cfg_equals<'a>(expr: &'a str, key: &str) -> Option<&'a str> {
    let (left, right) = expr.split_once('=')?;
    if left.trim() != key {
        return None;
    }
    Some(right.trim().trim_matches('"'))
}

fn split_cfg_args(input: &str) -> Vec<&str> {
    let mut args = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (idx, ch) in input.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                let arg = input[start..idx].trim();
                if !arg.is_empty() {
                    args.push(arg);
                }
                start = idx + 1;
            }
            _ => {}
        }
    }
    let tail = input[start..].trim();
    if !tail.is_empty() {
        args.push(tail);
    }
    args
}

fn build_configs(
    opts: &AnalyzeOptions,
    package: Option<&MetadataPackage>,
) -> Result<Vec<Config>, String> {
    let mut specs = opts.configs.clone();
    if specs.is_empty() {
        specs.extend(
            ["default", "all", "none", "test"]
                .into_iter()
                .map(str::to_string),
        );
        if let Some(package) = package {
            specs.extend(
                package
                    .features
                    .iter()
                    .map(|feature| format!("feature:{feature}")),
            );
        }
    }
    let default_features = package
        .map(|pkg| pkg.features.contains("default"))
        .unwrap_or(false);
    let mut configs = Vec::new();
    for spec in specs {
        let mut config = Config {
            name: spec.clone(),
            features: BTreeSet::new(),
            symbols: BTreeSet::new(),
            values: BTreeMap::new(),
            default_features: false,
            all_features: false,
            test: false,
            target_arch: "x86_64".to_string(),
            target_os: "linux".to_string(),
        };
        if let Some(package) = package {
            config
                .symbols
                .extend(package.build_cfg_symbols.iter().cloned());
            config.values.extend(
                package
                    .build_cfg_values
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone())),
            );
        }
        match spec.as_str() {
            "default" => {
                config.default_features = default_features;
                if default_features {
                    config.features.insert("default".to_string());
                }
            }
            "all" | "all-features" => config.all_features = true,
            "none" | "no-default-features" => {}
            "test" => {
                config.test = true;
                config.symbols.insert("test".to_string());
                config.default_features = default_features;
                if default_features {
                    config.features.insert("default".to_string());
                }
            }
            "wasm" => {
                config.name = "target:wasm32-unknown-unknown".to_string();
                config.target_arch = "wasm32".to_string();
                config.target_os = "unknown".to_string();
            }
            value if value.starts_with("feature:") => {
                config
                    .features
                    .insert(value["feature:".len()..].to_string());
            }
            value if value.starts_with("cfg:") => {
                config
                    .symbols
                    .insert(value["cfg:".len()..].trim().to_string());
            }
            value if value.starts_with("target:") => {
                let target = &value["target:".len()..];
                if let Some((arch, rest)) = target.split_once('-') {
                    config.target_arch = arch.to_string();
                    config.target_os = if rest.contains("windows") {
                        "windows"
                    } else if rest.contains("apple") || rest.contains("darwin") {
                        "macos"
                    } else if rest.contains("unknown") {
                        "unknown"
                    } else {
                        "linux"
                    }
                    .to_string();
                }
            }
            other => return Err(format!("unknown config spec: {other}")),
        }
        configs.push(config);
    }
    Ok(configs)
}

fn load_workspace_memory(workspace_root: &Path) -> Result<WorkspaceMemory, String> {
    let root = workspace_root
        .to_str()
        .ok_or_else(|| "workspace root is not utf-8".to_string())?;
    let vfs = crate::filesystem::load_vfs_from_dir(
        root,
        &[
            ".git",
            ".next",
            ".turbo",
            "build",
            "coverage",
            "dist",
            "node_modules",
            "out",
            "target",
        ],
    )?;
    let packages = index_workspace_manifests(&vfs)?;
    Ok(WorkspaceMemory { vfs, packages })
}

fn index_workspace_manifests(
    vfs: &crate::vfs::VirtualFileSystem,
) -> Result<Vec<MetadataPackage>, String> {
    use crate::codealyzer::cargo_toml_projection::parse_cargo_toml_projection;

    let root_manifest = vfs
        .read_str("Cargo.toml")
        .ok_or_else(|| "workspace Cargo.toml not loaded in VFS".to_string())?;
    let root_projection = parse_cargo_toml_projection(root_manifest);
    if root_projection.workspace_members.is_empty() {
        return Err("root Cargo.toml has no [workspace].members projection".to_string());
    }
    let member_patterns = root_projection.workspace_members;
    let default_patterns = root_projection
        .workspace_default_members
        .into_iter()
        .collect::<BTreeSet<_>>();

    let mut member_dirs = BTreeSet::new();
    for pattern in &member_patterns {
        expand_member_pattern(vfs, pattern, &mut member_dirs);
    }

    let mut packages = Vec::new();
    for dir in member_dirs {
        let manifest_path = format!("{dir}/Cargo.toml");
        let Some(manifest) = vfs.read_str(&manifest_path) else {
            continue;
        };
        let projection = parse_cargo_toml_projection(manifest);
        let Some(name) = projection.package_name.clone() else {
            continue;
        };
        let features = projection.feature_keys.clone();
        let dependency_names = collect_manifest_dependencies(&projection);
        let (build_cfg_symbols, build_cfg_values) = read_build_cfgs(vfs, &dir);
        packages.push(MetadataPackage {
            name,
            manifest_path: PathBuf::from(&manifest_path),
            features,
            build_cfg_symbols,
            build_cfg_values,
            dependency_names,
            is_workspace: true,
            is_default_workspace: default_patterns.contains(&dir),
        });
    }
    if packages.iter().all(|package| !package.is_default_workspace) {
        for package in &mut packages {
            package.is_default_workspace = true;
        }
    }
    Ok(packages)
}

fn select_package(
    opts: &AnalyzeOptions,
    packages: &[MetadataPackage],
) -> Result<Option<MetadataPackage>, String> {
    if let Some(name) = &opts.package {
        return packages
            .iter()
            .find(|package| package.name == *name)
            .cloned()
            .map(Some)
            .ok_or_else(|| format!("package not found: {name}"));
    }
    if let Some(path) = &opts.path {
        let root = normalize_vfs_path(&path.to_string_lossy());
        return Ok(packages
            .iter()
            .filter(|package| package.is_workspace)
            .find(|package| {
                package
                    .manifest_path
                    .parent()
                    .is_some_and(|parent| parent.to_string_lossy().starts_with(&root))
            })
            .cloned());
    }
    Ok(None)
}

fn collect_selected_sources(
    opts: &AnalyzeOptions,
    package: Option<&MetadataPackage>,
    vfs: &crate::vfs::VirtualFileSystem,
) -> Result<Vec<SourceFile>, String> {
    let root = if let Some(path) = &opts.path {
        normalize_vfs_path(&path.to_string_lossy())
    } else if let Some(package) = package {
        package
            .manifest_path
            .parent()
            .ok_or_else(|| "package manifest has no parent".to_string())?
            .to_string_lossy()
            .to_string()
    } else {
        String::new()
    };
    let mut files = Vec::new();
    for (path, content) in vfs.files() {
        if !path.ends_with(".rs") {
            continue;
        }
        if !root.is_empty() && !path.starts_with(&root) {
            continue;
        }
        let Ok(source) = std::str::from_utf8(content) else {
            continue;
        };
        files.push(SourceFile {
            path: path.clone(),
            source: source.to_string(),
        });
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

fn read_build_cfgs(
    vfs: &crate::vfs::VirtualFileSystem,
    package_dir: &str,
) -> (BTreeSet<String>, BTreeMap<String, String>) {
    let build_path = format!("{package_dir}/build.rs");
    let Some(source) = vfs.read_str(&build_path) else {
        return (BTreeSet::new(), BTreeMap::new());
    };
    let mut symbols = BTreeSet::new();
    let mut values = BTreeMap::new();
    for line in source.lines() {
        let Some(pos) = line.find("cargo:rustc-cfg=") else {
            continue;
        };
        let cfg = &line[pos + "cargo:rustc-cfg=".len()..];
        let cfg = cfg
            .trim_matches(|ch| matches!(ch, '"' | '\'' | ')' | ';' | ' ' | '\t'))
            .trim();
        if cfg.is_empty() || cfg.contains('{') {
            continue;
        }
        if let Some((key, value)) = cfg.split_once('=') {
            values.insert(
                key.trim().to_string(),
                value.trim().trim_matches('"').to_string(),
            );
        } else {
            symbols.insert(cfg.to_string());
        }
    }
    (symbols, values)
}

fn expand_member_pattern(
    vfs: &crate::vfs::VirtualFileSystem,
    pattern: &str,
    out: &mut BTreeSet<String>,
) {
    let pattern = normalize_vfs_path(pattern);
    if !pattern.contains('*') && !pattern.contains('?') {
        if vfs.read_str(&format!("{pattern}/Cargo.toml")).is_some() {
            out.insert(pattern);
        }
        return;
    }
    for (path, _) in vfs.files() {
        let Some(dir) = path.strip_suffix("/Cargo.toml") else {
            continue;
        };
        if crate::glob::glob_match(&pattern, dir) {
            out.insert(dir.to_string());
        }
    }
}

fn collect_manifest_dependencies(
    projection: &crate::codealyzer::cargo_toml_projection::CargoTomlProjection,
) -> Vec<String> {
    let mut names = BTreeSet::new();
    for row in &projection.dependencies {
        names.insert(row.name.clone());
    }
    for row in &projection.dev_dependencies {
        names.insert(row.name.clone());
    }
    for row in &projection.build_dependencies {
        names.insert(row.name.clone());
    }
    names.into_iter().collect()
}

fn normalize_vfs_path(path: &str) -> String {
    path.trim()
        .trim_start_matches("./")
        .trim_matches('/')
        .replace('\\', "/")
}

fn unused_workspace_crates(
    packages: &[MetadataPackage],
    selected: Option<&MetadataPackage>,
) -> Vec<UnusedCrate> {
    if selected.is_some() {
        return Vec::new();
    }
    let package_by_name = packages
        .iter()
        .filter(|package| package.is_workspace)
        .map(|package| (package.name.clone(), package))
        .collect::<HashMap<_, _>>();
    let mut reachable = HashSet::new();
    let mut queue = VecDeque::new();
    for package in packages {
        if package.is_workspace && package.is_default_workspace {
            queue.push_back(package.name.clone());
        }
    }
    while let Some(name) = queue.pop_front() {
        if !reachable.insert(name.clone()) {
            continue;
        }
        let Some(package) = package_by_name.get(&name) else {
            continue;
        };
        for dep in &package.dependency_names {
            if package_by_name.contains_key(dep) {
                queue.push_back(dep.clone());
            }
        }
    }
    let mut unused = packages
        .iter()
        .filter(|package| package.is_workspace)
        .filter(|package| !package.is_default_workspace)
        .filter(|package| !reachable.contains(&package.name))
        .filter_map(|package| {
            let package_dir = package.manifest_path.parent()?.to_path_buf();
            Some(UnusedCrate {
                name: package.name.clone(),
                manifest_path: package.manifest_path.clone(),
                package_dir,
            })
        })
        .collect::<Vec<_>>();
    unused.sort_by(|a, b| a.name.cmp(&b.name));
    unused
}

fn resolve_callee(
    file: &str,
    name: &str,
    file_name_to_ids: &HashMap<String, HashMap<String, Vec<String>>>,
    name_to_ids: &HashMap<String, Vec<String>>,
) -> Option<String> {
    file_name_to_ids
        .get(file)
        .and_then(|names| names.get(name))
        .and_then(|ids| ids.first())
        .cloned()
        .or_else(|| name_to_ids.get(name).and_then(|ids| ids.first()).cloned())
}

fn find_matching_paren(input: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (idx, ch) in input.char_indices().skip_while(|(idx, _)| *idx < open) {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(idx);
                }
            }
            _ => {}
        }
    }
    None
}

fn ranges_overlap(a_start: usize, a_end: usize, b_start: usize, b_end: usize) -> bool {
    a_start < b_end && b_start < a_end
}

fn signature_contains_pub(signature: &str) -> bool {
    signature
        .split(|ch: char| ch.is_whitespace() || matches!(ch, '(' | '{' | '<'))
        .any(|part| part == "pub" || part.starts_with("pub("))
}

fn has_test_attr(prefix: &str) -> bool {
    prefix.contains("#[test]")
        || prefix.contains("#[tokio::test]")
        || prefix.contains("#[async_std::test]")
}

fn line_number(source: &str, byte: usize) -> usize {
    source[..byte.min(source.len())]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

fn line_count(source: &str) -> usize {
    source.lines().count().max(1)
}

fn dir_size(path: &Path) -> Result<usize, String> {
    let mut total = 0usize;
    for entry in
        fs::read_dir(path).map_err(|err| format!("read {} failed: {err}", path.display()))?
    {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            total += dir_size(&path)?;
        } else {
            total += fs::metadata(&path)
                .map_err(|err| format!("stat {} failed: {err}", path.display()))?
                .len() as usize;
        }
    }
    Ok(total)
}

fn remove_workspace_member_lines(manifest: &Path, member_paths: &[String]) -> Result<(), String> {
    let source = fs::read_to_string(manifest)
        .map_err(|err| format!("read {} failed: {err}", manifest.display()))?;
    let mut output = String::new();
    for line in source.lines() {
        let trimmed = line.trim().trim_end_matches(',');
        let quoted = trimmed.trim_matches('"');
        if member_paths.iter().any(|path| quoted == path) {
            continue;
        }
        output.push_str(line);
        output.push('\n');
    }
    fs::write(manifest, output).map_err(|err| format!("write {} failed: {err}", manifest.display()))
}
