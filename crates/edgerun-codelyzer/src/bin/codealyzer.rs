use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process;

use edgerun_codelyzer::codealyzer::{
    collect_dependency_footprints,
    generate_report,
    save_report,
    write_dependency_footprints,
};
use edgerun_codelyzer::codealyzer::crate_model::{CrateReport, CrateType};

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
        "Usage: {program} [--workspace-root <path>] [--out <path>] [--inventory] [--dependency-metrics] [--clean-targets] [--json]"
    );
    println!();
    println!("Run the crate analyzer across crates/ and aggregate totals.");
    println!(
        "Defaults: workspace root = current Cargo workspace, output dir = target/codealyzer"
    );
    println!("  --inventory  print a per-crate contents summary");
    println!("  --dependency-metrics");
    println!("               collect non-edgerun transitive dependency size/LOC metrics");
    println!("  --clean-targets");
    println!("               remove all target directories under workspace root and exit");
    println!("  --json       emit machine-readable JSON summary");
}

fn parse_args() -> (PathBuf, PathBuf, bool, bool, bool, bool) {
    let mut workspace_root: Option<PathBuf> = None;
    let mut output_dir: Option<PathBuf> = None;
    let mut show_inventory = false;
    let mut emit_json = false;
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
            "--json" => {
                emit_json = true;
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
            unknown => {
                if !unknown.starts_with('-') {
                    workspace_root.get_or_insert_with(|| PathBuf::from(unknown));
                }
            }
        }
    }

    if show_help {
        print_usage(&std::env::args().next().unwrap_or_else(|| "codealyzer".into()));
        process::exit(0);
    }

    (
        workspace_root.unwrap_or_else(edgerun_codelyzer::codealyzer::workspace::get_workspace_root),
        output_dir.unwrap_or_else(|| PathBuf::from("target/codealyzer")),
        show_inventory,
        emit_json,
        collect_metrics,
        clean_targets,
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
    println!("  total_target_size: {} ({} bytes)", format_byte_size(total_size_bytes), total_size_bytes);
    println!("  removed_size: {} ({} bytes)", format_byte_size(removed_size_bytes), removed_size_bytes);

    for report in reports {
        if report.removed {
            println!(
                "  removed: {} ({})",
                report.path.display(),
                format_byte_size(report.size_bytes)
            );
        } else if let Some(err) = report.error.as_deref() {
            println!(
                "  failed:  {} ({})",
                report.path.display(),
                err
            );
        }
    }
}

fn print_target_cleanup_json(reports: &[TargetDirectoryReport]) -> String {
    let removed = reports.iter().filter(|r| r.removed).count();
    let failed = reports.len().saturating_sub(removed);
    let total_size_bytes: u64 = reports.iter().map(|r| r.size_bytes).sum();
    let removed_size_bytes: u64 = reports
        .iter()
        .filter(|r| r.removed)
        .map(|r| r.size_bytes)
        .sum();

    let mut out = String::new();
    out.push_str("{\"target_cleanup\":{");
    out.push_str(&format!("\"found\":{},", reports.len()));
    out.push_str(&format!("\"removed\":{},", removed));
    out.push_str(&format!("\"failed\":{},", failed));
    out.push_str(&format!("\"total_size_bytes\":{},", total_size_bytes));
    out.push_str(&format!("\"removed_size_bytes\":{},", removed_size_bytes));
    out.push_str("\"entries\":[");

    for (idx, report) in reports.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }

        let status = if report.removed { "removed" } else { "failed" };
        let size = report.size_bytes;
        let error = report.error.as_deref().unwrap_or("null");

        out.push('{');
        out.push_str(&format!(
            "\"path\":\"{}\",",
            escape_json(&report.path.display().to_string())
        ));
        out.push_str(&format!("\"size_bytes\":{},", size));
        out.push_str(&format!("\"status\":\"{status}\","));
        if report.error.is_some() {
            out.push_str(&format!("\"error\":\"{}\"", escape_json(error)));
        } else {
            out.push_str("\"error\":null");
        }
        out.push('}');
    }

    out.push_str("]}}");
    out
}

fn list_crate_dirs(workspace_root: &Path) -> Vec<PathBuf> {
    let crates_root = workspace_root.join("crates");
    if !crates_root.exists() {
        return Vec::new();
    }

    let mut dirs = Vec::new();
    let entries = match std::fs::read_dir(&crates_root) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if path.join("Cargo.toml").exists() {
            dirs.push(path);
        }
    }

    dirs.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
    dirs
}

