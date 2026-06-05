use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CrateIdentity {
    pub name: String,
    pub version: String,
    pub path: PathBuf,
    pub description: Option<String>,
    pub license: Option<String>,
    pub edition: String,
    pub rust_version: Option<String>,
    pub features: Vec<String>,
    pub lib_target: bool,
    pub bin_targets: Vec<String>,
    pub test_targets: Vec<String>,
    pub bench_targets: Vec<String>,
    pub crate_type: CrateType,
    pub visible_files: Vec<PathBuf>,
    pub hidden_files_count: usize,
    pub commit_hash: Option<String>,
}
#[derive(Debug, Clone, Copy)]
pub enum CrateType {
    Unknown,
    Library,
    Binary,
    LibraryAndBinary,
}

impl CrateType {
    pub fn as_str(&self) -> &'static str {
        match self {
            CrateType::Unknown => "unknown",
            CrateType::Library => "library",
            CrateType::Binary => "binary",
            CrateType::LibraryAndBinary => "library+binary",
        }
    }
}
#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub version_req: Option<String>,
    pub kind: DependencyKind,
    pub optional: bool,
    pub features: Vec<String>,
    pub reason: Option<String>,
    pub is_workspace: bool,
    pub is_visible: bool,
    pub source: DependencySource,
    pub source_ref: Option<String>,
    pub weight: usize,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DependencySource {
    Workspace,
    CratesIo,
    Path,
    Git,
    Unknown,
}

impl DependencySource {
    pub fn as_str(&self) -> &'static str {
        match self {
            DependencySource::Workspace => "workspace",
            DependencySource::CratesIo => "crates.io",
            DependencySource::Path => "path",
            DependencySource::Git => "git",
            DependencySource::Unknown => "unknown",
        }
    }
}
#[derive(Debug, Clone)]
pub struct ApiItem {
    pub name: String,
    pub kind: ApiItemKind,
    pub file: PathBuf,
    pub line: usize,
    pub docs: bool,
    pub feature_gate: Option<String>,
    pub visibility: Visibility,
}
#[derive(Debug, Clone, Copy)]
pub enum ApiItemKind {
    Module,
    Struct,
    Enum,
    Trait,
    Function,
    Impl,
}
#[derive(Debug, Clone, Copy)]
pub enum Visibility {
    Public,
    Private,
    Crate,
}
#[derive(Debug, Clone)]
pub struct CallGraphEdge {
    pub caller: String,
    pub callee: String,
    pub file: PathBuf,
    pub line: usize,
    pub confidence: Confidence,
    pub runtime_count: u64,
}
#[derive(Debug, Clone)]
pub struct SecurityFinding {
    pub id: String,
    pub severity: Severity,
    pub title: String,
    pub file: Option<PathBuf>,
    pub line: Option<usize>,
    pub code_excerpt: Option<String>,
    pub explanation: String,
    pub confidence: Confidence,
    pub recommendation: String,
}
#[derive(Debug, Clone)]
pub struct TestInfo {
    pub total: usize,
    pub unit_tests: usize,
    pub integration_tests: usize,
    pub doc_tests: usize,
    pub ignored: usize,
    pub passing: Option<usize>,
    pub failing: Option<usize>,
    pub last_run_status: Option<String>,
    pub last_run_commit: Option<String>,
    pub compile_status: Option<String>,
    pub compile_exit_code: Option<i32>,
    pub compile_warnings: usize,
    pub functionality_coverage: FunctionalityCoverage,
}
#[derive(Debug, Clone)]
pub struct FunctionalityCoverage {
    pub public_items: usize,
    pub covered_items: usize,
    pub coverage_percent: usize,
}

impl FunctionalityCoverage {
    pub fn empty() -> Self {
        Self {
            public_items: 0,
            covered_items: 0,
            coverage_percent: 0,
        }
    }

