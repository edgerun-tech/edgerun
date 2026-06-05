use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::SystemTime;

use crate::codealyzer::errors::AnalyzerError;

#[derive(Clone)]
struct CargoMetadata {
    packages: Vec<CargoPackage>,
    workspace_members: Vec<String>,
    resolve: Option<CargoResolve>,
}

#[derive(Clone)]
struct CargoPackage {
    id: String,
    name: String,
    version: String,
    manifest_path: String,
    source: Option<String>,
    path: Option<String>,
    dependencies: Option<Vec<CargoDependency>>,
    targets: Option<Vec<CargoPackageTarget>>,
}

#[derive(Clone)]
struct CargoDependency {
    name: String,
}

#[derive(Clone)]
struct CargoPackageTarget {
    kind: Vec<String>,
}

#[derive(Clone)]
struct CargoResolve {
    nodes: Vec<CargoResolveNode>,
}

#[derive(Clone)]
struct CargoResolveNode {
    id: String,
    dependencies: Option<Vec<String>>,
    deps: Option<Vec<CargoResolveDep>>,
}

#[derive(Clone)]
struct CargoResolveDep {
    pkg: String,
}

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
    source: String,
    size_bytes: u64,
    loc: u64,
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

#[derive(Debug, Clone)]
pub struct DependencyFootprintReport {
    pub workspace_root: String,
    pub generated_at: String,
    pub summary: DependencyFootprintSummary,
    pub crates: Vec<CrateDependencyProfileSummary>,
    pub distinct_external_dependencies: Vec<DependencySummary>,
}

#[derive(Debug, Clone)]
pub struct DependencyFootprintSummary {
    pub workspace_root_crate_count: usize,
    pub crates_with_external_dependencies: usize,
    pub distinct_external_dependency_count: usize,
    pub distinct_external_dependency_size_bytes: u64,
    pub distinct_external_dependency_size_mb: f64,
    pub distinct_external_dependency_loc: u64,
}

#[derive(Debug, Clone)]
pub struct CrateDependencyProfileSummary {
    pub name: String,
    pub path: String,
    pub crate_type: String,
    pub self_size_bytes: u64,
    pub self_size_mb: f64,
    pub self_loc: u64,
    pub external_dependency_count: usize,
    pub external_dependency_size_bytes: u64,
    pub external_dependency_size_mb: f64,
    pub external_dependency_loc: u64,
    pub external_dependency_size_ratio_to_self_percent: f64,
    pub external_dependency_loc_ratio_to_self_percent: f64,
    pub direct_non_edgerun_count: usize,
    pub direct_non_edgerun_size_bytes: u64,
    pub direct_non_edgerun_size_mb: f64,
    pub direct_non_edgerun_loc: u64,
}

#[derive(Debug, Clone)]
pub struct DependencySummary {
    pub name: String,
    pub dependency_id: String,
    pub version: String,
    pub source: String,
    pub source_ref: Option<String>,
    pub manifest_path: String,
    pub size_bytes: u64,
    pub size_mb: f64,
    pub loc: u64,
    pub dependent_crate_count: usize,
    pub direct_dependent_crate_count: usize,
    pub dependent_crates: Vec<String>,
    pub direct_dependent_crates: Vec<String>,
}

