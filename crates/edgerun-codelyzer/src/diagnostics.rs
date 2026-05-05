#![allow(dead_code)]

//! Auto-discovery diagnostics system.
//! Scans for available linters/LSPs/formatters at startup, runs them,
//! and collects: lint errors, missing references, formatting issues,
//! test results, and code coverage data.

use std::{collections::HashSet, path::Path, process::Command, time::SystemTime};

// Use rkyv-normalized protocol types directly; no JSON or serde on this boundary.
pub use crate::generated::codeanalyzer::{
    Diagnostic, DiagnosticsReport, FormatIssue, GraphData, GraphEdge, GraphNode,
    MissingRef, TestResult, CoverageInfo,
};

fn current_timestamp() -> String {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default()
}

fn command_exists(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn has_npx_tool(tool: &str) -> bool {
    Command::new("npx")
        .args(["--yes", tool, "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn scan_tools(root_dir: &str) -> Vec<String> {
    let _ = root_dir;

    let tools_config: &[(&str, Option<&str>)] = &[
        ("cargo", Some("cargo")),
        ("rustfmt", Some("rustfmt")),
        ("cargo-clippy", Some("clippy")),
        ("flake8", Some("flake8")),
        ("pylint", Some("pylint")),
        ("mypy", Some("mypy")),
        ("black", Some("black")),
        ("pytest", Some("pytest")),
        ("eslint", Some("eslint")),
        ("tsc", Some("tsc")),
        ("prettier", Some("prettier")),
        ("jest", Some("jest")),
        ("clang-tidy", Some("clang-tidy")),
        ("cppcheck", Some("cppcheck")),
        ("clang-format", Some("clang-format")),
        ("golangci-lint", Some("golangci-lint")),
        ("go", Some("go")),
        ("checkstyle", Some("checkstyle")),
    ];

    let mut tools = Vec::new();
    for (cmd, name) in tools_config {
        if command_exists(cmd) {
            if let Some(n) = name {
                tools.push(n.to_string());
            }
        }
    }

    // Handle npx tools separately
    if has_npx_tool("eslint") {
        tools.push("npx:eslint".to_string());
    }
    if has_npx_tool("tsc") {
        tools.push("npx:tsc".to_string());
    }

    tools
}

pub fn collect_all(root_dir: &str, graph: &GraphData) -> DiagnosticsReport {
    let tools = scan_tools(root_dir);
    let mut diagnostics = Vec::new();
    let mut tests = Vec::new();
    let coverage = Vec::new();
    let mut formatting_issues = Vec::new();
    let mut missing_refs = Vec::new();

    if tools.iter().any(|t| t == "cargo") || tools.iter().any(|t| t == "clippy") {
        run_rust_checks(
            root_dir,
            &mut diagnostics,
            &mut tests,
            &mut formatting_issues,
        );
    }

    if tools.iter().any(|t| t == "flake8")
        || tools.iter().any(|t| t == "pylint")
        || tools.iter().any(|t| t == "mypy")
    {
        run_python_checks(
            root_dir,
            &tools,
            &mut diagnostics,
            &mut tests,
            &mut formatting_issues,
        );
    }

    if tools.iter().any(|t| t == "eslint")
        || tools.iter().any(|t| t == "npx:eslint")
        || tools.iter().any(|t| t == "npx:tsc")
    {
        run_js_checks(root_dir, &tools, &mut diagnostics, &mut formatting_issues);
    }

    if tools.iter().any(|t| t == "clang-tidy") || tools.iter().any(|t| t == "cppcheck") {
        run_c_checks(root_dir, &tools, &mut diagnostics);
    }

    collect_test_info(root_dir, &mut tests);

    find_missing_references(graph, &mut missing_refs);

    DiagnosticsReport {
        linters_available: tools,
        diagnostics,
        tests,
        coverage,
        formatting_issues,
        missing_references: missing_refs,
        scan_timestamp: current_timestamp(),
    }
}

fn run_rust_checks(
    root_dir: &str,
    diagnostics: &mut Vec<Diagnostic>,
    _tests: &mut Vec<TestResult>,
    format_issues: &mut Vec<FormatIssue>,
) {
    let out = Command::new("cargo")
        .args(["check", "--message-format", "short"])
        .current_dir(root_dir)
        .output();
    if let Ok(o) = out {
        let stderr = String::from_utf8_lossy(&o.stderr);
        let stdout = String::from_utf8_lossy(&o.stdout);
        for line in stderr.lines().chain(stdout.lines()) {
            if let Some(d) = parse_cargo_diagnostic(line, root_dir) {
                diagnostics.push(d);
            }
        }
    }

    let out = Command::new("cargo")
        .args(["test", "--no-run", "--message-format", "short"])
        .current_dir(root_dir)
        .output();
    if let Ok(o) = out {
        let stderr = String::from_utf8_lossy(&o.stderr);
        let stdout = String::from_utf8_lossy(&o.stdout);
        for line in stderr.lines().chain(stdout.lines()) {
            if line.contains("error[") {
                if let Some(d) = parse_cargo_diagnostic(line, root_dir) {
                    diagnostics.push(d);
                }
            }
        }
    }

    let out = Command::new("rustfmt")
        .args(["--check", "src/lib.rs", "src/main.rs"])
        .current_dir(root_dir)
        .output();
    if let Ok(o) = out {
        if !o.status.success() {
            format_issues.push(FormatIssue {
                file: "src/".to_string(),
                line: None,
                message: "rustfmt: code is not properly formatted".to_string(),
            });
        }
    }
}

fn parse_cargo_diagnostic(line: &str, _root_dir: &str) -> Option<Diagnostic> {
    if line.contains("error") || line.contains("warning") {
        let severity = if line.contains("error") {
            "error"
        } else {
            "warning"
        };
        let file = line.split(':').next().unwrap_or("").to_string();
        let line_num = line.split(':').nth(1).and_then(|n| n.trim().parse().ok());
        Some(Diagnostic {
            file: file.trim().to_string(),
            line: line_num,
            column: None,
            severity: severity.to_string(),
            message: line.trim().to_string(),
            source: "cargo".to_string(),
        })
    } else {
        None
    }
}

fn run_python_checks(
    root_dir: &str,
    tools: &[String],
    diagnostics: &mut Vec<Diagnostic>,
    tests: &mut Vec<TestResult>,
    format_issues: &mut Vec<FormatIssue>,
) {
    if tools.iter().any(|t| t == "flake8") {
        let out = Command::new("flake8")
            .args(["--max-line-length", "120", "."])
            .current_dir(root_dir)
            .output();
        if let Ok(o) = out {
            for line in String::from_utf8_lossy(&o.stdout).lines() {
                diagnostics.push(parse_linter_line(line, "flake8"));
            }
        }
    }

    if tools.iter().any(|t| t == "mypy") {
        let out = Command::new("mypy")
            .args([".", "--ignore-missing-imports"])
            .current_dir(root_dir)
            .output();
        if let Ok(o) = out {
            for line in String::from_utf8_lossy(&o.stdout)
                .lines()
                .chain(String::from_utf8_lossy(&o.stderr).lines())
            {
                if line.contains(": error:") || line.contains(": warning:") {
                    diagnostics.push(parse_linter_line(line, "mypy"));
                }
            }
        }
    }

    if tools.iter().any(|t| t == "pytest") {
        let out = Command::new("pytest")
            .args(["--collect-only", "-q"])
            .current_dir(root_dir)
            .output();
        if let Ok(o) = out {
            for line in String::from_utf8_lossy(&o.stdout).lines() {
                if line.contains("::test_") || line.contains("::Test") {
                    tests.push(TestResult {
                        name: line.trim().to_string(),
                        passed: true,
                        file: None,
                        message: None,
                        duration_ms: None,
                    });
                }
            }
        }
    }

    if tools.iter().any(|t| t == "black") {
        let out = Command::new("black")
            .args(["--check", "."])
            .current_dir(root_dir)
            .output();
        if let Ok(o) = out {
            if !o.status.success() {
                for line in String::from_utf8_lossy(&o.stderr).lines() {
                    if line.contains("would reformat") {
                        format_issues.push(FormatIssue {
                            file: line.trim().to_string(),
                            line: None,
                            message: "black: file needs reformatting".to_string(),
                        });
                    }
                }
            }
        }
    }
}

fn run_js_checks(
    root_dir: &str,
    tools: &[String],
    diagnostics: &mut Vec<Diagnostic>,
    format_issues: &mut Vec<FormatIssue>,
) {
    if tools.iter().any(|t| t == "eslint") {
        let out = Command::new("eslint")
            .args([".", "--format", "compact"])
            .current_dir(root_dir)
            .output();
        if let Ok(o) = out {
            for line in String::from_utf8_lossy(&o.stdout).lines() {
                diagnostics.push(parse_linter_line(line, "eslint"));
            }
        }
    } else if tools.iter().any(|t| t == "npx:eslint") {
        let out = Command::new("npx")
            .args(["--yes", "eslint", ".", "--format", "compact"])
            .current_dir(root_dir)
            .output();
        if let Ok(o) = out {
            for line in String::from_utf8_lossy(&o.stdout).lines() {
                diagnostics.push(parse_linter_line(line, "eslint"));
            }
        }
    }

    if tools.iter().any(|t| t == "npx:tsc") {
        let out = Command::new("npx")
            .args(["--yes", "tsc", "--noEmit"])
            .current_dir(root_dir)
            .output();
        if let Ok(o) = out {
            for line in String::from_utf8_lossy(&o.stderr)
                .lines()
                .chain(String::from_utf8_lossy(&o.stdout).lines())
            {
                if line.contains("error TS") {
                    diagnostics.push(parse_linter_line(line, "tsc"));
                }
            }
        }
    }

    if tools.iter().any(|t| t == "prettier") {
        let out = Command::new("prettier")
            .args(["--check", "src/**/*.{js,ts,tsx}"])
            .current_dir(root_dir)
            .output();
        if let Ok(o) = out {
            if !o.status.success() {
                format_issues.push(FormatIssue {
                    file: "src/".to_string(),
                    line: None,
                    message: "prettier: files need formatting".to_string(),
                });
            }
        }
    }
}

fn run_c_checks(_root_dir: &str, tools: &[String], diagnostics: &mut Vec<Diagnostic>) {
    if tools.iter().any(|t| t == "cppcheck") {
        let out = Command::new("cppcheck")
            .args(["--enable=all", "--quiet", "."])
            .output();
        if let Ok(o) = out {
            for line in String::from_utf8_lossy(&o.stderr)
                .lines()
                .chain(String::from_utf8_lossy(&o.stdout).lines())
            {
                diagnostics.push(parse_linter_line(line, "cppcheck"));
            }
        }
    }

    if tools.iter().any(|t| t == "clang-tidy") {
        let out = Command::new("clang-tidy")
            .args(["-p", "build", "."])
            .output();
        if let Ok(o) = out {
            for line in String::from_utf8_lossy(&o.stderr)
                .lines()
                .chain(String::from_utf8_lossy(&o.stdout).lines())
            {
                if !line.is_empty() {
                    diagnostics.push(parse_linter_line(line, "clang-tidy"));
                }
            }
        }
    }
}

fn collect_test_info(root_dir: &str, tests: &mut Vec<TestResult>) {
    let test_dirs = ["tests", "test", "__tests__", "spec", "specs"];
    for dir in &test_dirs {
        let path = format!("{}/{}", root_dir.trim_end_matches('/'), dir);
        if Path::new(&path).is_dir() {
            if let Ok(entries) = std::fs::read_dir(&path) {
                for entry in entries.flatten() {
                    if entry.path().is_file() {
                        if let Some(name) = entry.file_name().to_str() {
                            tests.push(TestResult {
                                name: format!("{}/{}", dir, name),
                                passed: true,
                                file: Some(format!("{}/{}", dir, name)),
                                message: None,
                                duration_ms: None,
                            });
                        }
                    }
                }
            }
        }
    }

    let deps_path = format!("{}/target/debug/deps", root_dir.trim_end_matches('/'));
    if let Ok(entries) = std::fs::read_dir(&deps_path) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with("-test") || name.contains("test") {
                    tests.push(TestResult {
                        name: name.to_string(),
                        passed: true,
                        file: None,
                        message: None,
                        duration_ms: None,
                    });
                }
            }
        }
    }
}

