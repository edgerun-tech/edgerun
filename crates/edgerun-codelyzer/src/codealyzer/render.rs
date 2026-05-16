use crate::codealyzer::crate_model::*;

pub fn render_html_report(report: &CrateReport) -> String {
    let mut html = String::new();

    html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    html.push_str(&format!(
        "<title>{} - Crate Analysis Report</title>\n",
        report.identity.name
    ));
    html.push_str("<meta charset=\"UTF-8\">\n");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
    html.push_str("</head>\n<body>\n");

    html.push_str(&render_summary(report));
    html.push_str(&render_dependencies(report));
    html.push_str(&render_public_api(report));
    html.push_str(&render_call_graph(report));
    html.push_str(&render_runtime_call_summary(report));
    html.push_str(&render_security(report));
    html.push_str(&render_tests(report));
    html.push_str(&render_footprint(report));
    html.push_str(&render_benchmarks(report));
    html.push_str(&render_visibility(report));
    html.push_str(&render_issue_buttons(report));

    html.push_str("</body>\n</html>\n");
    html
}

fn render_summary(report: &CrateReport) -> String {
    let mut s = String::new();
    s.push_str(&format!("<h1>{}</h1>\n", report.identity.name));
    s.push_str(&format!("<p>Version: {}</p>\n", report.identity.version));
    s.push_str(&format!(
        "<p>Type: {}</p>\n",
        report.identity.crate_type.as_str()
    ));
    s.push_str(&format!("<p>Status: analyzed</p>\n"));
    s.push_str(&format!(
        "<p>Visibility: {}</p>\n",
        if report.visibility.hidden_count > 0 {
            "public subset"
        } else {
            "fully public"
        }
    ));
    s.push_str(&format!(
        "<p>Compile check: {} (code={})</p>\n",
        report
            .test_info
            .compile_status
            .as_deref()
            .unwrap_or("not run"),
        report
            .test_info
            .compile_exit_code
            .map(|v| v.to_string())
            .unwrap_or_else(|| "n/a".to_string())
    ));
    s.push_str(&format!(
        "<p>Functionality coverage: {}/{} ({:}%)</p>\n",
        report.test_info.functionality_coverage.covered_items,
        report.test_info.functionality_coverage.public_items,
        report.test_info.functionality_coverage.coverage_percent
    ));
    s.push_str(&format!(
        "<p>Tests: {} passing / {} failing / {} ignored</p>\n",
        report.test_info.passing.unwrap_or(0),
        report.test_info.failing.unwrap_or(0),
        report.test_info.ignored
    ));
    s.push_str(&format!(
        "<p>Public API items: {}</p>\n",
        report.public_api.len()
    ));
    s.push_str(&format!(
        "<p>Dependencies: {} internal / {} external</p>\n",
        report
            .dependencies
            .iter()
            .filter(|d| d.is_workspace)
            .count(),
        report
            .dependencies
            .iter()
            .filter(|d| !d.is_workspace)
            .count()
    ));
    s.push_str(&format!(
        "<p>Dependency weight: {} total references</p>\n",
        report
            .dependencies
            .iter()
            .map(|dep| dep.weight)
            .sum::<usize>()
    ));
    s.push_str(&format!(
        "<p>Unsafe: {} blocks</p>\n",
        report
            .security_findings
            .iter()
            .filter(|f| f.title.contains("Unsafe"))
            .count()
    ));
    s.push_str(&format!(
        "<p>Security findings: {}</p>\n",
        report.security_findings.len()
    ));
    s.push_str(&format!("<p>Generated: {}</p>\n", report.generated_at));
    s.push_str(&format!(
        "<p>Parser confidence: {}</p>\n",
        report.parser_confidence
    ));
    s.push_str(&format!(
        "<p>Runtime call observations: {}</p>\n",
        report.runtime_call_observations
    ));
    s
}

fn render_dependencies(report: &CrateReport) -> String {
    let mut s = String::from(
        "<h2>Dependencies</h2>\n<table border=\"1\"><tr><th>Dependency</th><th>Kind</th><th>Source</th><th>Optional</th><th>Weight</th><th>Features</th><th>Reason</th><th>Visible</th></tr>\n",
    );
    for dep in &report.dependencies {
        s.push_str(&format!(
            "<tr><td>{}</td><td>{:?}</td><td>{}</td><td>{}</td><td>{}</td><td>{:?}</td><td>{}</td><td>{}</td></tr>\n",
            dep.name,
            dep.kind,
            dep.source.as_str(),
            dep.optional,
            dep.weight,
            dep.features,
            dep.reason.as_deref().unwrap_or("-"),
            dep.is_visible
        ));
    }
    s.push_str("</table>\n");
    s
}

