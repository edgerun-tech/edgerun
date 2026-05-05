use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::fs;
use std::path::Path;
use std::process::Command;

use chrono::offset::Local;

use crate::codealyzer::errors::AnalyzerError;
use edgerun_json::{to_json_string, JsonValue, Map, ToJson};

#[derive(Clone, Copy)]
struct Footprint {
    size_bytes: u64,
    loc: u64,
}

#[derive(Clone)]
struct PackageInfo {
    id: String,
    name: String,
    version: String,
    manifest_path: String,
    source: Option<String>,
    path_override: Option<String>,
    crate_type: String,
    is_workspace: bool,
    direct_dependencies: Vec<String>,
}

#[derive(Clone)]
struct DependencyRecord {
    name: String,
    id: String,
    version: String,
    source: String,
    source_ref: Option<String>,
    manifest_path: String,
    size_bytes: u64,
    size_mb: f64,
    loc: u64,
    is_direct: bool,
    size_ratio_to_self_percent: f64,
    loc_ratio_to_self_percent: f64,
}

#[derive(Clone)]
struct DependencyTotals {
    count: usize,
    size_bytes: u64,
    loc: u64,
}

#[derive(Clone)]
struct CrateDependencyProfile {
    name: String,
    path: String,
    crate_type: String,
    self_size_bytes: u64,
    self_loc: u64,
    self_size_mb: f64,
    all_dependencies: Vec<DependencyRecord>,
    direct_dependencies: Vec<DependencyRecord>,
    all_totals: DependencyTotals,
    direct_totals: DependencyTotals,
}

#[derive(Clone)]
struct ExternalDependencyUsage {
    id: String,
    name: String,
    version: String,
    source: String,
    source_ref: Option<String>,
    manifest_path: String,
    size_bytes: u64,
    size_mb: f64,
    loc: u64,
    dependent_crates: BTreeSet<String>,
    direct_dependent_crates: BTreeSet<String>,
}