fn find_missing_references(graph: &GraphData, missing_refs: &mut Vec<MissingRef>) {
    let node_ids: HashSet<&str> = graph.nodes.iter().map(|n| n.id.as_str()).collect();

    for e in &graph.edges {
        let target = e.target.as_str();
        if !node_ids.contains(target)
            && !target.starts_with("UNKNOWN_")
            && !target.starts_with("INDIRECT_")
            && !target.starts_with("MACRO_")
            && !target.starts_with("UNRESOLVED_")
        {
            missing_refs.push(MissingRef {
                name: target.to_string(),
                referenced_by: e.source.clone(),
                kind: "function".to_string(),
            });
        }
    }

    missing_refs.sort_by(|a, b| a.name.cmp(&b.name));
    missing_refs.dedup_by(|a, b| a.name == b.name);
    if missing_refs.len() > 50 {
        missing_refs.truncate(50);
    }
}

fn parse_linter_line(line: &str, source: &str) -> Diagnostic {
    let parts: Vec<&str> = line.splitn(4, ": ").collect();
    let (file, line_num, col, severity, message) = if parts.len() >= 4 {
        let loc_parts: Vec<&str> = parts[0].split(':').collect();
        let file = loc_parts
            .first()
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let line_num = loc_parts.get(1).and_then(|s| s.trim().parse().ok());
        let col = loc_parts.get(2).and_then(|s| s.trim().parse().ok());
        let severity = if parts[1].contains("error") || parts[2].contains("error") {
            "error"
        } else if parts[1].contains("warning") || parts[2].contains("warning") {
            "warning"
        } else {
            "info"
        };
        let message = parts
            .last()
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        (file, line_num, col, severity, message)
    } else {
        (
            line.trim().to_string(),
            None,
            None,
            "info",
            line.trim().to_string(),
        )
    };

    Diagnostic {
        file,
        line: line_num,
        column: col,
        severity: severity.to_string(),
        message,
        source: source.to_string(),
    }
}