pub fn collect_dependency_footprints(
    workspace_root: &Path,
) -> Result<DependencyFootprintReport, AnalyzerError> {
    let workspace_root = workspace_root
        .canonicalize()
        .unwrap_or_else(|_| workspace_root.to_path_buf());

    let metadata = load_cargo_metadata(&workspace_root)?;
    let packages_root = &metadata.packages;
    if packages_root.is_empty() {
        return Ok(empty_report(&workspace_root));
    }

    let packages_by_id = collect_packages(packages_root, &workspace_root);
    if packages_by_id.is_empty() {
        return Ok(empty_report(&workspace_root));
    }

    let members = metadata.workspace_members.clone();

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

        let self_fp =
            estimate_package_footprint(&member_package.manifest_path, &mut package_footprint_cache);

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
                source,
                size_bytes: dependency_fp.size_bytes,
                loc: dependency_fp.loc,
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

            usage_entry
                .dependent_crates
                .insert(member_package.name.clone());
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
            all_totals,
            direct_totals,
        });
    }

    crate_profiles.sort_by(|a, b| a.name.cmp(&b.name));

    let mut crate_entries = Vec::new();
    for profile in crate_profiles {
        crate_entries.push(format_crate_profile(profile));
    }

    let mut distinct_dependencies = Vec::new();
    for usage in global_dependency_usage.values() {
        let dependent_crates = usage.dependent_crates.iter().cloned().collect::<Vec<_>>();
        let direct_dependent_crates = usage
            .direct_dependent_crates
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        distinct_dependencies.push(DependencySummary {
            dependency_id: usage.id.to_string(),
            name: usage.name.to_string(),
            version: usage.version.to_string(),
            source: usage.source.to_string(),
            source_ref: usage.source_ref.clone(),
            manifest_path: usage.manifest_path.to_string(),
            size_bytes: usage.size_bytes,
            size_mb: usage.size_mb,
            loc: usage.loc,
            dependent_crate_count: dependent_crates.len(),
            direct_dependent_crate_count: direct_dependent_crates.len(),
            dependent_crates,
            direct_dependent_crates,
        });
    }

    let distinct_dependency_count = distinct_dependencies.len();
    distinct_dependencies.sort_by(|lhs, rhs| rhs.size_bytes.cmp(&lhs.size_bytes));

    let mut global_size_bytes = 0u64;
    let mut global_loc = 0u64;
    for usage in &distinct_dependencies {
        global_size_bytes = global_size_bytes.saturating_add(usage.size_bytes);
        global_loc = global_loc.saturating_add(usage.loc);
    }

    let summary = DependencyFootprintSummary {
        workspace_root_crate_count: crate_entries.len(),
        crates_with_external_dependencies: crate_entries
            .iter()
            .filter(|entry| entry.external_dependency_count > 0)
            .count(),
        distinct_external_dependency_count: distinct_dependency_count,
        distinct_external_dependency_size_bytes: global_size_bytes,
        distinct_external_dependency_size_mb: bytes_to_megabytes(global_size_bytes),
        distinct_external_dependency_loc: global_loc,
    };

    Ok(DependencyFootprintReport {
        workspace_root: workspace_root.display().to_string(),
        generated_at: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_default(),
        summary,
        crates: crate_entries,
        distinct_external_dependencies: distinct_dependencies,
    })
}

pub fn write_dependency_footprints(
    report: &DependencyFootprintReport,
    output_dir: &Path,
) -> Result<(), AnalyzerError> {
    std::fs::create_dir_all(output_dir)
        .map_err(|error| AnalyzerError::IoError(error.to_string()))?;

    let output_path = output_dir.join("dependency-metrics.txt");
    let mut body = String::new();
    body.push_str("# dependency-metrics\n");
    body.push_str(&format!("workspace_root={}\n", report.workspace_root));
    body.push_str(&format!("generated_at={}\n", report.generated_at));
    body.push_str(&format!(
        "workspace_root_crate_count={}\n",
        report.summary.workspace_root_crate_count
    ));
    body.push_str(&format!(
        "crates_with_non_edgerun_external_dependencies={}\n",
        report.summary.crates_with_external_dependencies
    ));
    body.push_str(&format!(
        "distinct_external_dependency_count={}\n",
        report.summary.distinct_external_dependency_count
    ));
    body.push_str(&format!(
        "distinct_external_dependency_size_bytes={}\n",
        report.summary.distinct_external_dependency_size_bytes
    ));
    body.push_str(&format!(
        "distinct_external_dependency_size_mb={:.3}\n",
        report.summary.distinct_external_dependency_size_mb
    ));
    body.push_str(&format!(
        "distinct_external_dependency_loc={}\n",
        report.summary.distinct_external_dependency_loc
    ));
    body.push_str(&format!("crates_analyzed={}\n", report.crates.len()));
    body.push_str(&format!(
        "distinct_external_dependencies_in_report={}\n",
        report.distinct_external_dependencies.len()
    ));

    std::fs::write(output_path, body).map_err(|error| AnalyzerError::IoError(error.to_string()))
}