fn update_totals(totals: &mut Totals, report: &CrateReport) {
    totals.crates_success += 1;
    totals.total_visible_files += report.identity.visible_files.len();
    totals.total_hidden_files += report.identity.hidden_files_count;
    totals.total_dependencies += report.dependencies.len();
    totals.total_external_dependencies += report.dependencies.iter().filter(|dep| !dep.is_workspace).count();
    totals.total_dependency_weight += report.dependencies.iter().map(|dep| dep.weight).sum::<usize>();
    totals.total_public_api += report.public_api.len();
    totals.total_call_edges += report.call_graph.len();
    totals.total_security_findings += report.security_findings.len();
    totals.total_test_files += report.test_info.total;
    totals.total_unit_tests += report.test_info.unit_tests;
    totals.total_integration_tests += report.test_info.integration_tests;
    totals.total_doc_tests += report.test_info.doc_tests;
    totals.total_compile_warnings += report.test_info.compile_warnings;
    totals.total_public_items += report
        .test_info
        .functionality_coverage
        .public_items;
    totals.total_covered_public_items += report
        .test_info
        .functionality_coverage
        .covered_items;

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

fn gather_crate_inventory(crate_dir: &Path, visible_files: &[PathBuf], hidden_files: usize) -> CrateInventory {
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

fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

fn normalize_toml_string(s: &str) -> &str {
    if s.len() >= 2 {
        let bytes = s.as_bytes();
        if (bytes[0] == b'"' && bytes[s.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[s.len() - 1] == b'\'')
        {
            return &s[1..s.len() - 1];
        }
    }
    s
}

fn json_string_vec(values: &[String]) -> String {
    let mut out = String::new();
    out.push('[');
    for (idx, item) in values.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        out.push('"');
        out.push_str(&escape_json(item));
        out.push('"');
    }
    out.push(']');
    out
}

fn json_exts(exts: &[(String, usize)]) -> String {
    let mut out = String::new();
    out.push('[');
    for (idx, (ext, count)) in exts.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"ext\":\"{}\",\"count\":{}}}",
            escape_json(ext),
            count
        ));
    }
    out.push(']');
    out
}

fn dependencies_json_list(deps: &[edgerun_codelyzer::codealyzer::crate_model::Dependency]) -> String {
    let mut out = String::new();
    out.push('[');
    for (idx, dep) in deps.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }

        out.push('{');
        out.push_str(&format!("\"name\":\"{}\"", escape_json(&dep.name)));
        out.push(',');
        out.push_str(&format!("\"source\":\"{}\"", dep.source.as_str()));
        out.push(',');
        out.push_str(&format!("\"kind\":\"{:?}\"", dep.kind));
        out.push(',');
        out.push_str(&format!("\"is_workspace\":{}", dep.is_workspace));
        out.push(',');
        out.push_str(&format!("\"weight\":{}", dep.weight));
        out.push(',');
        if let Some(reference) = dep.version_req.as_deref() {
            out.push_str(&format!("\"version_req\":\"{}\"", escape_json(reference)));
        } else {
            out.push_str("\"version_req\":null");
        }
        out.push(',');
        out.push_str(&format!("\"optional\":{}", dep.optional));
        out.push('}');
    }
    out.push(']');
    out
}