fn run_cargo_linter<F>(
    project_root: &str,
    file_path: Option<&str>,
    args: &[&str],
    diagnostics: &mut Vec<Diagnostic>,
    matches_filter: F,
) where
    F: Fn(&Diagnostic) -> bool,
{
    let mut cmd = Command::new("cargo");
    cmd.args(args);
    if let Some(f) = file_path {
        cmd.arg("--manifest-path").arg(f);
    }
    if let Ok(o) = cmd.current_dir(project_root).output() {
        for line in String::from_utf8_lossy(&o.stderr)
            .lines()
            .chain(String::from_utf8_lossy(&o.stdout).lines())
        {
            if let Some(d) = parse_cargo_diagnostic(line, project_root) {
                if matches_filter(&d) {
                    diagnostics.push(d);
                }
            }
        }
    }
}

fn run_simple_linter<F>(
    project_root: &str,
    file_path: Option<&str>,
    source: &str,
    cmd_args: &[&str],
    filter_lines: bool,
    diagnostics: &mut Vec<Diagnostic>,
    matches_filter: F,
) where
    F: Fn(&Diagnostic) -> bool,
{
    let mut cmd = Command::new(cmd_args[0]);
    for arg in &cmd_args[1..] {
        cmd.arg(arg);
    }
    if let Some(f) = file_path {
        cmd.arg(f);
    } else {
        cmd.arg(".");
    }
    let output_check = if cmd_args[0] == "clang-tidy" {
        cmd.output()
    } else {
        cmd.current_dir(project_root).output()
    };
    if let Ok(o) = output_check {
        for line in String::from_utf8_lossy(&o.stdout)
            .lines()
            .chain(String::from_utf8_lossy(&o.stderr).lines())
        {
            if filter_lines && !line.contains(": error:") && !line.contains(": warning:") {
                continue;
            }
            if line.is_empty() && source == "clang-tidy" {
                continue;
            }
            let d = parse_linter_line(line, source);
            if matches_filter(&d) {
                diagnostics.push(d);
            }
        }
    }
}