pub fn collect_dependency_footprints(workspace_root: &Path) -> Result<JsonValue, AnalyzerError> {
    let workspace_root = workspace_root
        .canonicalize()
        .unwrap_or_else(|_| workspace_root.to_path_buf());

    let metadata = load_cargo_metadata(&workspace_root)?;
    let packages_root = metadata.get("packages").and_then(JsonValue::as_array);
    let Some(packages_root) = packages_root else {
        return Ok(empty_report(&workspace_root));
    };

    let packages_by_id = collect_packages(packages_root, &workspace_root);
    if packages_by_id.is_empty() {
        return Ok(empty_report(&workspace_root));
    }

    let members = metadata
        .get("workspace_members")
        .and_then(JsonValue::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(JsonValue::as_str)
                .map(|value| value.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let dependency_edges = parse_resolve_edges(&metadata);
    let mut name_to_ids: HashMap<String, Vec<String>> = HashMap::new();
    for package in packages_by_id.values() {
        name_to_ids
            .entry(package.name.clone())
            .or_default()
            .push(package.id.clone());
    }

    let mut package_footprint_cache: HashMap<String, Footprint> = HashMap::new();
    let mut crate_profiles: Vec<CrateDependencyProfile> = Vec::new();
    let mut global_dependency_usage: HashMap<String, ExternalDependencyUsage> = HashMap::new();

    for member_id in members {
        let Some(member_package) = packages_by_id.get(&member_id) else {
            continue;
        };
        if !member_package.is_workspace {
            continue;
        }

        let self_fp = estimate_package_footprint(
            &member_package.manifest_path,
            &mut package_footprint_cache,
        );

        if self_fp.size_bytes == 0 && self_fp.loc == 0 {
            // Workspace manifests can become unavailable in partial checkouts. Keep analysis
            // resilient and treat them as an empty package instead of failing the full run.
            continue;
        }

        let dependency_closure = collect_dependency_closure(&member_package.id, &dependency_edges);
        let direct_dependency_ids = collect_direct_dependency_ids(member_package, &name_to_ids);

        let mut all_dependencies = Vec::new();
        let mut direct_dependencies = Vec::new();

        for dependency_id in dependency_closure {
            let Some(dependency_package) = packages_by_id.get(&dependency_id) else {
                continue;
            };

            if dependency_package.is_workspace
                || dependency_package.name.starts_with("edgerun-")
                || dependency_package.name.is_empty()
            {
                continue;
            }

            let dependency_fp = estimate_package_footprint(
                &dependency_package.manifest_path,
                &mut package_footprint_cache,
            );
            let source = classify_dependency_source(dependency_package, &workspace_root);
            let is_direct = direct_dependency_ids.contains(&dependency_package.id);
            let source_ref = dependency_package
                .source
                .clone()
                .or_else(|| dependency_package.path_override.clone());
            let source_ref_for_entry = source_ref.clone();

            let dependency = DependencyRecord {
                name: dependency_package.name.clone(),
                id: dependency_package.id.clone(),
                version: dependency_package.version.clone(),
                source,
                source_ref,
                manifest_path: dependency_package.manifest_path.clone(),
                size_bytes: dependency_fp.size_bytes,
                size_mb: bytes_to_megabytes(dependency_fp.size_bytes),
                loc: dependency_fp.loc,
                is_direct,
                size_ratio_to_self_percent: ratio_percent(dependency_fp.size_bytes, self_fp.size_bytes),
                loc_ratio_to_self_percent: ratio_percent(dependency_fp.loc, self_fp.loc),
            };

            if is_direct {
                direct_dependencies.push(dependency.clone());
            }

            let usage_entry = global_dependency_usage
                .entry(dependency_package.id.clone())
                .or_insert_with(|| ExternalDependencyUsage {
                    id: dependency_package.id.clone(),
                    name: dependency_package.name.clone(),
                    version: dependency_package.version.clone(),
                    source: dependency.source.as_str().to_string(),
                    source_ref: source_ref_for_entry,
                    manifest_path: dependency_package.manifest_path.clone(),
                    size_bytes: dependency_fp.size_bytes,
                    size_mb: bytes_to_megabytes(dependency_fp.size_bytes),
                    loc: dependency_fp.loc,
                    dependent_crates: BTreeSet::new(),
                    direct_dependent_crates: BTreeSet::new(),
                });

            usage_entry.dependent_crates.insert(member_package.name.clone());
            if is_direct {
                usage_entry
                    .direct_dependent_crates
                    .insert(member_package.name.clone());
            }

            all_dependencies.push(dependency);
        }

        all_dependencies.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));
        direct_dependencies.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));
        let all_totals = sum_dependency_totals(&all_dependencies);
        let direct_totals = sum_dependency_totals(&direct_dependencies);

        crate_profiles.push(CrateDependencyProfile {
            name: member_package.name.clone(),
            path: member_package.manifest_path.clone(),
            crate_type: member_package.crate_type.clone(),
            self_size_bytes: self_fp.size_bytes,
            self_loc: self_fp.loc,
            self_size_mb: bytes_to_megabytes(self_fp.size_bytes),
            all_dependencies,
            direct_dependencies,
            all_totals,
            direct_totals,
        });
    }

    crate_profiles.sort_by(|a, b| a.name.cmp(&b.name));

    let mut crate_entries = Vec::new();
    for profile in crate_profiles {
        crate_entries.push(format_crate_profile_to_json(profile));
    }

    let mut distinct_dependencies = Vec::new();
    for usage in global_dependency_usage.values() {
        let dependent_crates = usage
            .dependent_crates
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let direct_dependent_crates = usage
            .direct_dependent_crates
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let mut usage_obj = Map::new();
        usage_obj.insert("dependency_id".into(), usage.id.to_json());
        usage_obj.insert("name".into(), usage.name.to_json());
        usage_obj.insert("version".into(), usage.version.to_json());
        usage_obj.insert("source".into(), usage.source.to_json());
        usage_obj.insert("source_ref".into(), usage.source_ref.to_json());
        usage_obj.insert("manifest_path".into(), usage.manifest_path.to_json());
        usage_obj.insert("size_bytes".into(), usage.size_bytes.to_json());
        usage_obj.insert("size_mb".into(), usage.size_mb.to_json());
        usage_obj.insert("loc".into(), usage.loc.to_json());
        usage_obj.insert("dependent_crate_count".into(), dependent_crates.len().to_json());
        usage_obj.insert("direct_dependent_crate_count".into(), direct_dependent_crates.len().to_json());
        usage_obj.insert("dependent_crates".into(), dependent_crates.to_json());
        usage_obj.insert(
            "direct_dependent_crates".into(),
            direct_dependent_crates.to_json(),
        );
        distinct_dependencies.push(JsonValue::Object(usage_obj));
    }

    let distinct_dependency_count = distinct_dependencies.len();
    distinct_dependencies
        .sort_by(|lhs, rhs| lhs.get("size_bytes").and_then(JsonValue::as_u64).cmp(&rhs.get("size_bytes").and_then(JsonValue::as_u64)).reverse());

    let mut summary = Map::new();
    let mut global_size_bytes = 0u64;
    let mut global_loc = 0u64;
    for usage in &distinct_dependencies {
        if let Some(size) = usage.get("size_bytes").and_then(JsonValue::as_u64) {
            global_size_bytes = global_size_bytes.saturating_add(size);
        }
        if let Some(loc) = usage.get("loc").and_then(JsonValue::as_u64) {
            global_loc = global_loc.saturating_add(loc);
        }
    }

    summary.insert("workspace_root_crate_count".into(), crate_entries.len().to_json());
    summary.insert(
        "crates_with_external_dependencies".into(),
        crate_entries
            .iter()
            .filter(|raw| {
                raw.get("external_dependency_count")
                    .and_then(JsonValue::as_u64)
                    .unwrap_or_default()
                    > 0
            })
            .count()
            .to_json(),
    );
    summary.insert(
        "distinct_external_dependency_count".into(),
        distinct_dependency_count.to_json(),
    );
    summary.insert(
        "distinct_external_dependency_size_bytes".into(),
        global_size_bytes.to_json(),
    );
    summary.insert(
        "distinct_external_dependency_size_mb".into(),
        bytes_to_megabytes(global_size_bytes).to_json(),
    );
    summary.insert("distinct_external_dependency_loc".into(), global_loc.to_json());
    summary.insert(
        "scope".into(),
        "non-edgerun workspace dependencies".to_json(),
    );

    let mut top = Map::new();
    top.insert(
        "workspace_root".into(),
        workspace_root.display().to_string().to_json(),
    );
    top.insert("generated_at".into(), Local::now().to_rfc3339().to_json());
    top.insert("summary".into(), JsonValue::Object(summary));
    top.insert(
        "crates".into(),
        JsonValue::array(crate_entries),
    );
    top.insert(
        "distinct_external_dependencies".into(),
        JsonValue::array(distinct_dependencies),
    );

    Ok(JsonValue::Object(top))
}