fn crate_json(record: &CrateInventory, report: &CrateReport) -> String {
    let deps_internal = report.dependencies.iter().filter(|dep| dep.is_workspace).count();
    let deps_external = report.dependencies.iter().filter(|dep| !dep.is_workspace).count();
    let visible_total = report.identity.visible_files.len();
    let compile_status = report
        .test_info
        .compile_status
        .as_deref()
        .unwrap_or("unknown");
    let compile_exit = report
        .test_info
        .compile_exit_code
        .map(|code| code.to_string())
        .unwrap_or_else(|| "null".to_string());
    let compile_warnings = report.test_info.compile_warnings;

    format!(
        "{{\"crate\":\"{}\",\"path\":\"{}\",\"version\":\"{}\",\"edition\":\"{}\",\"crate_type\":\"{}\",\"dependencies\":{{\"total\":{},\"internal\":{},\"external\":{},\"weight\":{} }},\"build\":{{\"status\":\"{}\",\"exit_code\":{},\"warnings\":{} }},\"files\":{{\"visible\":{},\"hidden\":{},\"visible_rs\":{},\"ext\":{}}},\"contents\":{{\"top_dirs\":{},\"top_files\":{},\"has_src_dir\":{},\"has_examples_dir\":{},\"has_tests_dir\":{},\"has_benches_dir\":{},\"has_build_rs\":{}}},\"analysis\":{{\"public_api\":{},\"call_edges\":{},\"security_findings\":{},\"test_files\":{},\"unit_tests\":{},\"integration_tests\":{},\"doc_tests\":{},\"coverage\":{{\"covered\":{},\"total\":{},\"percent\":{}}}}},\"dependency_list\":{}}}",
        escape_json(normalize_toml_string(&report.identity.name)),
        escape_json(&report.identity.path.to_string_lossy()),
        escape_json(normalize_toml_string(&report.identity.version)),
        escape_json(normalize_toml_string(&report.identity.edition)),
        report.identity.crate_type.as_str(),
        report.dependencies.len(),
        deps_internal,
        deps_external,
        report.dependencies.iter().map(|dep| dep.weight).sum::<usize>(),
        compile_status,
        compile_exit,
        compile_warnings,
        visible_total,
        record.hidden_files,
        record.visible_rs_files,
        json_exts(&record.extension_counts),
        json_string_vec(&record.top_dirs),
        json_string_vec(&record.top_files),
        record.has_src_dir,
        record.has_examples_dir,
        record.has_tests_dir,
        record.has_benches_dir,
        record.has_build_rs,
        report.public_api.len(),
        report.call_graph.len(),
        report.security_findings.len(),
        report.test_info.total,
        report.test_info.unit_tests,
        report.test_info.integration_tests,
        report.test_info.doc_tests,
        report.test_info.functionality_coverage.covered_items,
        report.test_info.functionality_coverage.public_items,
        report.test_info.functionality_coverage.coverage_percent,
        dependencies_json_list(&report.dependencies)
    )
}

fn print_crate_inventory(record: &CrateInventory, report: &CrateReport) {
    let deps_internal = report.dependencies.iter().filter(|dep| dep.is_workspace).count();
    let deps_external = report.dependencies.iter().filter(|dep| !dep.is_workspace).count();
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
        if record.hidden_files > 0 { "true" } else { "false" },
        report.test_info.compile_status.as_deref().unwrap_or("unknown"),
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
        report.dependencies.iter().map(|dep| dep.weight).sum::<usize>()
    );
    println!(
        "  analysis: api={}, edges={}, findings={}, warnings={}, tests={} ({}), compiled={}",
        report.public_api.len(),
        report.call_graph.len(),
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
    println!("  security_findings:     {}", totals.total_security_findings);
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
    println!("  integration_tests:     {}", totals.total_integration_tests);
    println!("  doc_tests:             {}", totals.total_doc_tests);
    println!("  output:                {}", out_dir.display());
}

fn print_totals_json(totals: &Totals, crates_json: &[String]) -> String {
    format!(
        "{{\"totals\":{{\"crates_attempted\":{},\"crates_ok\":{},\"crates_failed\":{},\"crates_compiled\":{},\"crates_compile_failed\":{},\"library_crates\":{},\"binary_crates\":{},\"library_binary_crates\":{},\"unknown_crates\":{},\"visible_files\":{},\"hidden_files\":{},\"dependencies\":{},\"external_dependencies\":{},\"dependency_weight\":{},\"public_api_items\":{},\"call_graph_edges\":{},\"security_findings\":{},\"compile_warnings\":{},\"functionality_coverage_covered\":{},\"functionality_coverage_total\":{},\"functionality_coverage_percent\":{},\"tests\":{},\"unit_tests\":{},\"integration_tests\":{},\"doc_tests\":{} }},\"crates\":[{}]}}",
        totals.crates_attempted,
        totals.crates_success,
        totals.crates_failed,
        totals.crates_compiled,
        totals.crates_compile_failed,
        totals.library_crates,
        totals.binary_crates,
        totals.library_binary_crates,
        totals.unknown_crates,
        totals.total_visible_files,
        totals.total_hidden_files,
        totals.total_dependencies,
        totals.total_external_dependencies,
        totals.total_dependency_weight,
        totals.total_public_api,
        totals.total_call_edges,
        totals.total_security_findings,
        totals.total_compile_warnings,
        totals.total_covered_public_items,
        totals.total_public_items,
        percentage_covered(totals.total_covered_public_items, totals.total_public_items),
        totals.total_test_files,
        totals.total_unit_tests,
        totals.total_integration_tests,
        totals.total_doc_tests,
        crates_json.join(",")
    )
}