/// Run linters on a single file or all files. Used by /api/diagnostics
/// endpoint. When file_path is Some, lints only that file; otherwise lints the
/// whole project.
pub fn run_linters(project_root: &str, file_path: Option<&str>) -> Vec<Diagnostic> {
    let tools = scan_tools(project_root);
    let mut diagnostics = Vec::new();

    let matches_filter = |d: &Diagnostic| file_path.as_ref().is_none_or(|fp| d.file.ends_with(fp));

    for tool in &tools {
        match tool.as_str() {
            "cargo" => {
                run_cargo_linter(
                    project_root,
                    file_path,
                    &["check", "--message-format", "short"],
                    &mut diagnostics,
                    matches_filter,
                );
            }
            "clippy" => {
                run_cargo_linter(
                    project_root,
                    file_path,
                    &["clippy", "--message-format", "short"],
                    &mut diagnostics,
                    matches_filter,
                );
            }
            "flake8" => {
                run_simple_linter(
                    project_root,
                    file_path,
                    "flake8",
                    &["flake8", "--max-line-length", "120"],
                    false,
                    &mut diagnostics,
                    matches_filter,
                );
            }
            "mypy" => {
                run_simple_linter(
                    project_root,
                    file_path,
                    "mypy",
                    &["mypy", "--ignore-missing-imports"],
                    true,
                    &mut diagnostics,
                    matches_filter,
                );
            }
            "eslint" => {
                run_simple_linter(
                    project_root,
                    file_path,
                    "eslint",
                    &["eslint", "--format", "compact"],
                    false,
                    &mut diagnostics,
                    matches_filter,
                );
            }
            "npx:eslint" => {
                run_simple_linter(
                    project_root,
                    file_path,
                    "eslint",
                    &["npx", "--yes", "eslint", "--format", "compact"],
                    false,
                    &mut diagnostics,
                    matches_filter,
                );
            }
            "pylint" => {
                run_simple_linter(
                    project_root,
                    file_path,
                    "pylint",
                    &["pylint"],
                    false,
                    &mut diagnostics,
                    matches_filter,
                );
            }
            "clang-tidy" => {
                run_simple_linter(
                    project_root,
                    file_path,
                    "clang-tidy",
                    &["clang-tidy", "-p", "build"],
                    false,
                    &mut diagnostics,
                    matches_filter,
                );
            }
            "cppcheck" => {
                run_simple_linter(
                    project_root,
                    file_path,
                    "cppcheck",
                    &["cppcheck", "--enable=all", "--quiet"],
                    false,
                    &mut diagnostics,
                    matches_filter,
                );
            }
            _ => {}
        }
    }

    diagnostics
}