    pub fn from_counts(public_items: usize, covered_items: usize) -> Self {
        let coverage_percent = if public_items == 0 {
            0
        } else {
            covered_items.saturating_mul(100) / public_items
        };

        Self {
            public_items,
            covered_items,
            coverage_percent,
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub enum Severity {
    Low,
    Medium,
    High,
    Info,
}
#[derive(Debug, Clone, Copy)]
pub enum DependencyKind {
    Normal,
    Dev,
    Build,
}
#[derive(Debug, Clone, Copy)]
pub enum Confidence {
    Exact,
    Likely,
    Ambiguous,
}
#[derive(Debug, Clone)]
pub struct FootprintInfo {
    pub binary_size: Option<usize>,
    pub stripped_size: Option<usize>,
    pub compressed_size: Option<usize>,
    pub idle_rss: Option<usize>,
    pub peak_rss: Option<usize>,
    pub threads: Option<usize>,
    pub open_fds: Option<usize>,
    pub target_triple: Option<String>,
    pub measured: bool,
}
#[derive(Debug, Clone)]
pub struct BenchmarkArtifact {
    pub name: String,
    pub path: PathBuf,
    pub scenario: Option<String>,
    pub commit: Option<String>,
}
#[derive(Debug, Clone)]
pub struct StandardCoverage {
    pub standard: String,
    pub status: String,
    pub code_refs: Vec<String>,
    pub tests: Vec<String>,
    pub confidence: Confidence,
    pub notes: Option<String>,
}
#[derive(Debug, Clone)]
pub struct VisibilityReport {
    pub visible_count: usize,
    pub hidden_count: usize,
    pub public_surface: String,
    pub blocked_refs: Vec<String>,
    pub suspicious_visible: Vec<String>,
}
#[derive(Debug, Clone)]
pub struct CrateReport {
    pub identity: CrateIdentity,
    pub dependencies: Vec<Dependency>,
    pub public_api: Vec<ApiItem>,
    pub call_graph: Vec<CallGraphEdge>,
    pub runtime_call_observations: u64,
    pub security_findings: Vec<SecurityFinding>,
    pub test_info: TestInfo,
    pub footprint: FootprintInfo,
    pub benchmarks: Vec<BenchmarkArtifact>,
    pub standards: Vec<StandardCoverage>,
    pub visibility: VisibilityReport,
    pub generated_at: String,
    pub parser_confidence: String,
}

impl CrateReport {
    pub fn to_json_string(&self) -> String {
        json_object(&[
            json_field("identity", &self.identity.to_json_string()),
            json_field(
                "dependencies",
                &json_array(&self.dependencies, Dependency::to_json_string),
            ),
            json_field(
                "public_api",
                &json_array(&self.public_api, ApiItem::to_json_string),
            ),
            json_field(
                "call_graph",
                &json_array(&self.call_graph, CallGraphEdge::to_json_string),
            ),
            json_field(
                "runtime_call_observations",
                &self.runtime_call_observations.to_string(),
            ),
            json_field(
                "security_findings",
                &json_array(&self.security_findings, SecurityFinding::to_json_string),
            ),
            json_field("test_info", &self.test_info.to_json_string()),
            json_field("footprint", &self.footprint.to_json_string()),
            json_field(
                "benchmarks",
                &json_array(&self.benchmarks, BenchmarkArtifact::to_json_string),
            ),
            json_field(
                "standards",
                &json_array(&self.standards, StandardCoverage::to_json_string),
            ),
            json_field("visibility", &self.visibility.to_json_string()),
            json_field("generated_at", &json_string(&self.generated_at)),
            json_field("parser_confidence", &json_string(&self.parser_confidence)),
        ])
    }
}

impl CrateIdentity {
    pub fn to_json_string(&self) -> String {
        let visible_files = self
            .visible_files
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        json_object(&[
            json_field("name", &json_string(&self.name)),
            json_field("version", &json_string(&self.version)),
            json_field("path", &json_string(&self.path.to_string_lossy())),
            json_field(
                "description",
                &json_option_string(self.description.as_deref()),
            ),
            json_field("license", &json_option_string(self.license.as_deref())),
            json_field("edition", &json_string(&self.edition)),
            json_field(
                "rust_version",
                &json_option_string(self.rust_version.as_deref()),
            ),
            json_field("features", &json_string_array(&self.features)),
            json_field("lib_target", &json_bool(self.lib_target)),
            json_field("bin_targets", &json_string_array(&self.bin_targets)),
            json_field("test_targets", &json_string_array(&self.test_targets)),
            json_field("bench_targets", &json_string_array(&self.bench_targets)),
            json_field("crate_type", &json_string(self.crate_type.as_str())),
            json_field("visible_files", &json_string_array(&visible_files)),
            json_field("hidden_files_count", &self.hidden_files_count.to_string()),
            json_field(
                "commit_hash",
                &json_option_string(self.commit_hash.as_deref()),
            ),
        ])
    }
}

impl Dependency {
    pub fn to_json_string(&self) -> String {
        json_object(&[
            json_field("name", &json_string(&self.name)),
            json_field(
                "version_req",
                &json_option_string(self.version_req.as_deref()),
            ),
            json_field("kind", &json_string(self.kind.as_str())),
            json_field("optional", &json_bool(self.optional)),
            json_field("features", &json_string_array(&self.features)),
            json_field("reason", &json_option_string(self.reason.as_deref())),
            json_field("is_workspace", &json_bool(self.is_workspace)),
            json_field("is_visible", &json_bool(self.is_visible)),
            json_field("source", &json_string(self.source.as_str())),
            json_field(
                "source_ref",
                &json_option_string(self.source_ref.as_deref()),
            ),
            json_field("weight", &self.weight.to_string()),
        ])
    }
}

impl ApiItem {
    pub fn to_json_string(&self) -> String {
        json_object(&[
            json_field("name", &json_string(&self.name)),
            json_field("kind", &json_string(self.kind.as_str())),
            json_field("file", &json_string(&self.file.to_string_lossy())),
            json_field("line", &self.line.to_string()),
            json_field("docs", &json_bool(self.docs)),
            json_field(
                "feature_gate",
                &json_option_string(self.feature_gate.as_deref()),
            ),
            json_field("visibility", &json_string(self.visibility.as_str())),
        ])
    }
}

impl CallGraphEdge {
    pub fn to_json_string(&self) -> String {
        json_object(&[
            json_field("caller", &json_string(&self.caller)),
            json_field("callee", &json_string(&self.callee)),
            json_field("file", &json_string(&self.file.to_string_lossy())),
            json_field("line", &self.line.to_string()),
            json_field("confidence", &json_string(self.confidence.as_str())),
            json_field("runtime_count", &self.runtime_count.to_string()),
        ])
    }
}

impl SecurityFinding {
    pub fn to_json_string(&self) -> String {
        let file = self.file.as_ref().map(|path| path.to_string_lossy());
        json_object(&[
            json_field("id", &json_string(&self.id)),
            json_field("severity", &json_string(self.severity.as_str())),
            json_field("title", &json_string(&self.title)),
            json_field("file", &json_option_cow(file.as_ref())),
            json_field("line", &json_option_usize(self.line)),
            json_field(
                "code_excerpt",
                &json_option_string(self.code_excerpt.as_deref()),
            ),
            json_field("explanation", &json_string(&self.explanation)),
            json_field("confidence", &json_string(self.confidence.as_str())),
            json_field("recommendation", &json_string(&self.recommendation)),
        ])
    }
}

impl TestInfo {
    pub fn to_json_string(&self) -> String {
        json_object(&[
            json_field("total", &self.total.to_string()),
            json_field("unit_tests", &self.unit_tests.to_string()),
            json_field("integration_tests", &self.integration_tests.to_string()),
            json_field("doc_tests", &self.doc_tests.to_string()),
            json_field("ignored", &self.ignored.to_string()),
            json_field("passing", &json_option_usize(self.passing)),
            json_field("failing", &json_option_usize(self.failing)),
            json_field(
                "last_run_status",
                &json_option_string(self.last_run_status.as_deref()),
            ),
            json_field(
                "last_run_commit",
                &json_option_string(self.last_run_commit.as_deref()),
            ),
            json_field(
                "compile_status",
                &json_option_string(self.compile_status.as_deref()),
            ),
            json_field(
                "compile_exit_code",
                &json_option_i32(self.compile_exit_code),
            ),
            json_field("compile_warnings", &self.compile_warnings.to_string()),
            json_field(
                "functionality_coverage",
                &self.functionality_coverage.to_json_string(),
            ),
        ])
    }
}

impl FunctionalityCoverage {
    pub fn to_json_string(&self) -> String {
        json_object(&[
            json_field("public_items", &self.public_items.to_string()),
            json_field("covered_items", &self.covered_items.to_string()),
            json_field("coverage_percent", &self.coverage_percent.to_string()),
        ])
    }
}

impl FootprintInfo {
    pub fn to_json_string(&self) -> String {
        json_object(&[
            json_field("binary_size", &json_option_usize(self.binary_size)),
            json_field("stripped_size", &json_option_usize(self.stripped_size)),
            json_field("compressed_size", &json_option_usize(self.compressed_size)),
            json_field("idle_rss", &json_option_usize(self.idle_rss)),
            json_field("peak_rss", &json_option_usize(self.peak_rss)),
            json_field("threads", &json_option_usize(self.threads)),
            json_field("open_fds", &json_option_usize(self.open_fds)),
            json_field(
                "target_triple",
                &json_option_string(self.target_triple.as_deref()),
            ),
            json_field("measured", &json_bool(self.measured)),
        ])
    }
}

impl BenchmarkArtifact {
    pub fn to_json_string(&self) -> String {
        json_object(&[
            json_field("name", &json_string(&self.name)),
            json_field("path", &json_string(&self.path.to_string_lossy())),
            json_field("scenario", &json_option_string(self.scenario.as_deref())),
            json_field("commit", &json_option_string(self.commit.as_deref())),
        ])
    }
}

impl StandardCoverage {
    pub fn to_json_string(&self) -> String {
        json_object(&[
            json_field("standard", &json_string(&self.standard)),
            json_field("status", &json_string(&self.status)),
            json_field("code_refs", &json_string_array(&self.code_refs)),
            json_field("tests", &json_string_array(&self.tests)),
            json_field("confidence", &json_string(self.confidence.as_str())),
            json_field("notes", &json_option_string(self.notes.as_deref())),
        ])
    }
}

impl VisibilityReport {
    pub fn to_json_string(&self) -> String {
        json_object(&[
            json_field("visible_count", &self.visible_count.to_string()),
            json_field("hidden_count", &self.hidden_count.to_string()),
            json_field("public_surface", &json_string(&self.public_surface)),
            json_field("blocked_refs", &json_string_array(&self.blocked_refs)),
            json_field(
                "suspicious_visible",
                &json_string_array(&self.suspicious_visible),
            ),
        ])
    }
}

impl ApiItemKind {
    fn as_str(&self) -> &'static str {
        match self {
            ApiItemKind::Module => "Module",
            ApiItemKind::Struct => "Struct",
            ApiItemKind::Enum => "Enum",
            ApiItemKind::Trait => "Trait",
            ApiItemKind::Function => "Function",
            ApiItemKind::Impl => "Impl",
        }
    }
}

impl Visibility {
    fn as_str(&self) -> &'static str {
        match self {
            Visibility::Public => "Public",
            Visibility::Private => "Private",
            Visibility::Crate => "Crate",
        }
    }
}

impl Severity {
    fn as_str(&self) -> &'static str {
        match self {
            Severity::Low => "Low",
            Severity::Medium => "Medium",
            Severity::High => "High",
            Severity::Info => "Info",
        }
    }
}

