use std::path::{Path, PathBuf};
use crate::crate_model::*;
use crate::cargo_toml;
use crate::gitvisible;
use crate::source_index;
use crate::dependency_analyzer;
use crate::api_analyzer;
use crate::call_graph;
use crate::unsafe_analyzer;
use crate::security_analyzer;
use crate::tests;
use crate::footprint;
use crate::benchmarks;
use crate::standards;
use crate::render;
use crate::seo;
use crate::sitemap;
use crate::issue_report;

pub fn generate_report(
    crate_name: &str,
    crate_path: &Path,
    workspace_root: &Path,
) -> Result<CrateReport, crate::errors::AnalyzerError> {
    let cargo_toml_path = crate_path.join("Cargo.toml");
    if !cargo_toml_path.exists() {
        return Err(crate::errors::AnalyzerError::CrateNotFound(crate_name.into()));
    }

    let identity = cargo_toml::parse_cargo_toml(&cargo_toml_path)?;

    let (visible_files, hidden_count) = source_index::index_crate_sources(crate_path);
    let mut identity = identity;
    identity.visible_files = visible_files.clone();
    identity.hidden_files_count = hidden_count;

    let dependencies = dependency_analyzer::analyze_dependencies(&cargo_toml_path, workspace_root)?;

    let (public_api, _api_count) = api_analyzer::analyze_public_api(&visible_files);

    let call_graph_edges = call_graph::build_call_graph(&visible_files);

    let mut unsafe_findings = unsafe_analyzer::find_unsafe_blocks(&visible_files);
    let mut security_findings = security_analyzer::static_security_analysis(&visible_files);
    let mut all_findings = Vec::new();
    all_findings.append(&mut unsafe_findings);
    all_findings.append(&mut security_findings);

    let test_info = tests::collect_test_info(crate_path);

    let footprint = footprint::collect_footprint(crate_path, "idle");

    let benchmark_artifacts = benchmarks::collect_benchmarks(crate_path, workspace_root);

    let standards = standards::load_standards_matrix(crate_path, workspace_root);

    let visibility_report = crate::crate_model::VisibilityReport {
        visible_count: visible_files.len(),
        hidden_count,
        public_surface: format!("{} visible files", visible_files.len()),
        blocked_refs: Vec::new(),
        suspicious_visible: Vec::new(),
    };

    Ok(CrateReport {
        identity,
        dependencies,
        public_api,
        call_graph: call_graph_edges,
        security_findings: all_findings,
        test_info,
        footprint,
        benchmarks: benchmark_artifacts,
        standards,
        visibility: visibility_report,
        generated_at: chrono::offset::Local::now().to_rfc3339(),
        parser_confidence: "lexical".into(),
    })
}

pub fn save_report(report: &CrateReport, output_dir: &Path) -> Result<(), crate::errors::AnalyzerError> {
    let crate_dir = output_dir.join("crates").join(&report.identity.name);
    std::fs::create_dir_all(&crate_dir)
        .map_err(|e| crate::errors::AnalyzerError::IoError(e.to_string()))?;

    let html = render::render_html_report(report);
    std::fs::write(crate_dir.join("report.html"), html)
        .map_err(|e| crate::errors::AnalyzerError::IoError(e.to_string()))?;

    let json = render::render_json_report(report);
    std::fs::write(crate_dir.join("report.json"), json)
        .map_err(|e| crate::errors::AnalyzerError::IoError(e.to_string()))?;

    Ok(())
}
