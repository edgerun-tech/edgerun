use std::path::PathBuf;

use edgerun_json::{JsonValue, ToJson};

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

impl ToJson for CrateIdentity {
    fn to_json(&self) -> JsonValue {
        let mut map = edgerun_json::Map::new();
        map.insert("name".into(), self.name.to_json());
        map.insert("version".into(), self.version.to_json());
        map.insert("path".into(), self.path.to_string_lossy().into_owned().to_json());
        map.insert("description".into(), self.description.to_json());
        map.insert("license".into(), self.license.to_json());
        map.insert("edition".into(), self.edition.to_json());
        map.insert("rust_version".into(), self.rust_version.to_json());
        map.insert("features".into(), self.features.to_json());
        map.insert("lib_target".into(), self.lib_target.to_json());
        map.insert("bin_targets".into(), self.bin_targets.to_json());
        map.insert("test_targets".into(), self.test_targets.to_json());
        map.insert("bench_targets".into(), self.bench_targets.to_json());
        map.insert("crate_type".into(), self.crate_type.to_json());
        let visible_files: Vec<String> = self
            .visible_files
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect();
        map.insert("visible_files".into(), visible_files.to_json());
        map.insert("hidden_files_count".into(), self.hidden_files_count.to_json());
        map.insert("commit_hash".into(), self.commit_hash.to_json());
        JsonValue::Object(map)
    }
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

impl ToJson for CrateType {
    fn to_json(&self) -> JsonValue {
        self.as_str().to_json()
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

impl ToJson for Dependency {
    fn to_json(&self) -> JsonValue {
        let mut map = edgerun_json::Map::new();
        map.insert("name".into(), self.name.to_json());
        map.insert("version_req".into(), self.version_req.to_json());
        map.insert("kind".into(), self.kind.to_json());
        map.insert("optional".into(), self.optional.to_json());
        map.insert("features".into(), self.features.to_json());
        map.insert("reason".into(), self.reason.to_json());
        map.insert("is_workspace".into(), self.is_workspace.to_json());
        map.insert("is_visible".into(), self.is_visible.to_json());
        map.insert("source".into(), self.source.to_json());
        map.insert("source_ref".into(), self.source_ref.to_json());
        map.insert("weight".into(), self.weight.to_json());
        JsonValue::Object(map)
    }
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

impl ToJson for DependencySource {
    fn to_json(&self) -> JsonValue {
        self.as_str().to_json()
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

impl ToJson for ApiItem {
    fn to_json(&self) -> JsonValue {
        let mut map = edgerun_json::Map::new();
        map.insert("name".into(), self.name.to_json());
        map.insert("kind".into(), self.kind.to_json());
        map.insert("file".into(), self.file.to_string_lossy().into_owned().to_json());
        map.insert("line".into(), self.line.to_json());
        map.insert("docs".into(), self.docs.to_json());
        map.insert("feature_gate".into(), self.feature_gate.to_json());
        map.insert("visibility".into(), self.visibility.to_json());
        JsonValue::Object(map)
    }
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

impl ToJson for ApiItemKind {
    fn to_json(&self) -> JsonValue {
        let s = match self {
            ApiItemKind::Module => "Module",
            ApiItemKind::Struct => "Struct",
            ApiItemKind::Enum => "Enum",
            ApiItemKind::Trait => "Trait",
            ApiItemKind::Function => "Function",
            ApiItemKind::Impl => "Impl",
        };
        JsonValue::String(s.into())
    }
}


#[derive(Debug, Clone, Copy)]
pub enum Visibility {
    Public,
    Private,
    Crate,
}

impl ToJson for Visibility {
    fn to_json(&self) -> JsonValue {
        let s = match self {
            Visibility::Public => "Public",
            Visibility::Private => "Private",
            Visibility::Crate => "Crate",
        };
        JsonValue::String(s.into())
    }
}


#[derive(Debug, Clone)]
pub struct CallGraphEdge {
    pub caller: String,
    pub callee: String,
    pub file: PathBuf,
    pub line: usize,
    pub confidence: Confidence,
}

impl ToJson for CallGraphEdge {
    fn to_json(&self) -> JsonValue {
        let mut map = edgerun_json::Map::new();
        map.insert("caller".into(), self.caller.to_json());
        map.insert("callee".into(), self.callee.to_json());
        map.insert("file".into(), self.file.to_string_lossy().into_owned().to_json());
        map.insert("line".into(), self.line.to_json());
        map.insert("confidence".into(), self.confidence.to_json());
        JsonValue::Object(map)
    }
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

impl ToJson for SecurityFinding {
    fn to_json(&self) -> JsonValue {
        let mut map = edgerun_json::Map::new();
        map.insert("id".into(), self.id.to_json());
        map.insert("severity".into(), self.severity.to_json());
        map.insert("title".into(), self.title.to_json());
        let file = self.file.as_ref().map(|path| path.to_string_lossy().into_owned());
        map.insert("file".into(), file.to_json());
        map.insert("line".into(), self.line.to_json());
        map.insert("code_excerpt".into(), self.code_excerpt.to_json());
        map.insert("explanation".into(), self.explanation.to_json());
        map.insert("confidence".into(), self.confidence.to_json());
        map.insert("recommendation".into(), self.recommendation.to_json());
        JsonValue::Object(map)
    }
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

impl ToJson for TestInfo {
    fn to_json(&self) -> JsonValue {
        let mut map = edgerun_json::Map::new();
        map.insert("total".into(), self.total.to_json());
        map.insert("unit_tests".into(), self.unit_tests.to_json());
        map.insert("integration_tests".into(), self.integration_tests.to_json());
        map.insert("doc_tests".into(), self.doc_tests.to_json());
        map.insert("ignored".into(), self.ignored.to_json());
        map.insert("passing".into(), self.passing.to_json());
        map.insert("failing".into(), self.failing.to_json());
        map.insert("last_run_status".into(), self.last_run_status.to_json());
        map.insert("last_run_commit".into(), self.last_run_commit.to_json());
        map.insert("compile_status".into(), self.compile_status.to_json());
        map.insert("compile_exit_code".into(), self.compile_exit_code.to_json());
        map.insert("compile_warnings".into(), self.compile_warnings.to_json());
        map.insert("functionality_coverage".into(), self.functionality_coverage.to_json());
        JsonValue::Object(map)
    }
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

impl ToJson for FunctionalityCoverage {
    fn to_json(&self) -> JsonValue {
        let mut map = edgerun_json::Map::new();
        map.insert("public_items".into(), self.public_items.to_json());
        map.insert("covered_items".into(), self.covered_items.to_json());
        map.insert("coverage_percent".into(), self.coverage_percent.to_json());
        JsonValue::Object(map)
    }
}



#[derive(Debug, Clone, Copy)]
pub enum Severity {
    Low,
    Medium,
    High,
    Info,
}

impl ToJson for Severity {
    fn to_json(&self) -> JsonValue {
        let value = match self {
            Severity::Low => "Low",
            Severity::Medium => "Medium",
            Severity::High => "High",
            Severity::Info => "Info",
        };
        JsonValue::String(value.into())
    }
}


#[derive(Debug, Clone, Copy)]
pub enum DependencyKind {
    Normal,
    Dev,
    Build,
}

impl ToJson for DependencyKind {
    fn to_json(&self) -> JsonValue {
        match self {
            DependencyKind::Normal => JsonValue::String("Normal".into()),
            DependencyKind::Dev => JsonValue::String("Dev".into()),
            DependencyKind::Build => JsonValue::String("Build".into()),
        }
    }
}


#[derive(Debug, Clone, Copy)]
pub enum Confidence {
    Exact,
    Likely,
    Ambiguous,
}

impl ToJson for Confidence {
    fn to_json(&self) -> JsonValue {
        let s = match self {
            Confidence::Exact => "Exact",
            Confidence::Likely => "Likely",
            Confidence::Ambiguous => "Ambiguous",
        };
        JsonValue::String(s.into())
    }
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

impl ToJson for FootprintInfo {
    fn to_json(&self) -> JsonValue {
        let mut map = edgerun_json::Map::new();
        map.insert("binary_size".into(), self.binary_size.to_json());
        map.insert("stripped_size".into(), self.stripped_size.to_json());
        map.insert("compressed_size".into(), self.compressed_size.to_json());
        map.insert("idle_rss".into(), self.idle_rss.to_json());
        map.insert("peak_rss".into(), self.peak_rss.to_json());
        map.insert("threads".into(), self.threads.to_json());
        map.insert("open_fds".into(), self.open_fds.to_json());
        map.insert("target_triple".into(), self.target_triple.to_json());
        map.insert("measured".into(), self.measured.to_json());
        JsonValue::Object(map)
    }
}


#[derive(Debug, Clone)]
pub struct BenchmarkArtifact {
    pub name: String,
    pub path: PathBuf,
    pub scenario: Option<String>,
    pub commit: Option<String>,
}

impl ToJson for BenchmarkArtifact {
    fn to_json(&self) -> JsonValue {
        let mut map = edgerun_json::Map::new();
        map.insert("name".into(), self.name.to_json());
        map.insert("path".into(), self.path.to_string_lossy().into_owned().to_json());
        map.insert("scenario".into(), self.scenario.to_json());
        map.insert("commit".into(), self.commit.to_json());
        JsonValue::Object(map)
    }
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

impl ToJson for StandardCoverage {
    fn to_json(&self) -> JsonValue {
        let mut map = edgerun_json::Map::new();
        map.insert("standard".into(), self.standard.to_json());
        map.insert("status".into(), self.status.to_json());
        map.insert("code_refs".into(), self.code_refs.to_json());
        map.insert("tests".into(), self.tests.to_json());
        map.insert("confidence".into(), self.confidence.to_json());
        map.insert("notes".into(), self.notes.to_json());
        JsonValue::Object(map)
    }
}


#[derive(Debug, Clone)]
pub struct VisibilityReport {
    pub visible_count: usize,
    pub hidden_count: usize,
    pub public_surface: String,
    pub blocked_refs: Vec<String>,
    pub suspicious_visible: Vec<String>,
}

impl ToJson for VisibilityReport {
    fn to_json(&self) -> JsonValue {
        let mut map = edgerun_json::Map::new();
        map.insert("visible_count".into(), self.visible_count.to_json());
        map.insert("hidden_count".into(), self.hidden_count.to_json());
        map.insert("public_surface".into(), self.public_surface.to_json());
        map.insert("blocked_refs".into(), self.blocked_refs.to_json());
        map.insert("suspicious_visible".into(), self.suspicious_visible.to_json());
        JsonValue::Object(map)
    }
}


#[derive(Debug, Clone)]
pub struct CrateReport {
    pub identity: CrateIdentity,
    pub dependencies: Vec<Dependency>,
    pub public_api: Vec<ApiItem>,
    pub call_graph: Vec<CallGraphEdge>,
    pub security_findings: Vec<SecurityFinding>,
    pub test_info: TestInfo,
    pub footprint: FootprintInfo,
    pub benchmarks: Vec<BenchmarkArtifact>,
    pub standards: Vec<StandardCoverage>,
    pub visibility: VisibilityReport,
    pub generated_at: String,
    pub parser_confidence: String,
}

impl ToJson for CrateReport {
    fn to_json(&self) -> JsonValue {
        let mut map = edgerun_json::Map::new();
        map.insert("identity".into(), self.identity.to_json());
        map.insert("dependencies".into(), self.dependencies.to_json());
        map.insert("public_api".into(), self.public_api.to_json());
        map.insert("call_graph".into(), self.call_graph.to_json());
        map.insert("security_findings".into(), self.security_findings.to_json());
        map.insert("test_info".into(), self.test_info.to_json());
        map.insert("footprint".into(), self.footprint.to_json());
        map.insert("benchmarks".into(), self.benchmarks.to_json());
        map.insert("standards".into(), self.standards.to_json());
        map.insert("visibility".into(), self.visibility.to_json());
        map.insert("generated_at".into(), self.generated_at.to_json());
        map.insert("parser_confidence".into(), self.parser_confidence.to_json());
        JsonValue::Object(map)
    }
}
