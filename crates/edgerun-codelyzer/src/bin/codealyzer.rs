use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{self, Command};

use edgerun_codelyzer::codealyzer::crate_model::{CrateReport, CrateType};
use edgerun_codelyzer::codealyzer::dependency_footprint::DependencyFootprintReport;
use edgerun_codelyzer::codealyzer::{
    collect_dependency_footprints, generate_report, load_runtime_events, save_report,
    write_dependency_footprints,
};

#[derive(Default)]
struct Totals {
    crates_attempted: usize,
    crates_success: usize,
    crates_failed: usize,
    total_visible_files: usize,
    total_hidden_files: usize,
    total_dependencies: usize,
    total_external_dependencies: usize,
    total_dependency_weight: usize,
    total_public_api: usize,
    total_call_edges: usize,
    total_runtime_call_observations: u64,
    total_runtime_call_edges: usize,
    total_security_findings: usize,
    total_test_files: usize,
    total_unit_tests: usize,
    total_integration_tests: usize,
    total_doc_tests: usize,
    crates_compiled: usize,
    crates_compile_failed: usize,
    total_compile_warnings: usize,
    total_public_items: usize,
    total_covered_public_items: usize,
    library_crates: usize,
    binary_crates: usize,
    library_binary_crates: usize,
    unknown_crates: usize,
}

#[derive(Debug, Default)]
struct TargetDirectoryReport {
    path: PathBuf,
    size_bytes: u64,
    removed: bool,
    error: Option<String>,
}

fn print_usage(program: &str) {
    println!(
        "Usage: {program} [--workspace-root <path>] [--out <path>] [--runtime-events <path>] [--inventory] [--dependency-metrics] [--clean-targets]"
    );
    println!();
    println!("Run the crate analyzer across crates/ and aggregate totals.");
    println!("Defaults: workspace root = current Cargo workspace, output dir = target/codealyzer");
    println!("  --inventory  print a per-crate contents summary");
    println!("  --dependency-metrics");
    println!("               collect non-edgerun transitive dependency size/LOC metrics");
    println!("  --runtime-events <path>");
    println!("               merge rkyv runtime call traces into call graph");
    println!("  --clean-targets");
    println!("               remove all target directories under workspace root and exit");
}

fn parse_args() -> (PathBuf, PathBuf, bool, bool, bool, Option<PathBuf>) {
    let mut workspace_root: Option<PathBuf> = None;
    let mut output_dir: Option<PathBuf> = None;
    let mut runtime_events_path: Option<PathBuf> = None;
    let mut show_inventory = false;
    let mut collect_metrics = false;
    let mut clean_targets = false;
    let mut show_help = false;

    let mut iter = std::env::args().skip(1).peekable();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                show_help = true;
                break;
            }
            "--inventory" => {
                show_inventory = true;
            }
            "--dependency-metrics" => {
                collect_metrics = true;
            }
            "--clean-targets" => {
                clean_targets = true;
            }
            "--workspace-root" => {
                let path = iter.next().unwrap_or_default();
                workspace_root = Some(PathBuf::from(path));
            }
            "--out" => {
                let path = iter.next().unwrap_or_default();
                output_dir = Some(PathBuf::from(path));
            }
            "--runtime-events" => {
                let path = iter.next().unwrap_or_default();
                runtime_events_path = Some(PathBuf::from(path));
            }
            unknown => {
                if !unknown.starts_with('-') {
                    workspace_root.get_or_insert_with(|| PathBuf::from(unknown));
                }
            }
        }
    }

    if show_help {
        print_usage(
            &std::env::args()
                .next()
                .unwrap_or_else(|| "codealyzer".into()),
        );
        process::exit(0);
    }

    (
        workspace_root.unwrap_or_else(edgerun_codelyzer::codealyzer::workspace::get_workspace_root),
        output_dir.unwrap_or_else(|| PathBuf::from("target/codealyzer")),
        show_inventory,
        collect_metrics,
        clean_targets,
        runtime_events_path,
    )
}

fn is_hidden_or_excluded_dir(name: &str) -> bool {
    matches!(
        name,
        ".git" | ".github" | ".idea" | ".next" | ".turbo" | ".vscode" | ".yarn" | ".cache"
    )
}