fn print_dependency_summary(report: &edgerun_json::JsonValue) {
    let summary = report.get("summary").and_then(edgerun_json::JsonValue::as_object);
    println!("\nDependency metrics");
    if let Some(summary) = summary {
        let crate_count = summary
            .get("workspace_root_crate_count")
            .and_then(edgerun_json::JsonValue::as_u64)
            .unwrap_or(0);
        let with_external = summary
            .get("crates_with_external_dependencies")
            .and_then(edgerun_json::JsonValue::as_u64)
            .unwrap_or(0);
        let dependency_count = summary
            .get("distinct_external_dependency_count")
            .and_then(edgerun_json::JsonValue::as_u64)
            .unwrap_or(0);
        let dep_size_bytes = summary
            .get("distinct_external_dependency_size_bytes")
            .and_then(edgerun_json::JsonValue::as_u64)
            .unwrap_or(0);
        let dep_size_mb = dep_size_bytes as f64 / 1024.0 / 1024.0;

        println!("  workspace crates: {crate_count}");
        println!("  crates with non-edgerun external dependencies: {with_external}");
        println!("  distinct non-edgerun dependencies: {dependency_count}");
        println!("  dependency disk footprint: {dep_size_bytes} bytes ({dep_size_mb:.2} MB)");
    }
}

fn write_workspace_summary(
    out_dir: &Path,
    totals: &Totals,
    crate_records: &[String],
) -> Result<(), String> {
    std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let summary = print_totals_json(totals, crate_records);
    let summary_path = out_dir.join("workspace-summary.json");
    std::fs::write(&summary_path, summary).map_err(|e| e.to_string())?;
    Ok(())
}

fn main() {
    let (
        workspace_root,
        output_dir,
        show_inventory,
        emit_json,
        collect_metrics,
        clean_targets,
    ) = parse_args();
    if !workspace_root.join("Cargo.toml").exists() {
        eprintln!("Workspace root not found: {workspace_root:?}");
        process::exit(1);
    }

    if clean_targets {
        let targets = clean_target_dirs(&workspace_root);
        if emit_json {
            println!("{}", print_target_cleanup_json(&targets));
        } else {
            print_target_cleanup_summary(&targets);
        }
        process::exit(0);
    }

    let crates = list_crate_dirs(&workspace_root);
    if crates.is_empty() {
        eprintln!("No crate directories found under: {}", workspace_root.join("crates").display());
        process::exit(1);
    }

    if !emit_json {
        println!(
            "Analyzing {} crate directories in {}",
            crates.len(),
            workspace_root.display()
        );
    }

    let mut totals = Totals::default();
    let mut failures: Vec<(String, String)> = Vec::new();
    let mut crate_json_records: Vec<String> = Vec::new();
    let mut dependency_report: Option<edgerun_json::JsonValue> = None;

    if collect_metrics {
        if !emit_json {
            println!("Collecting dependency footprint metrics...");
        }
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
        match generate_report(&crate_name, &crate_dir, &workspace_root) {
            Ok(report) => {
                if let Err(err) = save_report(&report, &output_dir) {
                    totals.crates_failed += 1;
                    failures.push((crate_name, format!("save failed: {err}")));
                } else {
                    update_totals(&mut totals, &report);
                    let inventory =
                        gather_crate_inventory(&crate_dir, &report.identity.visible_files, report.identity.hidden_files_count);

                    crate_json_records.push(crate_json(&inventory, &report));

                    if !emit_json {
                        println!(
                            "  ok  {name}: deps={deps}, api={api}, edges={edges}, findings={findings}, warnings={warnings}, coverage={covered}/{total} ({pct}%)",
                            name = report.identity.name,
                            deps = report.dependencies.len(),
                            api = report.public_api.len(),
                            edges = report.call_graph.len(),
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
            }
            Err(err) => {
                totals.crates_failed += 1;
                failures.push((crate_name, format!("report failed: {err}")));
            }
        }
    }

    if let Err(err) = write_workspace_summary(&output_dir, &totals, &crate_json_records) {
        eprintln!("Failed to write workspace summary: {err}");
        process::exit(1);
    }

    if emit_json {
        println!("{}", print_totals_json(&totals, &crate_json_records));
        if let Some(report) = dependency_report {
            println!(
                "{{\"dependency_metrics_path\":\"{}\",\"dependency_metrics_report\":{}}}",
                output_dir.join("dependency-metrics.json").display(),
                edgerun_json::to_json_string(&report).unwrap_or_else(|_| "null".to_string())
            );
        }
    } else {
        print_totals_text(&totals, &output_dir);
        if let Some(report) = dependency_report.as_ref() {
            print_dependency_summary(report);
            println!("  dependency report: {}", output_dir.join("dependency-metrics.json").display());
        }
        if show_inventory {
            println!("  per-crate output included above");
        }
    }

    if !failures.is_empty() {
        println!("\nFailures");
        for (name, err) in failures {
            println!("  {name}: {err}");
        }
        process::exit(1);
    }
}