fn format_crate_profile(profile: CrateDependencyProfile) -> CrateDependencyProfileSummary {
    let all_total_size_percent =
        ratio_percent(profile.all_totals.size_bytes, profile.self_size_bytes);
    let all_total_loc_percent = ratio_percent(profile.all_totals.loc, profile.self_loc);

    CrateDependencyProfileSummary {
        name: profile.name,
        path: profile.path,
        crate_type: profile.crate_type,
        self_size_bytes: profile.self_size_bytes,
        self_size_mb: profile.self_size_mb,
        self_loc: profile.self_loc,
        external_dependency_count: profile.all_totals.count,
        external_dependency_size_bytes: profile.all_totals.size_bytes,
        external_dependency_size_mb: bytes_to_megabytes(profile.all_totals.size_bytes),
        external_dependency_loc: profile.all_totals.loc,
        external_dependency_size_ratio_to_self_percent: all_total_size_percent,
        external_dependency_loc_ratio_to_self_percent: all_total_loc_percent,
        direct_non_edgerun_count: profile.direct_totals.count,
        direct_non_edgerun_size_bytes: profile.direct_totals.size_bytes,
        direct_non_edgerun_size_mb: bytes_to_megabytes(profile.direct_totals.size_bytes),
        direct_non_edgerun_loc: profile.direct_totals.loc,
    }
}

fn load_cargo_metadata(workspace_root: &Path) -> Result<CargoMetadata, AnalyzerError> {
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

    parse_cargo_metadata_projection(&output).map_err(|message| AnalyzerError::ParseError {
        file: workspace_manifest,
        message,
    })
}

fn parse_cargo_metadata_projection(input: &str) -> Result<CargoMetadata, String> {
    let package_objects = fixed_json_object_array_field(input, "packages")
        .ok_or("cargo metadata missing packages")?;
    let mut packages = Vec::new();
    for package in package_objects {
        packages.push(project_cargo_package(package)?);
    }

    let workspace_members = fixed_json_string_array_field(input, "workspace_members")
        .ok_or("cargo metadata missing workspace_members")?;
    let resolve = fixed_json_object_field(input, "resolve").and_then(project_cargo_resolve);

    Ok(CargoMetadata {
        packages,
        workspace_members,
        resolve,
    })
}

fn project_cargo_package(input: &str) -> Result<CargoPackage, String> {
    let id = fixed_json_string_field(input, "id").ok_or("cargo package missing id")?;
    let name = fixed_json_string_field(input, "name").ok_or("cargo package missing name")?;
    let version =
        fixed_json_string_field(input, "version").ok_or("cargo package missing version")?;
    let manifest_path = fixed_json_string_field(input, "manifest_path")
        .ok_or("cargo package missing manifest_path")?;
    let source = fixed_json_optional_string_field(input, "source");
    let path = fixed_json_optional_string_field(input, "path");
    let dependencies = fixed_json_object_array_field(input, "dependencies").map(|objects| {
        objects
            .into_iter()
            .filter_map(|object| fixed_json_string_field(object, "name"))
            .map(|name| CargoDependency { name })
            .collect::<Vec<_>>()
    });
    let targets = fixed_json_object_array_field(input, "targets").map(|objects| {
        objects
            .into_iter()
            .filter_map(|object| {
                fixed_json_string_array_field(object, "kind")
                    .map(|kind| CargoPackageTarget { kind })
            })
            .collect::<Vec<_>>()
    });

    Ok(CargoPackage {
        id,
        name,
        version,
        manifest_path,
        source,
        path,
        dependencies,
        targets,
    })
}