pub fn write_dependency_footprints(
    report: &JsonValue,
    output_dir: &Path,
) -> Result<(), AnalyzerError> {
    std::fs::create_dir_all(output_dir)
        .map_err(|error| AnalyzerError::IoError(error.to_string()))?;

    let output_path = output_dir.join("dependency-metrics.json");
    let body = to_json_string(report).map_err(|error| AnalyzerError::IoError(error.to_string()))?;
    std::fs::write(output_path, body).map_err(|error| AnalyzerError::IoError(error.to_string()))
}

fn format_crate_profile_to_json(profile: CrateDependencyProfile) -> JsonValue {
    let all_dependencies = profile
        .all_dependencies
        .into_iter()
        .map(format_dependency_record_to_json)
        .collect::<Vec<_>>();
    let direct_dependencies = profile
        .direct_dependencies
        .into_iter()
        .map(format_dependency_record_to_json)
        .collect::<Vec<_>>();

    let all_total_size_percent = ratio_percent(profile.all_totals.size_bytes, profile.self_size_bytes);
    let all_total_loc_percent = ratio_percent(profile.all_totals.loc, profile.self_loc);

    let mut item = Map::new();
    item.insert("name".into(), profile.name.to_json());
    item.insert("path".into(), profile.path.to_json());
    item.insert("crate_type".into(), profile.crate_type.to_json());
    item.insert("self_size_bytes".into(), profile.self_size_bytes.to_json());
    item.insert("self_size_mb".into(), profile.self_size_mb.to_json());
    item.insert("self_loc".into(), profile.self_loc.to_json());
    item.insert(
        "external_dependency_count".into(),
        profile.all_totals.count.to_json(),
    );
    item.insert(
        "external_dependency_size_bytes".into(),
        profile.all_totals.size_bytes.to_json(),
    );
    item.insert(
        "external_dependency_size_mb".into(),
        bytes_to_megabytes(profile.all_totals.size_bytes).to_json(),
    );
    item.insert("external_dependency_loc".into(), profile.all_totals.loc.to_json());
    item.insert(
        "external_dependency_size_ratio_to_self_percent".into(),
        all_total_size_percent.to_json(),
    );
    item.insert(
        "external_dependency_loc_ratio_to_self_percent".into(),
        all_total_loc_percent.to_json(),
    );
    item.insert(
        "direct_non_edgerun_count".into(),
        profile.direct_totals.count.to_json(),
    );
    item.insert(
        "direct_non_edgerun_size_bytes".into(),
        profile.direct_totals.size_bytes.to_json(),
    );
    item.insert(
        "direct_non_edgerun_size_mb".into(),
        bytes_to_megabytes(profile.direct_totals.size_bytes).to_json(),
    );
    item.insert(
        "direct_non_edgerun_loc".into(),
        profile.direct_totals.loc.to_json(),
    );
    item.insert("dependencies".into(), JsonValue::array(all_dependencies));
    item.insert(
        "direct_dependencies".into(),
        JsonValue::array(direct_dependencies),
    );
    JsonValue::Object(item)
}