impl DependencyKind {
    fn as_str(&self) -> &'static str {
        match self {
            DependencyKind::Normal => "Normal",
            DependencyKind::Dev => "Dev",
            DependencyKind::Build => "Build",
        }
    }
}

impl Confidence {
    fn as_str(&self) -> &'static str {
        match self {
            Confidence::Exact => "Exact",
            Confidence::Likely => "Likely",
            Confidence::Ambiguous => "Ambiguous",
        }
    }
}

fn json_object(fields: &[String]) -> String {
    format!("{{{}}}", fields.join(","))
}

fn json_field(name: &str, encoded: &str) -> String {
    format!("{}:{}", json_string(name), encoded)
}

fn json_array<T>(items: &[T], emit: fn(&T) -> String) -> String {
    let values = items.iter().map(emit).collect::<Vec<_>>();
    format!("[{}]", values.join(","))
}

fn json_string_array(items: &[String]) -> String {
    let values = items
        .iter()
        .map(|item| json_string(item))
        .collect::<Vec<_>>();
    format!("[{}]", values.join(","))
}

fn json_option_string(value: Option<&str>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_string())
}

fn json_option_cow(value: Option<&std::borrow::Cow<'_, str>>) -> String {
    value
        .map(|value| json_string(value.as_ref()))
        .unwrap_or_else(|| "null".to_string())
}

fn json_option_usize(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn json_option_i32(value: Option<i32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn json_bool(value: bool) -> String {
    if value {
        "true".to_string()
    } else {
        "false".to_string()
    }
}

fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}