/// Lint a single file.
#[allow(dead_code)]
pub fn lint_single_file(root: &str, file: &str) -> Vec<Diagnostic> {
    run_linters(root, Some(file))
}

/// Get diagnostics summary as markdown for chat context.
pub fn get_diagnostics_summary(root_dir: &str, graph: &GraphData) -> String {
    let report = collect_all(root_dir, graph);
    let mut md = String::new();

    if !report.linters_available.is_empty() {
        md.push_str(&format!(
            "## Available Tools\n{}\n",
            report.linters_available.join(", ")
        ));
    }

    let errors: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.severity == "error")
        .collect();
    let warnings: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.severity == "warning")
        .collect();
    if !errors.is_empty() || !warnings.is_empty() {
        md.push_str(&format!(
            "## Lint Results\n- Errors: {}\n- Warnings: {}\n",
            errors.len(),
            warnings.len()
        ));
        for d in errors.iter().take(10) {
            md.push_str(&format!(
                "  - [{}] {}:{}: {}\n",
                d.source,
                d.file,
                d.line.unwrap_or(0),
                d.message
            ));
        }
        for d in warnings.iter().take(5) {
            md.push_str(&format!(
                "  - [{}] {}:{}: {}\n",
                d.source,
                d.file,
                d.line.unwrap_or(0),
                d.message
            ));
        }
    }

    if !report.tests.is_empty() {
        md.push_str(&format!(
            "## Tests\n- {} test files found\n",
            report.tests.len()
        ));
    }

    if !report.formatting_issues.is_empty() {
        md.push_str(&format!(
            "## Formatting\n- {} files need formatting\n",
            report.formatting_issues.len()
        ));
    }

    if !report.missing_references.is_empty() {
        md.push_str(&format!(
            "## Missing References\n- {} unresolved function references\n",
            report.missing_references.len()
        ));
        for r in report.missing_references.iter().take(10) {
            md.push_str(&format!(
                "  - `{}` called from `{}`\n",
                r.name, r.referenced_by
            ));
        }
    }

    md
}