fn render_public_api(report: &CrateReport) -> String {
    let mut s = String::from("<h2>Public API</h2>\n<ul>\n");
    for item in &report.public_api {
        s.push_str(&format!(
            "<li>{:?} {} ({}:{})</li>\n",
            item.kind,
            item.name,
            item.file.display(),
            item.line
        ));
    }
    s.push_str("</ul>\n");
    s
}

fn render_call_graph(report: &CrateReport) -> String {
    let mut s = String::from(
        "<h2>Call Graph</h2>\n<table border=\"1\"><tr><th>Caller</th><th>Callee</th><th>File</th><th>Line</th><th>Confidence</th><th>Runtime Count</th></tr>\n",
    );
    for edge in &report.call_graph {
        s.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{:?}</td><td>{}</td></tr>\n",
            edge.caller,
            edge.callee,
            edge.file.display(),
            edge.line,
            edge.confidence,
            edge.runtime_count
        ));
    }
    s.push_str("</table>\n");
    s
}

fn render_runtime_call_summary(report: &CrateReport) -> String {
    let observed_edges = report
        .call_graph
        .iter()
        .filter(|edge| edge.runtime_count > 0)
        .count();
    format!(
        "<h2>Runtime Call Summary</h2>\n<p>Observed calls: {} | observed edges: {}</p>\n",
        report.runtime_call_observations, observed_edges
    )
}

fn render_security(report: &CrateReport) -> String {
    let mut s = String::from("<h2>Security Findings</h2>\n");
    if report.security_findings.is_empty() {
        s.push_str("<p>No security findings.</p>\n");
    } else {
        s.push_str("<table border=\"1\"><tr><th>ID</th><th>Severity</th><th>Title</th><th>File</th><th>Line</th><th>Confidence</th></tr>\n");
        for f in &report.security_findings {
            s.push_str(&format!(
                "<tr><td>{}</td><td>{:?}</td><td>{}</td><td>{}</td><td>{}</td><td>{:?}</td></tr>\n",
                f.id,
                f.severity,
                f.title,
                f.file
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "-".to_string()),
                f.line.unwrap_or(0),
                f.confidence
            ));
        }
        s.push_str("</table>\n");
    }
    s
}

fn render_tests(report: &CrateReport) -> String {
    let mut s = String::from("<h2>Tests</h2>\n");
    s.push_str(&format!("<p>Total: {}</p>\n", report.test_info.total));
    s.push_str(&format!(
        "<p>Unit: {} | Integration: {} | Doc: {}</p>\n",
        report.test_info.unit_tests, report.test_info.integration_tests, report.test_info.doc_tests
    ));
    s.push_str(&format!(
        "<p>Compile status: {}</p>\n",
        report
            .test_info
            .compile_status
            .as_deref()
            .unwrap_or("unknown")
    ));
    s.push_str(&format!(
        "<p>Functionality coverage: {}/{} ({}%)</p>\n",
        report.test_info.functionality_coverage.covered_items,
        report.test_info.functionality_coverage.public_items,
        report.test_info.functionality_coverage.coverage_percent,
    ));
    s.push_str(&format!(
        "<p>Status: {}</p>\n",
        report
            .test_info
            .last_run_status
            .as_deref()
            .unwrap_or("unknown")
    ));
    s
}

fn render_footprint(report: &CrateReport) -> String {
    let mut s = String::from("<h2>Runtime Footprint</h2>\n");
    if report.footprint.measured {
        s.push_str(&format!(
            "<p>Binary: {} KB</p>\n",
            report.footprint.binary_size.unwrap_or(0) / 1024
        ));
    } else {
        s.push_str("<p>Footprint: not measured</p>\n");
    }
    s
}

fn render_benchmarks(report: &CrateReport) -> String {
    let mut s = String::from("<h2>Benchmarks</h2>\n");
    if report.benchmarks.is_empty() {
        s.push_str("<p>No published benchmark artifacts yet.</p>\n");
    } else {
        for b in &report.benchmarks {
            s.push_str(&format!("<p>{} - {:?}</p>\n", b.name, b.path));
        }
    }
    s
}

fn render_visibility(report: &CrateReport) -> String {
    let mut s = String::from("<h2>Visibility Report</h2>\n");
    s.push_str(&format!(
        "<p>Visible files: {}</p>\n",
        report.visibility.visible_count
    ));
    s.push_str(&format!(
        "<p>Hidden files: {}</p>\n",
        report.visibility.hidden_count
    ));
    s
}

fn render_issue_buttons(report: &CrateReport) -> String {
    let mut s = String::from("<h2>Issue Reporting</h2>\n");
    s.push_str(&format!(
        "<button onclick=\"window.location='{}'\">Report Issue</button>\n",
        crate::codealyzer::issue_report::generate_issue_link(
            &report.identity.name,
            "",
            None,
            None,
            None,
            None,
            None,
            false,
            "https://github.com/edgerun-tech/edgerun"
        )
    ));
    s
}