fn project_cargo_resolve(input: &str) -> Option<CargoResolve> {
    let nodes = fixed_json_object_array_field(input, "nodes")?
        .into_iter()
        .filter_map(project_cargo_resolve_node)
        .collect::<Vec<_>>();
    Some(CargoResolve { nodes })
}

fn project_cargo_resolve_node(input: &str) -> Option<CargoResolveNode> {
    let id = fixed_json_string_field(input, "id")?;
    let dependencies = fixed_json_string_array_field(input, "dependencies");
    let deps = fixed_json_object_array_field(input, "deps").map(|objects| {
        objects
            .into_iter()
            .filter_map(|object| fixed_json_string_field(object, "pkg"))
            .map(|pkg| CargoResolveDep { pkg })
            .collect::<Vec<_>>()
    });
    Some(CargoResolveNode {
        id,
        dependencies,
        deps,
    })
}

fn fixed_json_optional_string_field(input: &str, key: &str) -> Option<String> {
    let value = fixed_json_field_value(input, key)?.trim_start();
    if value.starts_with("null") {
        None
    } else {
        parse_json_string(value).map(|(value, _)| value)
    }
}

fn fixed_json_string_field(input: &str, key: &str) -> Option<String> {
    let value = fixed_json_field_value(input, key)?;
    parse_json_string(value.trim_start()).map(|(value, _)| value)
}

fn fixed_json_string_array_field(input: &str, key: &str) -> Option<Vec<String>> {
    let value = fixed_json_field_value(input, key)?.trim_start();
    if !value.starts_with('[') {
        return None;
    }
    let end = matching_json_end(value, '[', ']')?;
    let mut rest = value[1..end - 1].trim_start();
    let mut items = Vec::new();
    while !rest.is_empty() {
        let (item, used) = parse_json_string(rest)?;
        items.push(item);
        rest = rest[used..].trim_start();
        if rest.starts_with(',') {
            rest = rest[1..].trim_start();
        } else if !rest.is_empty() {
            return None;
        }
    }
    Some(items)
}

fn fixed_json_object_field<'a>(input: &'a str, key: &str) -> Option<&'a str> {
    let value = fixed_json_field_value(input, key)?.trim_start();
    if !value.starts_with('{') {
        return None;
    }
    let end = matching_json_end(value, '{', '}')?;
    Some(&value[..end])
}

fn fixed_json_object_array_field<'a>(input: &'a str, key: &str) -> Option<Vec<&'a str>> {
    let value = fixed_json_field_value(input, key)?.trim_start();
    if !value.starts_with('[') {
        return None;
    }
    let end = matching_json_end(value, '[', ']')?;
    let mut rest = value[1..end - 1].trim_start();
    let mut objects = Vec::new();
    while !rest.is_empty() {
        if !rest.starts_with('{') {
            return None;
        }
        let object_end = matching_json_end(rest, '{', '}')?;
        objects.push(&rest[..object_end]);
        rest = rest[object_end..].trim_start();
        if rest.starts_with(',') {
            rest = rest[1..].trim_start();
        } else if !rest.is_empty() {
            return None;
        }
    }
    Some(objects)
}

fn fixed_json_field_value<'a>(input: &'a str, key: &str) -> Option<&'a str> {
    let mut rest = input;
    while let Some(pos) = rest.find('"') {
        rest = &rest[pos..];
        let (found, used) = parse_json_string(rest)?;
        rest = &rest[used..];
        let after_key = rest.trim_start();
        if !after_key.starts_with(':') {
            continue;
        }
        let value = after_key[1..].trim_start();
        if found == key {
            return Some(value);
        }
        rest = value;
    }
    None
}