fn collect_target_dirs(root: &Path) -> Vec<PathBuf> {
    let mut targets = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(current_dir) = stack.pop() {
        let entries = match std::fs::read_dir(&current_dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.filter_map(|entry| entry.ok()) {
            let entry_path = entry.path();
            let file_name = match entry_path.file_name().and_then(|s| s.to_str()) {
                Some(name) => name,
                None => continue,
            };

            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_dir() {
                continue;
            }
            if file_type.is_symlink() {
                continue;
            }
            if file_name == "target" {
                targets.push(entry_path);
                continue;
            }
            if is_hidden_or_excluded_dir(file_name) {
                continue;
            }
            stack.push(entry_path);
        }
    }

    targets.sort_unstable();
    targets
}

fn directory_size_bytes(path: &Path) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];

    while let Some(current) = stack.pop() {
        let entries = match std::fs::read_dir(&current) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.filter_map(|entry| entry.ok()) {
            let entry_path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                stack.push(entry_path);
                continue;
            }
            if let Ok(metadata) = entry.metadata() {
                total = total.saturating_add(metadata.len());
            }
        }
    }

    total
}

fn clean_target_dirs(workspace_root: &Path) -> Vec<TargetDirectoryReport> {
    let mut reports = Vec::new();
    let targets = collect_target_dirs(workspace_root);

    for target in targets {
        if !target.exists() {
            continue;
        }

        let size_bytes = directory_size_bytes(&target);
        match std::fs::remove_dir_all(&target) {
            Ok(_) => reports.push(TargetDirectoryReport {
                path: target,
                size_bytes,
                removed: true,
                error: None,
            }),
            Err(err) => reports.push(TargetDirectoryReport {
                path: target,
                size_bytes,
                removed: false,
                error: Some(err.to_string()),
            }),
        }
    }

    reports
}

fn format_byte_size(bytes: u64) -> String {
    let mut value = bytes as f64;
    let units = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut unit = 0usize;

    while value >= 1024.0 && unit + 1 < units.len() {
        value /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{value:.0} {}", units[unit])
    } else {
        format!("{value:.2} {}", units[unit])
    }
}

fn print_target_cleanup_summary(reports: &[TargetDirectoryReport]) {
    println!("Target cleanup");

    if reports.is_empty() {
        println!("  no target directories found");
        return;
    }

    let removed = reports.iter().filter(|r| r.removed).count();
    let failed = reports.len().saturating_sub(removed);
    let total_size_bytes: u64 = reports.iter().map(|r| r.size_bytes).sum();
    let removed_size_bytes: u64 = reports
        .iter()
        .filter(|r| r.removed)
        .map(|r| r.size_bytes)
        .sum();

    println!("  found: {}", reports.len());
    println!("  removed: {removed}");
    println!("  failed: {failed}");
    println!(
        "  total_target_size: {} ({} bytes)",
        format_byte_size(total_size_bytes),
        total_size_bytes
    );
    println!(
        "  removed_size: {} ({} bytes)",
        format_byte_size(removed_size_bytes),
        removed_size_bytes
    );

    for report in reports {
        if report.removed {
            println!(
                "  removed: {} ({})",
                report.path.display(),
                format_byte_size(report.size_bytes)
            );
        } else if let Some(err) = report.error.as_deref() {
            println!("  failed:  {} ({})", report.path.display(), err);
        }
    }
}

fn list_crate_dirs(workspace_root: &Path) -> Vec<PathBuf> {
    if let Some(dirs) = list_workspace_package_dirs(workspace_root) {
        return dirs;
    }

    let crates_root = workspace_root.join("crates");
    let mut dirs = Vec::new();
    let mut stack = vec![crates_root];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(v) => v,
            Err(_) => continue,
        };

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|v| v.to_str()) else {
                continue;
            };
            if is_hidden_or_excluded_dir(name) || name == "target" {
                continue;
            }
            let manifest = path.join("Cargo.toml");
            if manifest.exists() && is_package_manifest(&manifest) {
                dirs.push(path);
                continue;
            }
            stack.push(path);
        }
    }

    dirs.sort_by_key(|p| {
        p.strip_prefix(workspace_root)
            .unwrap_or(p)
            .to_string_lossy()
            .into_owned()
    });
    dirs
}