fn format_dependency_record_to_json(entry: DependencyRecord) -> JsonValue {
    let mut item = Map::new();
    item.insert("name".into(), entry.name.to_json());
    item.insert("dependency_id".into(), entry.id.to_json());
    item.insert("version".into(), entry.version.to_json());
    item.insert("source".into(), entry.source.to_json());
    item.insert("source_ref".into(), entry.source_ref.to_json());
    item.insert("manifest_path".into(), entry.manifest_path.to_json());
    item.insert("size_bytes".into(), entry.size_bytes.to_json());
    item.insert("size_mb".into(), entry.size_mb.to_json());
    item.insert("loc".into(), entry.loc.to_json());
    item.insert("is_direct".into(), entry.is_direct.to_json());
    item.insert(
        "size_ratio_to_self_percent".into(),
        entry.size_ratio_to_self_percent.to_json(),
    );
    item.insert(
        "loc_ratio_to_self_percent".into(),
        entry.loc_ratio_to_self_percent.to_json(),
    );
    JsonValue::Object(item)
}

fn load_cargo_metadata(workspace_root: &Path) -> Result<JsonValue, AnalyzerError> {
    let workspace_manifest = workspace_root.join("Cargo.toml");
    let output = Command::new("cargo")
        .args([
            "metadata",
            "--format-version",
            "1",
            "--manifest-path",
            workspace_manifest.to_str().unwrap_or("Cargo.toml"),
        ])
        .output()
        .map_err(|error| AnalyzerError::IoError(error.to_string()))?;

    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(AnalyzerError::ParseError {
            file: workspace_manifest,
            message: message.trim().to_string(),
        });
    }

    let output = String::from_utf8(output.stdout).map_err(|error| AnalyzerError::ParseError {
        file: workspace_manifest.clone(),
        message: error.to_string(),
    })?;

    edgerun_json::from_json_str::<JsonValue>(&output).map_err(|error| AnalyzerError::ParseError {
        file: workspace_manifest.clone(),
        message: error.to_string(),
    })
}