fn parse_json_string(input: &str) -> Option<(String, usize)> {
    let bytes = input.as_bytes();
    if bytes.first().copied() != Some(b'"') {
        return None;
    }
    let mut out = String::new();
    let mut i = 1usize;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => return Some((out, i + 1)),
            b'\\' => {
                i += 1;
                let escaped = *bytes.get(i)?;
                match escaped {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'b' => out.push('\u{0008}'),
                    b'f' => out.push('\u{000c}'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        let hex = input.get(i + 1..i + 5)?;
                        let code = u16::from_str_radix(hex, 16).ok()? as u32;
                        out.push(char::from_u32(code)?);
                        i += 4;
                    }
                    _ => return None,
                }
            }
            byte if byte < 0x20 => return None,
            byte => out.push(byte as char),
        }
        i += 1;
    }
    None
}

fn matching_json_end(input: &str, open: char, close: char) -> Option<usize> {
    let bytes = input.as_bytes();
    if !input.starts_with(open) {
        return None;
    }
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escape = false;
    for (index, byte) in bytes.iter().enumerate() {
        if in_string {
            if escape {
                escape = false;
            } else if *byte == b'\\' {
                escape = true;
            } else if *byte == b'"' {
                in_string = false;
            }
            continue;
        }
        if *byte == b'"' {
            in_string = true;
            continue;
        }
        let ch = *byte as char;
        if ch == open {
            depth += 1;
        } else if ch == close {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index + 1);
            }
        }
    }
    None
}

fn collect_packages(
    packages: &[CargoPackage],
    workspace_root: &Path,
) -> HashMap<String, PackageInfo> {
    let mut package_by_id = HashMap::new();

    for package in packages {
        let id = package.id.clone();
        let name = package.name.clone();
        if name.is_empty() {
            continue;
        }
        let version = package.version.clone();
        let manifest_path = package.manifest_path.clone();
        let source = package.source.clone();
        let path_override = package.path.clone();
        let crate_type = infer_crate_type(package);
        let is_workspace = package_is_workspace(&manifest_path, workspace_root);

        let direct_dependencies = package
            .dependencies
            .as_ref()
            .and_then(|deps| (!deps.is_empty()).then_some(deps))
            .map(|deps| {
                deps.iter()
                    .map(|item| item.name.clone())
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

fn parse_resolve_edges(metadata: &CargoMetadata) -> HashMap<String, Vec<String>> {
    let Some(resolve) = metadata.resolve.as_ref() else {
        return HashMap::new();
    };
    if resolve.nodes.is_empty() {
        return HashMap::new();
    }

    let mut edges = HashMap::new();
    for node in &resolve.nodes {
        let node_id = &node.id;
        let mut deps = Vec::new();

        if let Some(values) = &node.dependencies {
            deps.extend(values.iter().cloned());
        } else if let Some(values) = &node.deps {
            deps.extend(values.iter().map(|value| value.pkg.clone()));
        }

        edges.insert(node_id.clone(), deps);
    }

    edges
}

fn collect_dependency_closure(root_id: &str, edges: &HashMap<String, Vec<String>>) -> Vec<String> {
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

fn infer_crate_type(package: &CargoPackage) -> String {
    let mut has_lib = false;
    let mut has_bin = false;

    if let Some(targets) = package.targets.as_ref() {
        for target in targets {
            for kind in &target.kind {
                match kind.as_str() {
                    "lib" => has_lib = true,
                    "bin" => has_bin = true,
                    _ => {}
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

fn empty_report(workspace_root: &Path) -> DependencyFootprintReport {
    DependencyFootprintReport {
        workspace_root: workspace_root.display().to_string(),
        generated_at: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_default(),
        summary: DependencyFootprintSummary {
            workspace_root_crate_count: 0,
            crates_with_external_dependencies: 0,
            distinct_external_dependency_count: 0,
            distinct_external_dependency_size_bytes: 0,
            distinct_external_dependency_size_mb: 0.0,
            distinct_external_dependency_loc: 0,
        },
        crates: Vec::new(),
        distinct_external_dependencies: Vec::new(),
    }
}