fn list_workspace_package_dirs(workspace_root: &Path) -> Option<Vec<PathBuf>> {
    let manifest_path = workspace_root.join("Cargo.toml");
    let output = Command::new("cargo")
        .args([
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
            manifest_path.to_str()?,
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    let workspace_members = metadata
        .get("workspace_members")?
        .as_array()?
        .iter()
        .filter_map(|member| member.as_str())
        .collect::<BTreeSet<_>>();

    let mut dirs = Vec::new();
    for package in metadata.get("packages")?.as_array()? {
        let Some(id) = package.get("id").and_then(|id| id.as_str()) else {
            continue;
        };
        if !workspace_members.contains(id) {
            continue;
        }
        let Some(manifest) = package.get("manifest_path").and_then(|path| path.as_str()) else {
            continue;
        };
        let Some(dir) = Path::new(manifest).parent() else {
            continue;
        };
        dirs.push(dir.to_path_buf());
    }

    dirs.sort_by_key(|p| {
        p.strip_prefix(workspace_root)
            .unwrap_or(p)
            .to_string_lossy()
            .into_owned()
    });
    dirs.dedup();
    Some(dirs)
}

fn is_package_manifest(manifest: &Path) -> bool {
    let Ok(content) = std::fs::read_to_string(manifest) else {
        return false;
    };
    edgerun_json::from_toml_str(&content)
        .ok()
        .and_then(|toml| toml.get("package").map(|_| ()))
        .is_some()
}

fn update_totals(totals: &mut Totals, report: &CrateReport) {
    totals.crates_success += 1;
    totals.total_visible_files += report.identity.visible_files.len();
    totals.total_hidden_files += report.identity.hidden_files_count;
    totals.total_dependencies += report.dependencies.len();
    totals.total_external_dependencies += report
        .dependencies
        .iter()
        .filter(|dep| !dep.is_workspace)
        .count();
    totals.total_dependency_weight += report
        .dependencies
        .iter()
        .map(|dep| dep.weight)
        .sum::<usize>();
    totals.total_public_api += report.public_api.len();
    totals.total_call_edges += report.call_graph.len();
    totals.total_runtime_call_observations = totals
        .total_runtime_call_observations
        .saturating_add(report.runtime_call_observations);
    totals.total_runtime_call_edges += report
        .call_graph
        .iter()
        .filter(|edge| edge.runtime_count > 0)
        .count();
    totals.total_security_findings += report.security_findings.len();
    totals.total_test_files += report.test_info.total;
    totals.total_unit_tests += report.test_info.unit_tests;
    totals.total_integration_tests += report.test_info.integration_tests;
    totals.total_doc_tests += report.test_info.doc_tests;
    totals.total_compile_warnings += report.test_info.compile_warnings;
    totals.total_public_items += report.test_info.functionality_coverage.public_items;
    totals.total_covered_public_items += report.test_info.functionality_coverage.covered_items;

    match report.identity.crate_type {
        CrateType::Library => totals.library_crates += 1,
        CrateType::Binary => totals.binary_crates += 1,
        CrateType::LibraryAndBinary => totals.library_binary_crates += 1,
        CrateType::Unknown => totals.unknown_crates += 1,
    }

    if report.test_info.compile_status.as_deref() == Some("compiled") {
        totals.crates_compiled += 1;
    } else {
        totals.crates_compile_failed += 1;
    }
}

#[derive(Default)]
struct CrateInventory {
    top_dirs: Vec<String>,
    top_files: Vec<String>,
    has_src_dir: bool,
    has_examples_dir: bool,
    has_tests_dir: bool,
    has_benches_dir: bool,
    has_build_rs: bool,
    visible_rs_files: usize,
    hidden_files: usize,
    extension_counts: Vec<(String, usize)>,
}

fn gather_crate_inventory(
    crate_dir: &Path,
    visible_files: &[PathBuf],
    hidden_files: usize,
) -> CrateInventory {
    let mut dirs = BTreeSet::new();
    let mut files = BTreeSet::new();
    let mut extensions: BTreeMap<String, usize> = BTreeMap::new();
    let mut visible_rs_files = 0usize;

    for file in visible_files {
        if let Some(ext) = file.extension().and_then(|e| e.to_str()) {
            let key = ext.to_ascii_lowercase();
            *extensions.entry(key.clone()).or_insert(0) += 1;
            if key == "rs" {
                visible_rs_files += 1;
            }
        } else {
            *extensions.entry("<no_ext>".to_string()).or_insert(0) += 1;
        }

        if let Ok(relative) = file.strip_prefix(crate_dir) {
            let mut components = relative.components();
            if let Some(component) = components.next() {
                dirs.insert(component.as_os_str().to_string_lossy().into_owned());
            }
            if let Some(file_name) = file.file_name().and_then(|v| v.to_str()) {
                files.insert(file_name.to_string());
            }
        }
    }

    let has_dir = |name: &str| crate_dir.join(name).is_dir();

    CrateInventory {
        top_dirs: dirs.into_iter().collect(),
        top_files: files.into_iter().collect(),
        has_src_dir: has_dir("src"),
        has_examples_dir: has_dir("examples"),
        has_tests_dir: has_dir("tests"),
        has_benches_dir: has_dir("benches"),
        has_build_rs: crate_dir.join("build.rs").exists(),
        visible_rs_files,
        hidden_files,
        extension_counts: extensions.into_iter().collect(),
    }
}

fn print_crate_inventory(record: &CrateInventory, report: &CrateReport) {
    let deps_internal = report
        .dependencies
        .iter()
        .filter(|dep| dep.is_workspace)
        .count();
    let deps_external = report
        .dependencies
        .iter()
        .filter(|dep| !dep.is_workspace)
        .count();
    let extension_summary = record
        .extension_counts
        .iter()
        .map(|(ext, count)| format!("{ext}={count}"))
        .collect::<Vec<_>>()
        .join(", ");

    println!("  path: {}", report.identity.path.display());
    println!(
        "  contents dirs: {}",
        if record.top_dirs.is_empty() {
            "none".to_owned()
        } else {
            record.top_dirs.join(", ")
        }
    );
    if !record.top_files.is_empty() {
        println!("  top-level files: {}", record.top_files.join(", "));
    }
    println!(
        "  structure: has_src_dir={}, has_tests_dir={}, has_examples_dir={}, has_benches_dir={}, build.rs={}",
        record.has_src_dir,
        record.has_tests_dir,
        record.has_examples_dir,
        record.has_benches_dir,
        record.has_build_rs
    );
    println!(
        "  type: {}, visibility=public subset={}, compile={} (code={}), coverage={}/{} ({}%)",
        report.identity.crate_type.as_str(),
        if record.hidden_files > 0 {
            "true"
        } else {
            "false"
        },
        report
            .test_info
            .compile_status
            .as_deref()
            .unwrap_or("unknown"),
        report
            .test_info
            .compile_exit_code
            .map(|v| v.to_string())
            .unwrap_or_else(|| "n/a".to_string()),
        report.test_info.functionality_coverage.covered_items,
        report.test_info.functionality_coverage.public_items,
        report.test_info.functionality_coverage.coverage_percent,
    );
    println!(
        "  files: visible={}, hidden={}, visible_rs={}, ext [{}]",
        report.identity.visible_files.len(),
        record.hidden_files,
        record.visible_rs_files,
        extension_summary
    );
    println!(
        "  deps: total={}, internal={}, external={}, weight={}",
        report.dependencies.len(),
        deps_internal,
        deps_external,
        report
            .dependencies
            .iter()
            .map(|dep| dep.weight)
            .sum::<usize>()
    );
    println!(
        "  analysis: api={}, edges={}, runtime_calls={}, findings={}, warnings={}, tests={} ({}), compiled={}",
        report.public_api.len(),
        report.call_graph.len(),
        report.runtime_call_observations,
        report.security_findings.len(),
        report.test_info.compile_warnings,
        report.test_info.total,
        report.test_info.compile_status.as_deref().unwrap_or("unknown"),
        report
            .test_info
            .compile_status
            .as_deref()
            .is_some_and(|status| status == "compiled")
    );
}

fn print_dependency_summary(report: &DependencyFootprintReport) {
    println!("\nDependency metrics");
    println!(
        "  workspace crates: {}",
        report.summary.workspace_root_crate_count
    );
    println!(
        "  crates with non-edgerun external dependencies: {}",
        report.summary.crates_with_external_dependencies
    );
    println!(
        "  distinct non-edgerun dependencies: {}",
        report.summary.distinct_external_dependency_count
    );
    println!(
        "  dependency disk footprint: {} bytes ({:.2} MB)",
        report.summary.distinct_external_dependency_size_bytes,
        report.summary.distinct_external_dependency_size_mb
    );
    println!(
        "  dependency LOC: {}",
        report.summary.distinct_external_dependency_loc
    );
}

fn percentage_covered(covered: usize, total: usize) -> usize {
    if total == 0 {
        0
    } else {
        covered.saturating_mul(100) / total
    }
}

fn print_totals_text(totals: &Totals, out_dir: &Path) {
    println!("\nTotals");
    println!("  crates_attempted:      {}", totals.crates_attempted);
    println!("  crates_ok:             {}", totals.crates_success);
    println!("  crates_failed:         {}", totals.crates_failed);
    println!("  crates_compiled:       {}", totals.crates_compiled);
    println!("  crates_compile_failed: {}", totals.crates_compile_failed);
    println!("  library_crates:        {}", totals.library_crates);
    println!("  binary_crates:         {}", totals.binary_crates);
    println!("  library+binary:        {}", totals.library_binary_crates);
    println!("  unknown_type:          {}", totals.unknown_crates);
    println!("  visible_files:         {}", totals.total_visible_files);
    println!("  hidden_files:          {}", totals.total_hidden_files);
    println!(
        "  dependencies:          {} ({} external, {} total weight)",
        totals.total_dependencies,
        totals.total_external_dependencies,
        totals.total_dependency_weight
    );
    println!("  public_api_items:      {}", totals.total_public_api);
    println!("  call_graph_edges:      {}", totals.total_call_edges);
    println!(
        "  runtime_call_observations: {}",
        totals.total_runtime_call_observations
    );
    println!(
        "  runtime_call_edges:    {}",
        totals.total_runtime_call_edges
    );
    println!(
        "  security_findings:     {}",
        totals.total_security_findings
    );
    println!("  compile_warnings:      {}", totals.total_compile_warnings);
    println!(
        "  functionality_coverage: {}/{} ({}%)",
        totals.total_covered_public_items,
        totals.total_public_items,
        percentage_covered(totals.total_covered_public_items, totals.total_public_items)
    );
    println!(
        "  tests:                 {}", // alias for compatibility
        totals.total_test_files
    );
    println!("  unit_tests:            {}", totals.total_unit_tests);
    println!(
        "  integration_tests:     {}",
        totals.total_integration_tests
    );
    println!("  doc_tests:             {}", totals.total_doc_tests);
    println!("  output:                {}", out_dir.display());
}

fn write_workspace_summary(out_dir: &Path, totals: &Totals) -> Result<(), String> {
    std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let summary_path = out_dir.join("workspace-summary.txt");
    let mut summary = String::new();
    summary.push_str("Totals\n");
    summary.push_str(&format!("crates_attempted {}\n", totals.crates_attempted));
    summary.push_str(&format!("crates_ok {}\n", totals.crates_success));
    summary.push_str(&format!("crates_failed {}\n", totals.crates_failed));
    summary.push_str(&format!("crates_compiled {}\n", totals.crates_compiled));
    summary.push_str(&format!(
        "crates_compile_failed {}\n",
        totals.crates_compile_failed
    ));
    summary.push_str(&format!("library_crates {}\n", totals.library_crates));
    summary.push_str(&format!("binary_crates {}\n", totals.binary_crates));
    summary.push_str(&format!(
        "library_binary_crates {}\n",
        totals.library_binary_crates
    ));
    summary.push_str(&format!("unknown_type {}\n", totals.unknown_crates));
    summary.push_str(&format!("visible_files {}\n", totals.total_visible_files));
    summary.push_str(&format!("hidden_files {}\n", totals.total_hidden_files));
    summary.push_str(&format!(
        "dependencies {} {} {}\n",
        totals.total_dependencies,
        totals.total_external_dependencies,
        totals.total_dependency_weight
    ));
    summary.push_str(&format!("public_api_items {}\n", totals.total_public_api));
    summary.push_str(&format!("call_graph_edges {}\n", totals.total_call_edges));
    summary.push_str(&format!(
        "runtime_call_observations {}\n",
        totals.total_runtime_call_observations
    ));
    summary.push_str(&format!(
        "runtime_call_edges {}\n",
        totals.total_runtime_call_edges
    ));
    summary.push_str(&format!(
        "security_findings {}\n",
        totals.total_security_findings
    ));
    summary.push_str(&format!(
        "compile_warnings {}\n",
        totals.total_compile_warnings
    ));
    summary.push_str(&format!(
        "functionality_coverage {}/{} {}%\n",
        totals.total_covered_public_items,
        totals.total_public_items,
        percentage_covered(totals.total_covered_public_items, totals.total_public_items)
    ));
    summary.push_str(&format!("tests {}\n", totals.total_test_files));
    summary.push_str(&format!("unit_tests {}\n", totals.total_unit_tests));
    summary.push_str(&format!(
        "integration_tests {}\n",
        totals.total_integration_tests
    ));
    summary.push_str(&format!("doc_tests {}\n", totals.total_doc_tests));
    summary.push_str(&format!("output {}\n", out_dir.display()));
    std::fs::write(&summary_path, summary).map_err(|e| e.to_string())?;
    Ok(())
}

fn main() {
    let (
        workspace_root,
        output_dir,
        show_inventory,
        collect_metrics,
        clean_targets,
        runtime_events_path,
    ) = parse_args();
    if !workspace_root.join("Cargo.toml").exists() {
        eprintln!("Workspace root not found: {workspace_root:?}");
        process::exit(1);
    }

    if clean_targets {
        let targets = clean_target_dirs(&workspace_root);
        print_target_cleanup_summary(&targets);
        process::exit(0);
    }

    let crates = list_crate_dirs(&workspace_root);
    if crates.is_empty() {
        eprintln!(
            "No crate directories found under: {}",
            workspace_root.join("crates").display()
        );
        process::exit(1);
    }

    println!(
        "Analyzing {} crate directories in {}",
        crates.len(),
        workspace_root.display()
    );

    let mut totals = Totals::default();
    let mut failures: Vec<(String, String)> = Vec::new();
    let mut dependency_report: Option<DependencyFootprintReport> = None;
    let runtime_events = if let Some(runtime_events_path) = runtime_events_path {
        let runtime_events = match load_runtime_events(&runtime_events_path) {
            Ok(runtime_events) => runtime_events,
            Err(err) => {
                eprintln!(
                    "Failed to load runtime events from {}: {err}",
                    runtime_events_path.display()
                );
                process::exit(1);
            }
        };
        println!(
            "Loaded {} runtime observations from {}",
            runtime_events.len(),
            runtime_events_path.display()
        );
        Some(runtime_events)
    } else {
        None
    };

    if collect_metrics {
        println!("Collecting dependency footprint metrics...");
        match collect_dependency_footprints(&workspace_root) {
            Ok(report) => {
                if let Err(err) = write_dependency_footprints(&report, &output_dir) {
                    eprintln!("Failed to write dependency metrics: {err}");
                    process::exit(1);
                }
                dependency_report = Some(report);
            }
            Err(err) => {
                eprintln!("Dependency metrics collection failed: {err}");
                process::exit(1);
            }
        }
    }

    for crate_dir in crates {
        let crate_name = crate_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        totals.crates_attempted += 1;
        match generate_report(
            &crate_name,
            &crate_dir,
            &workspace_root,
            runtime_events.as_deref(),
        ) {
            Ok(report) => {
                if let Err(err) = save_report(&report, &output_dir) {
                    totals.crates_failed += 1;
                    failures.push((crate_name, format!("save failed: {err}")));
                } else {
                    update_totals(&mut totals, &report);
                    let inventory = gather_crate_inventory(
                        &crate_dir,
                        &report.identity.visible_files,
                        report.identity.hidden_files_count,
                    );

                    println!(
                        "  ok  {name}: deps={deps}, api={api}, edges={edges}, runtime_calls={runtime_calls}, findings={findings}, warnings={warnings}, coverage={covered}/{total} ({pct}%)",
                        name = report.identity.name,
                        deps = report.dependencies.len(),
                        api = report.public_api.len(),
                        edges = report.call_graph.len(),
                        runtime_calls = report.runtime_call_observations,
                        findings = report.security_findings.len(),
                        warnings = report.test_info.compile_warnings,
                        covered = report.test_info.functionality_coverage.covered_items,
                        total = report.test_info.functionality_coverage.public_items,
                        pct = report.test_info.functionality_coverage.coverage_percent
                    );

                    if show_inventory {
                        print_crate_inventory(&inventory, &report);
                    }
                }
            }
            Err(err) => {
                totals.crates_failed += 1;
                failures.push((crate_name, format!("report failed: {err}")));
            }
        }
    }

    if let Err(err) = write_workspace_summary(&output_dir, &totals) {
        eprintln!("Failed to write workspace summary: {err}");
        process::exit(1);
    }

    print_totals_text(&totals, &output_dir);
    if let Some(report) = dependency_report.as_ref() {
        print_dependency_summary(report);
        println!(
            "  dependency report: {}",
            output_dir.join("dependency-metrics.txt").display()
        );
    }
    if show_inventory {
        println!("  per-crate output included above");
    }

    if !failures.is_empty() {
        println!("\nFailures");
        for (name, err) in failures {
            println!("  {name}: {err}");
        }
        process::exit(1);
    }
}