fn collect_packages(packages: &[JsonValue], workspace_root: &Path) -> HashMap<String, PackageInfo> {
    let mut package_by_id = HashMap::new();

    for package in packages {
        let Some(package) = package.as_object() else {
            continue;
        };

        let Some(id) = package.get("id").and_then(JsonValue::as_str) else {
            continue;
        };
        let name = package
            .get("name")
            .and_then(JsonValue::as_str)
            .unwrap_or_default()
            .to_string();
        if name.is_empty() {
            continue;
        }

        let version = package
            .get("version")
            .and_then(JsonValue::as_str)
            .unwrap_or_default()
            .to_string();
        let manifest_path = package
            .get("manifest_path")
            .and_then(JsonValue::as_str)
            .unwrap_or_default()
            .to_string();

        let source = package.get("source").and_then(JsonValue::as_str).map(str::to_string);
        let path_override = package.get("path").and_then(JsonValue::as_str).map(str::to_string);
        let crate_type = infer_crate_type(package);
        let is_workspace = package_is_workspace(&manifest_path, workspace_root);

        let direct_dependencies = package
            .get("dependencies")
            .and_then(JsonValue::as_array)
            .map(|deps| {
                deps.iter()
                    .filter_map(JsonValue::as_object)
                    .filter_map(|item| item.get("name").and_then(JsonValue::as_str))
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        package_by_id.insert(
            id.to_string(),
            PackageInfo {
                id: id.to_string(),
                name,
                version,
                manifest_path,
                source,
                path_override,
                crate_type,
                is_workspace,
                direct_dependencies,
            },
        );
    }

    package_by_id
}

fn package_is_workspace(manifest_path: &str, workspace_root: &Path) -> bool {
    if manifest_path.is_empty() {
        return false;
    }

    let manifest_path = Path::new(manifest_path);
    let Ok(relative) = manifest_path.strip_prefix(workspace_root) else {
        return false;
    };

    if let Some(first) = relative.components().next() {
        first.as_os_str() == "crates"
    } else {
        false
    }
}

fn parse_resolve_edges(metadata: &JsonValue) -> HashMap<String, Vec<String>> {
    let Some(resolve) = metadata.get("resolve").and_then(JsonValue::as_object) else {
        return HashMap::new();
    };
    let Some(nodes) = resolve.get("nodes").and_then(JsonValue::as_array) else {
        return HashMap::new();
    };

    let mut edges = HashMap::new();
    for node in nodes {
        let Some(node) = node.as_object() else {
            continue;
        };
        let Some(node_id) = node.get("id").and_then(JsonValue::as_str) else {
            continue;
        };
        let mut deps = Vec::new();

        if let Some(values) = node.get("dependencies").and_then(JsonValue::as_array) {
            deps.extend(values.iter().filter_map(JsonValue::as_str).map(str::to_string));
        } else if let Some(values) = node.get("deps").and_then(JsonValue::as_array) {
            for value in values {
                if let Some(value) = value.as_object().and_then(|obj| obj.get("pkg").and_then(JsonValue::as_str)) {
                    deps.push(value.to_string());
                }
            }
        }

        edges.insert(node_id.to_string(), deps);
    }

    edges
}

fn collect_dependency_closure(
    root_id: &str,
    edges: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut queue = VecDeque::new();

    if let Some(root_children) = edges.get(root_id) {
        for dep in root_children {
            queue.push_back(dep.clone());
        }
    }

    while let Some(current_dependency) = queue.pop_front() {
        if !seen.insert(current_dependency.clone()) {
            continue;
        }

        if let Some(children) = edges.get(&current_dependency) {
            for child in children {
                queue.push_back(child.clone());
            }
        }
    }

    seen.into_iter().collect()
}

fn collect_direct_dependency_ids(
    package: &PackageInfo,
    name_to_ids: &HashMap<String, Vec<String>>,
) -> HashSet<String> {
    let mut direct = HashSet::new();
    for dependency_name in &package.direct_dependencies {
        if let Some(ids) = name_to_ids.get(dependency_name) {
            for id in ids {
                direct.insert(id.clone());
            }
        }
    }
    direct
}

fn estimate_package_footprint(
    manifest_path: &str,
    cache: &mut HashMap<String, Footprint>,
) -> Footprint {
    let manifest_path = Path::new(manifest_path);
    let Some(root_dir) = manifest_path.parent() else {
        return Footprint {
            size_bytes: 0,
            loc: 0,
        };
    };

    let root_key = root_dir.to_string_lossy().to_string();
    if let Some(footprint) = cache.get(&root_key) {
        return *footprint;
    }

    let mut size_bytes = 0u64;
    let mut loc = 0u64;
    let mut queue = vec![root_dir.to_path_buf()];

    while let Some(current) = queue.pop() {
        let entries = match fs::read_dir(&current) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };

            if file_type.is_symlink() {
                continue;
            }

            if file_type.is_dir() {
                if should_skip_dir(path.file_name().and_then(|name| name.to_str())) {
                    continue;
                }
                queue.push(path);
                continue;
            }

            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    size_bytes = size_bytes.saturating_add(meta.len());
                }
            }

            if matches!(path.extension().and_then(|ext| ext.to_str()), Some("rs")) {
                if let Ok(contents) = fs::read_to_string(path) {
                    loc = loc.saturating_add(contents.lines().count() as u64);
                }
            }
        }
    }

    let footprint = Footprint { size_bytes, loc };
    cache.insert(root_key, footprint);
    footprint
}

