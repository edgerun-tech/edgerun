use crate::codealyzer::api_analyzer;
use crate::codealyzer::benchmarks;
use crate::codealyzer::call_graph;
use crate::codealyzer::cargo_toml;
use crate::codealyzer::crate_model::*;
use crate::codealyzer::dependency_analyzer;
use crate::codealyzer::footprint;
use crate::codealyzer::render;
use crate::codealyzer::runtime_bridge;
use crate::codealyzer::security_analyzer;
use crate::codealyzer::source_index;
use crate::codealyzer::standards;
use crate::codealyzer::tests;
use crate::codealyzer::unsafe_analyzer;
use crate::timestamp_now;
use std::path::Path;

pub fn generate_report(
    crate_name: &str,
    crate_path: &Path,
    workspace_root: &Path,
    runtime_events: Option<&[runtime_bridge::RuntimeCallEvent]>,
) -> Result<CrateReport, crate::codealyzer::errors::AnalyzerError> {
    let cargo_toml_path = crate_path.join("Cargo.toml");
    if !cargo_toml_path.exists() {
        return Err(crate::codealyzer::errors::AnalyzerError::CrateNotFound(
            crate_name.into(),
        ));
    }

    let identity = cargo_toml::parse_cargo_toml(&cargo_toml_path)?;

    let (visible_files, hidden_count) = source_index::index_crate_sources(crate_path);
    let mut identity = identity;
    identity.visible_files = visible_files.clone();
    identity.hidden_files_count = hidden_count;

    let dependencies = dependency_analyzer::analyze_dependencies(
        &cargo_toml_path,
        workspace_root,
        &visible_files,
    )?;

    let (public_api, _api_count) = api_analyzer::analyze_public_api(&visible_files);

    let mut call_graph_edges = call_graph::build_call_graph(&visible_files);
    let runtime_call_observations = runtime_events.map_or(0, |events| {
        runtime_bridge::merge_runtime_calls_into_edges(&mut call_graph_edges, crate_path, events)
    });

    let mut unsafe_findings = unsafe_analyzer::find_unsafe_blocks(&visible_files);
    let mut security_findings = security_analyzer::static_security_analysis(&visible_files);
    let mut all_findings = Vec::new();
    all_findings.append(&mut unsafe_findings);
    all_findings.append(&mut security_findings);

    let test_info = tests::collect_test_info(crate_path, &public_api);

    let footprint = footprint::collect_footprint(crate_path, "idle");

    let benchmark_artifacts = benchmarks::collect_benchmarks(crate_path, workspace_root);

    let standards = standards::load_standards_matrix(crate_path, workspace_root);

    let visibility_report = crate::codealyzer::crate_model::VisibilityReport {
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
        runtime_call_observations,
        security_findings: all_findings,
        test_info,
        footprint,
        benchmarks: benchmark_artifacts,
        standards,
        visibility: visibility_report,
        generated_at: timestamp_now(),
        parser_confidence: "lexical".into(),
    })
}

pub fn save_report(
    report: &CrateReport,
    output_dir: &Path,
) -> Result<(), crate::codealyzer::errors::AnalyzerError> {
    let crate_dir = output_dir.join("crates").join(&report.identity.name);
    std::fs::create_dir_all(&crate_dir)
        .map_err(|e| crate::codealyzer::errors::AnalyzerError::IoError(e.to_string()))?;

    let html = render::render_html_report(report);
    std::fs::write(crate_dir.join("report.html"), html)
        .map_err(|e| crate::codealyzer::errors::AnalyzerError::IoError(e.to_string()))?;

    std::fs::write(crate_dir.join("report.json"), report.to_json_string())
        .map_err(|e| crate::codealyzer::errors::AnalyzerError::IoError(e.to_string()))?;

    Ok(())
}