fn should_skip_dir(name: Option<&str>) -> bool {
    matches!(
        name,
        Some(".git") | Some("target") | Some("node_modules") | Some(".idea") | Some(".vscode")
    ) || name.is_some_and(|value| value.starts_with('.'))
}

fn classify_dependency_source(package: &PackageInfo, workspace_root: &Path) -> String {
    if package.is_workspace {
        return "workspace".to_string();
    }
    if let Some(source) = package.source.as_deref() {
        if source.starts_with("registry+") {
            return "crates.io".to_string();
        }
        if source.starts_with("git+") {
            return "git".to_string();
        }
        return "other".to_string();
    }
    if package_is_workspace(&package.manifest_path, workspace_root) {
        return "workspace".to_string();
    }
    "path".to_string()
}

fn infer_crate_type(package: &edgerun_json::Map) -> String {
    let mut has_lib = false;
    let mut has_bin = false;

    if let Some(targets) = package.get("targets").and_then(JsonValue::as_array) {
        for target in targets {
            let Some(target) = target.as_object() else {
                continue;
            };
            let Some(kinds) = target.get("kind").and_then(JsonValue::as_array) else {
                continue;
            };
            for kind in kinds {
                if let Some(kind) = kind.as_str() {
                    match kind {
                        "lib" => has_lib = true,
                        "bin" => has_bin = true,
                        _ => {}
                    }
                }
            }
        }
    }

    match (has_lib, has_bin) {
        (true, true) => "library+binary".to_string(),
        (true, false) => "library".to_string(),
        (false, true) => "binary".to_string(),
        (false, false) => "unknown".to_string(),
    }
}

fn sum_dependency_totals(dependencies: &[DependencyRecord]) -> DependencyTotals {
    let mut totals = DependencyTotals {
        count: 0,
        size_bytes: 0,
        loc: 0,
    };

    for dependency in dependencies {
        totals.count += 1;
        totals.size_bytes = totals.size_bytes.saturating_add(dependency.size_bytes);
        totals.loc = totals.loc.saturating_add(dependency.loc);
    }

    totals
}

fn bytes_to_megabytes(bytes: u64) -> f64 {
    (bytes as f64) / 1024.0 / 1024.0
}

fn ratio_percent(part: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        (part as f64) / (total as f64) * 100.0
    }
}

fn empty_report(workspace_root: &Path) -> JsonValue {
    let mut top = Map::new();
    top.insert("workspace_root".into(), workspace_root.display().to_string().to_json());
    top.insert("generated_at".into(), Local::now().to_rfc3339().to_json());
    top.insert("summary".into(), JsonValue::Object(Map::new()));
    top.insert("crates".into(), JsonValue::empty_array());
    top.insert("distinct_external_dependencies".into(), JsonValue::empty_array());
    JsonValue::Object(top)
}
